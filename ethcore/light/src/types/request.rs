// Copyright 2015-2017 Parity Technologies (UK) Ltd.
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

//! Light protocol request types.

use std::collections::HashMap;

use ethcore::transaction::Action;
use rlp::{Encodable, Decodable, Decoder, DecoderError, RlpStream, Stream};
use util::{Address, H256, U256, Uint};

// re-exports of request types.
pub use self::header::{
	Complete as CompleteHeadersRequest,
	Incomplete as IncompleteHeadersRequest,
	Response as HeadersResponse
};
pub use self::header_proof::{
	Complete as CompleteHeaderProofRequest,
	Incomplete as IncompleteHeaderProofRequest,
	Response as HeaderProofResponse
};
pub use self::block_body::{
	Complete as CompleteBodyRequest,
	Incomplete as IncompleteBodyRequest,
	Response as BodyResponse
};
pub use self::receipts::{
	Complete as CompleteReceiptsRequest,
	Incomplete as IncompleteReceiptsRequest
	Response as ReceiptsResponse
};
pub use self::account::{
	Complete as CompleteAccountRequest,
	Incomplete as IncompleteAccountRequest,
	Response as AccountResponse,
};
pub use self::storage::{
	Complete as CompleteStorageRequest,
	Incomplete as IncompleteStorageRequest,
	Response as StorageResponse
};
pub use self::contract_code::{
	Complete as CompleteCodeRequest,
	Incomplete as IncompleteCodeRequest,
	Response as CodeResponse,
};

/// Error indicating a reference to a non-existent or wrongly-typed output.
pub struct NoSuchOutput;

/// An input to a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field<T> {
	/// A pre-specified input.
	Scalar(T),
	/// An input which can be resolved later on.
	/// (Request index, output index)
	BackReference(usize, usize),
}

impl From<T> for Field<T> {
	fn from(val: T) -> Self {
		Field::Scalar(val)
	}
}

impl<T: Decodable> Decodable for Field<T> {
	fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
		let rlp = decoder.as_rlp();

		match rlp.val_at::<u8>(0)? {
			0 => Ok(Field::Scalar(rlp.val_at::<T>(1)?)),
			1 => Ok({
				let inner_rlp = rlp.at(1)?;
				Field::BackReference(inner_rlp.val_at(0)?, inner_rlp.val_at(1)?)
			})
			_ => Err(DecoderError::Custom("Unknown discriminant for PIP field.")),
		}
	}
}

impl<T: Encodable> Encodable for Field<T> {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.begin_list(2);
		match *self {
			Field::Scalar(ref data) => s.append(&0u8).append(data),
			Field::BackReference(ref req, ref idx) =>
				s.append(&1u8).begin_list(2).append(req).append(idx),
		};
	}
}

/// Request outputs which can be reused as inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
	/// A 32-byte hash output.
	Hash(H256),
	/// An unsigned-integer output.
	Number(u64),
}

impl Output {
	fn kind(&self) -> OutputKind {
		match *self {
			Output::Hash(_) => OutputKind::Hash,
			Output::Number(_) => OutputKind::Number,
		}
	}
}

/// Response output kinds which can be used as back-references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputKind {
	/// A 32-byte hash output.
	Hash,
	/// An unsigned-integer output.
	Number,
}

/// Either a hash or a number.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ipc", binary)]
pub enum HashOrNumber {
	/// Block hash variant.
	Hash(H256),
	/// Block number variant.
	Number(u64),
}

impl From<H256> for HashOrNumber {
	fn from(hash: H256) -> Self {
		HashOrNumber::Hash(hash)
	}
}

impl From<u64> for HashOrNumber {
	fn from(num: u64) -> Self {
		HashOrNumber::Number(num)
	}
}

