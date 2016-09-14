// Copyright 2015, 2016 Ethcore (UK) Ltd.
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

//! A provider for the LES protocol. This is typically a full node, who can
//! give as much data as necessary to its peers.

pub use proof_request::{CHTProofRequest, ProofRequest};

use util::Bytes;
use util::hash::H256;

/// Defines the operations that a provider for `LES` must fulfill.
///
/// These are defined at [1], but may be subject to change.
///
/// [1]: https://github.com/ethcore/parity/wiki/Light-Ethereum-Subprotocol-(LES)
pub trait Provider {
	/// Provide a list of headers starting at the requested block,
	/// possibly in reverse and skipping `skip` at a time.
	///
	/// The returned vector may have any length in the range [0, `max`], but the
	/// results within must adhere to the `skip` and `reverse` parameters.
	fn block_headers(&self, block: H256, skip: usize, max: usize, reverse: bool) -> Vec<Bytes>;

	/// Provide as many as possible of the requested blocks (minus the headers) encoded
	/// in RLP format.
	fn block_bodies(&self, blocks: Vec<H256>) -> Vec<Bytes>;

	/// Provide the receipts as many as possible of the requested blocks.
	/// Returns a vector of RLP-encoded lists of receipts.
	fn receipts(&self, blocks: Vec<H256>) -> Vec<Bytes>;

	/// Provide a set of merkle proofs, as requested. Each request is a
	/// block hash and request parameters.
	///
	/// Returns a vector to RLP-encoded lists satisfying the requests.
	fn proofs(&self, requests: Vec<(H256, ProofRequest)>) -> Vec<Bytes>;

	/// Provide contract code for the specified (block_hash, account_hash) pairs.
	fn code(&self, accounts: Vec<(H256, H256)>) -> Vec<Bytes>;

	/// Provide header proofs from the Canonical Hash Tries.
	fn header_proofs(&self, requests: Vec<CHTProofRequest>) -> Vec<Bytes>;
}