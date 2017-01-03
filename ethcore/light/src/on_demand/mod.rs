// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! On-demand chain requests over LES. This is a major building block for RPCs.
//! The request service is implemented using Futures. Higher level request handlers
//! will take the raw data received here and extract meaningful results from it.

use std::collections::HashMap;

use ethcore::encoded;
use ethcore::receipt::Receipt;

use futures::{Async, Poll, Future};
use futures::sync::oneshot;
use network::PeerId;

use net::{Handler, Status, Capabilities, Announcement, EventContext, BasicContext, ReqId};
use util::{Bytes, H256, U256, RwLock};
use types::les_request::{self as les_request, Request as LesRequest};

pub mod request;

/// Basic account data.
// TODO: [rob] unify with similar struct in `snapshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
	/// Balance in Wei
	pub balance: U256,
	/// Storage trie root.
	pub storage_root: H256,
	/// Code hash
	pub code_hash: H256,
	/// Nonce
	pub nonce: U256,
}

/// Errors which can occur while trying to fulfill a request.
#[derive(Debug, Clone, Copy)]
pub enum Error {
	/// Request was canceled.
	Canceled,
	/// No suitable peers available to serve the request.
	NoPeersAvailable,
	/// Request timed out.
	TimedOut,
}

impl From<oneshot::Canceled> for Error {
	fn from(_: oneshot::Canceled) -> Self {
		Error::Canceled
	}
}

/// Future which awaits a response to an on-demand request.
pub struct Response<T>(oneshot::Receiver<Result<T, Error>>);

impl<T> Future for Response<T> {
	type Item = T;
	type Error = Error;

	fn poll(&mut self) -> Poll<T, Error> {
		match self.0.poll().map_err(Into::into) {
			Ok(Async::Ready(val)) => val.map(Async::Ready),
			Ok(Async::NotReady) => Ok(Async::NotReady),
			Err(e) => Err(e),
		}
	}
}

type Sender<T> = oneshot::Sender<Result<T, Error>>;

// relevant peer info.
struct Peer {
	status: Status,
	capabilities: Capabilities,
}

// Attempted request info and sender to put received value.
enum Pending {
	HeaderByNumber(request::HeaderByNumber, Sender<encoded::Header>), // num + CHT root
	HeaderByHash(request::HeaderByHash, Sender<encoded::Header>),
	Block(request::Body, Sender<encoded::Block>),
	BlockReceipts(request::BlockReceipts, Sender<Vec<Receipt>>),
	Account(request::Account, Sender<Account>),
}

/// On demand request service. See module docs for more details.
/// Accumulates info about all peers' capabilities and dispatches
/// requests to them accordingly.
pub struct OnDemand {
	peers: RwLock<HashMap<PeerId, Peer>>,
	pending_requests: RwLock<HashMap<ReqId, Pending>>,
}

impl OnDemand {
	/// Request a header by block number and CHT root hash.
	pub fn header_by_number(&self, ctx: &BasicContext, req: request::HeaderByNumber) -> Response<encoded::Header> {
		let (sender, receiver) = oneshot::channel();
		let num = req.num;
		let cht_num = ::client::cht::block_to_cht_number(req.num);
		let les_req = LesRequest::HeaderProofs(les_request::HeaderProofs {
			requests: vec![les_request::HeaderProof {
				cht_number: cht_num,
				block_number: req.num,
				from_level: 0,
			}],
		});

		// we're looking for a peer with serveHeaders who's far enough along in the
		// chain.
		for (id, peer) in self.peers.read().iter() {
			if peer.capabilities.serve_headers && peer.status.head_num >= num {
				match ctx.request_from(*id, les_req.clone()) {
					Ok(req_id) => {
						trace!(target: "on_demand", "Assigning request to peer {}", id);
						self.pending_requests.write().insert(
							req_id,
							Pending::HeaderByNumber(req, sender)
						);
						return Response(receiver);
					},
					Err(e) =>
						trace!(target: "on_demand", "Failed to make request of peer {}: {:?}", id, e),
				}
			}
		}

		// TODO: retrying
		trace!(target: "on_demand", "No suitable peer for request");
		sender.complete(Err(Error::NoPeersAvailable));
		Response(receiver)
	}

	/// Request a header by hash. This is less accurate than by-number because we don't know
	/// where in the chain this header lies, and therefore can't find a peer who is supposed to have
	/// it as easily.
	pub fn header_by_hash(&self, ctx: &BasicContext, req: request::HeaderByHash) -> Response<encoded::Header> {
		let (sender, receiver) = oneshot::channel();
		let les_req = LesRequest::Headers(les_request::Headers {
			start: req.0.into(),
			max: 1,
			skip: 0,
			reverse: false,
		});

		// all we've got is a hash, so we'll just guess at peers who might have
		// it randomly.
		let mut potential_peers = self.peers.read().iter()
			.filter(|&(_, peer)| peer.capabilities.serve_headers)
			.map(|(id, _)| *id)
			.collect::<Vec<_>>();

		let mut rng = ::rand::thread_rng();

		::rand::Rng::shuffle(&mut rng, &mut potential_peers);

		for id in potential_peers {
			match ctx.request_from(id, les_req.clone()) {
				Ok(req_id) => {
					trace!(target: "on_demand", "Assigning request to peer {}", id);
					self.pending_requests.write().insert(
						req_id,
						Pending::HeaderByHash(req, sender),
					);
					return Response(receiver);
				}
				Err(e) =>
					trace!(target: "on_demand", "Failed to make request of peer {}: {:?}", id, e),
			}
		}

		// TODO: retrying
		trace!(target: "on_demand", "No suitable peer for request");
		sender.complete(Err(Error::NoPeersAvailable));
		Response(receiver)
	}