/// All request types, as they're sent over the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
	/// A request for block headers.
	Headers(IncompleteHeadersRequest),
	/// A request for a header proof (from a CHT)
	HeaderProof(IncompleteHeaderProofRequest),
	// TransactionIndex,
	/// A request for a block's receipts.
	Receipts(IncompleteReceiptsRequest),
	/// A request for a block body.
	Body(IncompleteBodyRequest),
	/// A request for a merkle proof of an account.
	Account(IncompleteAccountRequest),
	/// A request for a merkle proof of contract storage.
	Storage(IncompleteStorageRequest),
	/// A request for contract code.
	Code(IncompleteCodeRequest),
	// Transaction proof.
}

/// All request types, as they're sent over the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompleteRequest {
	/// A request for block headers.
	Headers(CompleteHeadersRequest),
	/// A request for a header proof (from a CHT)
	HeaderProof(CompleteHeaderProofRequest),
	// TransactionIndex,
	/// A request for a block's receipts.
	Receipts(CompleteReceiptsRequest),
	/// A request for a block body.
	Body(CompleteBodyRequest),
	/// A request for a merkle proof of an account.
	Account(CompleteAccountRequest),
	/// A request for a merkle proof of contract storage.
	Storage(CompleteStorageRequest),
	/// A request for contract code.
	Code(CompleteCodeRequest),
	// Transaction proof.
}

impl Request {
	fn kind(&self) -> RequestKind {
		match *self {
			Request::Headers(_) => RequestKind::Headers,
			Request::HeaderProof(_) => RequestKind::HeaderProof,
			Request::Receipts(_) => RequestKind::Receipts,
			Request::Body(_) => RequestKind::Body,
			Request::Account(_) => RequestKind::Account,
			Request::Storage(_) => RequestKind::Storage,
			Request::Code(_) => RequestKind::Code,
		}
	}
}

impl Decodable for Request {
	fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
		let rlp = decoder.as_rlp();

		match rlp.val_at::<RequestKind>(0)? {
			RequestKind::Headers => Ok(Request::Headers(rlp.val_at(1)?)),
			RequestKind::HeaderProof => Ok(Request::HeaderProof(rlp.val_at(1)?)),
			RequestKind::Receipts => Ok(Request::Receipts(rlp.val_at(1)?)),
			RequestKind::Body => Ok(Request::Body(rlp.val_at(1)?)),
			RequestKind::Account => Ok(Request::Account(rlp.val_at(1)?)),
			RequestKind::Storage => Ok(Request::Storage(rlp.val_at(1)?)),
			RequestKind::Code => Ok(Request::Code(rlp.val_at(1)?)),
		}
	}
}

impl Encodable for Request {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.begin_list(2).append(&self.kind());

		match *self {
			Request::Headers(ref req) => s.append(req),
			Request::HeaderProof(ref req) => s.append(req),
			Request::Receipts(ref req) => s.append(req),
			Request::Body(ref req) => s.append(req),
			Request::Account(ref req) => s.append(req),
			Request::Storage(ref req) => s.append(req),
			Request::Code(ref req) => s.append(req),
		};
	}
}

impl IncompleteRequest for Request {
	type Complete = CompleteRequest;

	fn check_outputs<F>(&self, f: F) -> Result<(), NoSuchOutput>
		where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>
	{
		match *self {
			Request::Headers(ref req) => req.check_outputs(f),
			Request::HeaderProof(ref req) => req.check_outputs(f),
			Request::Receipts(ref req) => req.check_outputs(f),
			Request::Body(ref req) => req.check_outputs(f),
			Request::Account(ref req) => req.check_outputs(f),
			Request::Storage(ref req) => req.check_outputs(f),
			Request::Code(ref req) => req.check_outputs(f),
		}
	}

	fn note_outputs<F>(&self, f: F) where F: FnMut(usize, OutputKind) {
		match *self {
			Request::Headers(ref req) => req.note_outputs(f),
			Request::HeaderProof(ref req) => req.note_outputs(f),
			Request::Receipts(ref req) => req.note_outputs(f),
			Request::Body(ref req) => req.note_outputs(f),
			Request::Account(ref req) => req.note_outputs(f),
			Request::Storage(ref req) => req.note_outputs(f),
			Request::Code(ref req) => req.note_outputs(f),
		}
	}

	fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
		where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>
	{
		match self {
			Request::Headers(req) => CompleteRequest::Headers(req.fill(oracle)),
			Request::HeaderProof(req) => CompleteRequest::HeaderProof(req.fill(oracle)),
			Request::Receipts(req) => CompleteRequest::Receipts(req.fill(oracle)),
			Request::Body(req) => CompleteRequest::Body(req.fill(oracle)),
			Request::Account(req) => CompleteRequest::Account(req.fill(oracle)),
			Request::Storage(req) => CompleteRequest::Storage(req.fill(oracle)),
			Request::Code(req) => CompleteRequest::Code(req.fill(oracle)),
		}
	}
}

/// Kinds of requests.
/// Doubles as the "ID" field of the request.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestKind {
	/// A request for headers.
	Headers = 0,
	HeaderProof = 1,
	// TransactionIndex = 2,
	Receipts = 3,
	Body = 4,
	Account = 5,
	Storage = 6,
	Code = 7,
	// TransactionProof = 8,
}

impl Decodable for RequestKind {
	fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
		let rlp = decoder.as_rlp();

		match rlp.as_val::<u8>()? {
			0 => Ok(RequestKind::Headers),
			1 => Ok(RequestKind::HeaderProof),
			// 2 => Ok(RequestKind::TransactionIndex,
			3 => Ok(RequestKind::Receipts),
			4 => Ok(RequestKind::Body),
			5 => Ok(RequestKind::Account),
			6 => Ok(RequestKind::Storage),
			7 => Ok(RequestKind::Code),
			// 8 => Ok(RequestKind::TransactionProof),
			_ => Err(DecoderError::Custom("Unknown PIP request ID.")),
		}
	}
}

impl Encodable for RequestKind {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.append(self as &u8);
	}
}

/// All response types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
	/// A response for block headers.
	Headers(HeadersResponse),
	/// A response for a header proof (from a CHT)
	HeaderProof(HeaderProofResponse),
	// TransactionIndex,
	/// A response for a block's receipts.
	Receipts(ReceiptsResponse),
	/// A response for a block body.
	Body(BodyResponse),
	/// A response for a merkle proof of an account.
	Account(AccountResponse),
	/// A response for a merkle proof of contract storage.
	Storage(StorageResponse),
	/// A response for contract code.
	Code(CodeResponse),
	// Transaction proof.
}

impl Response {
	/// Fill reusable outputs by writing them into the function.
	pub fn fill_outputs<F>(&self, mut f: F) where F: FnMut(usize, Output) {
		match *self {
			Response::Headers(res) => res.fill_outputs(f)
			Response::HeaderProof(res) => res.fill_outputs(f)
			Response::Receipts(res) => res.fill_outputs(f)
			Response::Body(res) => res.fill_outputs(f)
			Response::Account(res) => res.fill_outputs(f)
			Response::Storage(res) => res.fill_outputs(f)
			Response::Code(res) => res.fill_outputs(f)
		}
	}
}

/// A potentially incomplete request.
pub trait IncompleteRequest: Sized {
	type Complete;

	/// Check prior outputs against the needed inputs.
	///
	/// This is called to ensure consistency of this request with
	/// others in the same packet.
	fn check_outputs<F>(&self, f: F) -> Result<(), NoSuchOutput>
		where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>;

	/// Note that this request will produce the following outputs.
	fn note_outputs<F>(&self, f: F) where F: FnMut(usize, OutputKind);

	/// Fill the request.
	///
	/// This function is provided an "output oracle" which allows fetching of
	/// prior request outputs.
	/// Only outputs previously checked with `check_outputs` will be available.
	fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
		where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>;
}

/// Header request.
pub mod header {
	use super::{Field, HashOrNumber, NoSuchOutput, OutputKind, Output};
	use ethcore::encoded;
	use rlp::{Encodable, Decodable, Decoder, DecoderError, RlpStream, Stream};
	use util::U256;