	/// Request a block, given its header. Block bodies are requestable by hash only,
	/// and the header is required anyway to verify and complete the block body
	/// -- this just doesn't obscure the network query.
	pub fn block(&self, ctx: &BasicContext, req: request::Body) -> Response<encoded::Block> {
		let (sender, receiver) = oneshot::channel();
		let num = req.header.number();
		let les_req = LesRequest::Bodies(les_request::Bodies {
			block_hashes: vec![req.hash],
		});

		// we're looking for a peer with serveChainSince(num)
		for (id, peer) in self.peers.read().iter() {
			if peer.capabilities.serve_chain_since.as_ref().map_or(false, |x| *x >= num) {
				match ctx.request_from(*id, les_req.clone()) {
					Ok(req_id) => {
						trace!(target: "on_demand", "Assigning request to peer {}", id);
						self.pending_requests.write().insert(
							req_id,
							Pending::Block(req, sender)
						);
						return Response(receiver)
					}
					Err(e) =>
						trace!(target: "on_demand", "Failed to make request of peer {}: {:?}", id, e),
				}
			}
		}

		// TODO: retrying
		trace!(target: "on_demand", "No suitable peer for request");
		sender.complete(Err(Error::NoPeersAvailable));
		Response(receiver)
	}

	/// Request the receipts for a block. The header serves two purposes:
	/// provide the block hash to fetch receipts for, and for verification of the receipts root.
	pub fn block_receipts(&self, ctx: &BasicContext, req: request::BlockReceipts) -> Response<Vec<Receipt>> {
		let (sender, receiver) = oneshot::channel();
		let num = req.0.number();
		let les_req = LesRequest::Receipts(les_request::Receipts {
			block_hashes: vec![req.0.hash()],
		});

		// we're looking for a peer with serveChainSince(num)
		for (id, peer) in self.peers.read().iter() {
			if peer.capabilities.serve_chain_since.as_ref().map_or(false, |x| *x >= num) {
				match ctx.request_from(*id, les_req.clone()) {
					Ok(req_id) => {
						trace!(target: "on_demand", "Assigning request to peer {}", id);
						self.pending_requests.write().insert(
							req_id,
							Pending::BlockReceipts(req, sender)
						);
						return Response(receiver)
					}
					Err(e) =>
						trace!(target: "on_demand", "Failed to make request of peer {}: {:?}", id, e),
				}
			}
		}

		// TODO: retrying
		trace!(target: "on_demand", "No suitable peer for request");
		sender.complete(Err(Error::NoPeersAvailable));
		Response(receiver)
	}

	/// Request an account by address and block header -- which gives a hash to query and a state root
	/// to verify against.
	pub fn account(&self, ctx: &BasicContext, req: request::Account) -> Response<Account> {
		let (sender, receiver) = oneshot::channel();
		let num = req.header.number();
		let les_req = LesRequest::StateProofs(les_request::StateProofs {
			requests: vec![les_request::StateProof {
				block: req.header.hash(),
				key1: ::util::Hashable::sha3(&req.address),
				key2: None,
				from_level: 0,
			}],
		});

		// we're looking for a peer with serveStateSince(num)
		for (id, peer) in self.peers.read().iter() {
			if peer.capabilities.serve_state_since.as_ref().map_or(false, |x| *x >= num) {
				match ctx.request_from(*id, les_req.clone()) {
					Ok(req_id) => {
						trace!(target: "on_demand", "Assigning request to peer {}", id);
						self.pending_requests.write().insert(
							req_id,
							Pending::Account(req, sender)
						);
						return Response(receiver)
					}
					Err(e) =>
						trace!(target: "on_demand", "Failed to make request of peer {}: {:?}", id, e),
				}
			}
		}

		// TODO: retrying
		trace!(target: "on_demand", "No suitable peer for request");
		sender.complete(Err(Error::NoPeersAvailable));
		Response(receiver)
	}
}

impl Handler for OnDemand {
	fn on_connect(&self, ctx: &EventContext, status: &Status, capabilities: &Capabilities) {
		self.peers.write().insert(ctx.peer(), Peer { status: status.clone(), capabilities: capabilities.clone() });
	}

	fn on_disconnect(&self, ctx: &EventContext, unfulfilled: &[ReqId]) {
		self.peers.write().remove(&ctx.peer());

		for unfulfilled in unfulfilled {
			if let Some(pending) = self.pending_requests.write().remove(unfulfilled) {
				trace!(target: "on_demand", "Attempting to reassign dropped request");
				unimplemented!()
			}
		}
	}

	fn on_announcement(&self, ctx: &EventContext, announcement: &Announcement) {
		let mut peers = self.peers.write();
		if let Some(ref mut peer) = peers.get_mut(&ctx.peer()) {
			peer.status.update_from(&announcement);
			peer.capabilities.update_from(&announcement);
		}
	}

	fn on_header_proofs(&self, ctx: &EventContext, req_id: ReqId, proofs: &[(Bytes, Vec<Bytes>)]) {
		let peer = ctx.peer();
		let req = match self.pending_requests.write().remove(&req_id) {
			Some(req) => req,
			None => return,
		};

		match req {
			Pending::HeaderByNumber(req, sender) => {
				let &&(ref header, ref proof) = match proofs.get(0) {
					Some(ref x) => x,
					None => {
						ctx.disconnect_peer(peer);
						return
					}
				};
			}
			_ => panic!("Only header by number request fetches header proofs; qed"),
		}
	}
}