	/// Potentially incomplete headers request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Incomplete {
		/// Start block.
		pub start: Field<HashOrNumber>,
		/// Skip between.
		pub skip: U256,
		/// Maximum to return.
		pub max: U256,
		/// Whether to reverse from start.
		pub reverse: bool,
	}

	impl Decodable for Incomplete {
		fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
			let rlp = decoder.as_rlp();
			Ok(Incomplete {
				start: rlp.val_at(0)?,
				skip: rlp.val_at(1)?,
				max: rlp.val_at(2)?,
				reverse: rlp.val_at(3)?
			})
		}
	}

	impl Encodable for Incomplete {
		fn rlp_append(&self, s: &mut RlpStream) {
			s.begin_list(4)
				.append(&self.start)
				.append(&self.skip)
				.append(&self.max)
				.append(&self.reverse);
		}
	}

	impl super::IncompleteRequest for Incomplete {
		type Complete = Complete;

		fn check_outputs<F>(&self, mut f: F) -> Result<(), NoSuchOutput>
			where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>
		{
			match self.start {
				Field::Scalar(_) => Ok(()),
				Field::BackReference(req, idx) =>
					f(req, idx, OutputKind::Hash).or_else(|| f(req, idx, OutputKind::Number))
			}
		}

		fn note_outputs<F>(&self, _: F) where F: FnMut(usize, OutputKind) { }

		fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
			where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>
		{
			let start = match self.start {
				Field::Scalar(start) => start,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash.into(),
					Output::Number(num) => num.into(),
				}
			};

			Ok(Complete {
				start: start,
				skip: self.skip,
				max: self.max,
				reverse: self.reverse,
			})
		}

	}

	/// A complete header request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Complete {
		/// Start block.
		pub start: HashOrNumber,
		/// Skip between.
		pub skip: U256,
		/// Maximum to return.
		pub max: U256,
		/// Whether to reverse from start.
		pub reverse: bool,
	}

	/// The output of a request for headers.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Response {
		/// The headers requested.
		pub headers: Vec<encoded::Header>,
	}

	impl Response {
		/// Fill reusable outputs by writing them into the function.
		pub fn fill_outputs<F>(&self, _: F) where F: FnMut(usize, Output) { }
	}
}

/// Request and response for header proofs.
pub mod header_proof {
	use super::{Field, NoSuchOutput, OutputKind, Output};
	use rlp::{Encodable, Decodable, Decoder, DecoderError, RlpStream, Stream};
	use util::{Bytes, U256, H256};

	/// Potentially incomplete header proof request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Incomplete {
		/// Block number.
		pub num: Field<u64>,
	}

	impl Decodable for Incomplete {
		fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
			let rlp = decoder.as_rlp();
			Ok(Incomplete {
				num: rlp.val_at(0)?,
			})
		}
	}

	impl Encodable for Incomplete {
		fn rlp_append(&self, s: &mut RlpStream) {
			s.begin_list(1).append(&self.num);
		}
	}

	impl super::IncompleteRequest for Incomplete {
		type Complete = Complete;

		fn check_outputs<F>(&self, mut f: F) -> Result<(), NoSuchOutput>
			where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>
		{
			match self.num {
				Field::Scalar(_) => Ok(()),
				Field::BackReference(req, idx) => f(req, idx, OutputKind::Number),
			}
		}

		fn note_outputs<F>(&self, mut note: F) where F: FnMut(usize, OutputKind) {
			note(1, OutputKind::Hash);
		}

		fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
			where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>
		{
			let num = match self.num {
				Field::Scalar(num) => num,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Number(num) => num,
					_ => return Err(NoSuchOutput),
				}
			};

			Ok(Complete {
				num: num,
			})
		}

	}

	/// A complete header proof request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Complete {
		/// The number to get a header proof for.
		pub num: u64,
	}

	/// The output of a request for a header proof.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Response {
		/// Inclusion proof of the header and total difficulty in the CHT.
		pub proof: Vec<Bytes>,
		/// The proved header's hash.
		pub hash: H256,
		/// The proved header's total difficulty.
		pub td: U256,
	}

	impl Response {
		/// Fill reusable outputs by providing them to the function.
		pub fn fill_outputs<F>(&self, mut f: F) where F: FnMut(usize, Output) {
			f(1, Output::Hash(self.hash));
		}
	}
}

/// Request and response for block receipts
pub mod block_receipts {
	use super::{Field, NoSuchOutput, OutputKind, Output};
	use rlp::{Encodable, Decodable, Decoder, DecoderError, RlpStream, Stream};
	use util::{Bytes, U256, H256};

	/// Potentially incomplete block receipts request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Incomplete {
		/// Block hash to get receipts for.
		pub hash: Field<H256>,
	}

	impl Decodable for Incomplete {
		fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
			let rlp = decoder.as_rlp();
			Ok(Incomplete {
				hash: rlp.val_at(0)?,
			})
		}
	}

	impl Encodable for Incomplete {
		fn rlp_append(&self, s: &mut RlpStream) {
			s.begin_list(1).append(&self.hash);
		}
	}

	impl super::IncompleteRequest for Incomplete {
		type Complete = Complete;

		fn check_outputs<F>(&self, mut f: F) -> Result<(), NoSuchOutput>
			where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>
		{
			match self.num {
				Field::Scalar(_) => Ok(()),
				Field::BackReference(req, idx) => f(req, idx, OutputKind::Hash),
			}
		}

		fn note_outputs<F>(&self, _: F) where F: FnMut(usize, OutputKind) {}

		fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
			where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>
		{
			let hash = match self.hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput),
				}
			};

			Ok(Complete {
				hash: hash,
			})
		}
	}

	/// A complete block receipts request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Complete {
		/// The number to get block receipts for.
		pub hash: H256,
	}

	/// The output of a request for block receipts.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Response {
		/// The block receipts.
		pub receipts: Vec<Receipt>
	}

	impl Response {
		/// Fill reusable outputs by providing them to the function.
		pub fn fill_outputs<F>(&self, _: F) where F: FnMut(usize, Output) {}
	}
}

/// Request and response for a block body
pub mod block_body {
	use super::{Field, NoSuchOutput, OutputKind, Output};
	use ethcore::encoded;
	use rlp::{Encodable, Decodable, Decoder, DecoderError, RlpStream, Stream};
	use util::{Bytes, U256, H256};

	/// Potentially incomplete block body request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Incomplete {
		/// Block hash to get receipts for.
		pub hash: Field<H256>,
	}

	impl Decodable for Incomplete {
		fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
			let rlp = decoder.as_rlp();
			Ok(Incomplete {
				hash: rlp.val_at(0)?,
			})
		}
	}

	impl Encodable for Incomplete {
		fn rlp_append(&self, s: &mut RlpStream) {
			s.begin_list(1).append(&self.hash);
		}
	}

	impl super::IncompleteRequest for Incomplete {
		type Complete = Complete;

		fn check_outputs<F>(&self, mut f: F) -> Result<(), NoSuchOutput>
			where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>
		{
			match self.num {
				Field::Scalar(_) => Ok(()),
				Field::BackReference(req, idx) => f(req, idx, OutputKind::Hash),
			}
		}

		fn note_outputs<F>(&self, _: F) where F: FnMut(usize, OutputKind) {}

		fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
			where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>
		{
			let hash = match self.hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput),
				}
			};

			Ok(Complete {
				hash: hash,
			})
		}

	}

	/// A complete block body request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Complete {
		/// The hash to get a block body for.
		pub hash: H256,
	}

	/// The output of a request for block body.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Response {
		/// The block body.
		pub body: encoded::Body,
	}

	impl Response {
		/// Fill reusable outputs by providing them to the function.
		pub fn fill_outputs<F>(&self, _: F) where F: FnMut(usize, Output) {}
	}
}

/// A request for an account proof.
pub mod account {
	use super::{Field, NoSuchOutput, OutputKind, Output};
	use ethcore::encoded;
	use rlp::{Encodable, Decodable, Decoder, DecoderError, RlpStream, Stream};
	use util::{Bytes, U256, H256};

	/// Potentially incomplete request for an account proof.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Incomplete {
		/// Block hash to request state proof for.
		pub block_hash: Field<H256>,
		/// Hash of the account's address.
		pub address_hash: Field<H256>,
	}

	impl Decodable for Incomplete {
		fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
			let rlp = decoder.as_rlp();
			Ok(Incomplete {
				block_hash: rlp.val_at(0)?,
				address_hash: rlp.val_at(1)?,
			})
		}
	}

	impl Encodable for Incomplete {
		fn rlp_append(&self, s: &mut RlpStream) {
			s.begin_list(2)
				.append(&self.block_hash)
				.append(&self.address_hash);
		}
	}

	impl super::IncompleteRequest for Incomplete {
		type Complete = Complete;

		fn check_outputs<F>(&self, mut f: F) -> Result<(), NoSuchOutput>
			where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>
		{
			if let Field::BackReference(req, idx) = self.block_hash {
				f(req, idx, OutputKind::Hash)?
			}

			if let Field::BackReference(req, idx) = self.address_hash {
				f(req, idx, OutputKind::Hash)?
			}

			Ok(())
		}

		fn note_outputs<F>(&self, mut f: F) where F: FnMut(usize, OutputKind) {
			f(0, OutputKind::Hash);
			f(1, OutputKind::Hash);
		}

		fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
			where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>
		{
			let block_hash = match self.block_hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput)?,
				}
			};

			let address_hash = match self.address_hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput)?,
				}
			};

			Ok(Complete {
				block_hash: block_hash,
				address_hash: address_hash,
			})
		}

	}

	/// A complete request for an account.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Complete {
		/// Block hash to request state proof for.
		pub block_hash: H256,
		/// Hash of the account's address.
		pub address_hash: H256,
	}

	/// The output of a request for an account state proof.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Response {
		/// Inclusion/exclusion proof
		pub proof: Vec<Bytes>,
		/// Account nonce.
		pub nonce: U256,
		/// Account balance.
		pub balance: U256,
		/// Account's code hash.
		pub code_hash: H256,
		/// Account's storage trie root.
		pub storage_root: H256,
	}

	impl Response {
		/// Fill reusable outputs by providing them to the function.
		pub fn fill_outputs<F>(&self, mut f: F) where F: FnMut(usize, Output) {
			f(0, Output::Hash(self.code_hash));
			f(1, Output::Hash(self.storage_root));
		}
	}
}

/// A request for a storage proof.
pub mod storage {
	use super::{Field, NoSuchOutput, OutputKind, Output};
	use ethcore::encoded;
	use rlp::{Encodable, Decodable, Decoder, DecoderError, RlpStream, Stream};
	use util::{Bytes, U256, H256};

	/// Potentially incomplete request for an storage proof.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Incomplete {
		/// Block hash to request state proof for.
		pub block_hash: Field<H256>,
		/// Hash of the account's address.
		pub address_hash: Field<H256>,
		/// Hash of the storage key.
		pub key_hash: Field<H256>,
	}

	impl Decodable for Incomplete {
		fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
			let rlp = decoder.as_rlp();
			Ok(Incomplete {
				block_hash: rlp.val_at(0)?,
				address_hash: rlp.val_at(1)?,
				key_hash: rlp.val_at(2)?,
			})
		}
	}

	impl Encodable for Incomplete {
		fn rlp_append(&self, s: &mut RlpStream) {
			s.begin_list(3)
				.append(&self.block_hash)
				.append(&self.address_hash)
				.append(&self.key_hash);
		}
	}

	impl super::IncompleteRequest for Incomplete {
		type Complete = Complete;

		fn check_outputs<F>(&self, mut f: F) -> Result<(), NoSuchOutput>
			where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>
		{
			if let Field::BackReference(req, idx) = self.block_hash {
				f(req, idx, OutputKind::Hash)?
			}

			if let Field::BackReference(req, idx) = self.address_hash {
				f(req, idx, OutputKind::Hash)?
			}

			if let Field::BackReference(req, idx) = self.key_hash {
				f(req, idx, OutputKind::Hash)?
			}

			Ok(())
		}

		fn note_outputs<F>(&self, mut f: F) where F: FnMut(usize, OutputKind) {
			f(0, OutputKind::Hash);
		}

		fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
			where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>
		{
			let block_hash = match self.block_hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput)?,
				}
			};

			let address_hash = match self.address_hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput)?,
				}
			};

			let key_hash = match self.key_hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput)?,
				}
			};

			Ok(Complete {
				block_hash: block_hash,
				address_hash: address_hash,
				key_hash: key_hash
			})
		}

	}

	/// A complete request for a storage proof.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Complete {
		/// Block hash to request state proof for.
		pub block_hash: H256,
		/// Hash of the account's address.
		pub address_hash: H256,
		/// Storage key hash.
		pub key_hash: H256,
	}

	/// The output of a request for an account state proof.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Response {
		/// Inclusion/exclusion proof
		pub proof: Vec<Bytes>,
		/// Storage value.
		pub value: H256,
	}

	impl Response {
		/// Fill reusable outputs by providing them to the function.
		pub fn fill_outputs<F>(&self, mut f: F) where F: FnMut(usize, Output) {
			f(0, Output::Hash(self.value));
		}
	}
}

/// A request for contract code.
pub mod contract_code {
	use super::{Field, NoSuchOutput, OutputKind, Output};
	use ethcore::encoded;
	use rlp::{Encodable, Decodable, Decoder, DecoderError, RlpStream, Stream};
	use util::{Bytes, U256, H256};

	/// Potentially incomplete _ request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Incomplete {
		/// The block hash to request the state for.
		pub block_hash: Field<H256>,
		/// The code hash.
		pub code_hash: Field<H256>,
	}

	impl Decodable for Incomplete {
		fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
			let rlp = decoder.as_rlp();
			Ok(Incomplete {
				block_hash: rlp.val_at(0)?,
				code_hash: rlp.val_at(1)?,
			})
		}
	}

	impl Encodable for Incomplete {
		fn rlp_append(&self, s: &mut RlpStream) {
			s.begin_list(2)
				.append(&self.block_hash)
				.append(&self.code_hash);
		}
	}

	impl super::IncompleteRequest for Incomplete {
		type Complete = Complete;

		fn check_outputs<F>(&self, mut f: F) -> Result<(), NoSuchOutput>
			where F: FnMut(usize, usize, OutputKind) -> Result<(), NoSuchOutput>
		{
			if let Field::BackReference(req, idx) = self.block_hash {
				f(req, idx, OutputKind::Hash)?;
			}
			if let Field::BackReference(req, idx) = self.code_hash {
				f(req, idx, OutputKind::Hash)?;
			}

			Ok(())
		}

		fn note_outputs<F>(&self, _: F) where F: FnMut(usize, OutputKind) {}

		fn fill<F>(self, oracle: F) -> Result<Self::Complete, NoSuchOutput>
			where F: Fn(usize, usize) -> Result<Output, NoSuchOutput>
		{
			let block_hash = match self.block_hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput)?,
				}
			};

			let code_hash = match self.code_hash {
				Field::Scalar(hash) => hash,
				Field::BackReference(req, idx) => match oracle(req, idx)? {
					Output::Hash(hash) => hash,
					_ => return Err(NoSuchOutput)?,
				}
			};

			Ok(Complete {
				block_hash: block_hash,
				code_hash: code_hash,
			})
		}

	}

	/// A complete request.
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Complete {
		/// The block hash to request the state for.
		pub block_hash: H256,
		/// The code hash.
		pub code_hash: H256,
	}

	/// The output of a request for
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Response {
		/// The requested code.
		pub code: Bytes,
	}

	impl Response {
		/// Fill reusable outputs by providing them to the function.
		pub fn fill_outputs<F>(&self, _: F) where F: FnMut(usize, Output) {}
	}
}
