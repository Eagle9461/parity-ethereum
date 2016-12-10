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

//! Defines error types and levels of punishment to use upon
//! encountering.

use rlp::DecoderError;
use network::NetworkError;

use std::fmt;

/// Levels of punishment.
///
/// Currently just encompasses two different kinds of disconnect and
/// no punishment, but this is where reputation systems might come into play.
// In ascending order
#[derive(Debug, PartialEq, Eq)]
pub enum Punishment {
	/// Perform no punishment.
	None,
	/// Disconnect the peer, but don't prevent them from reconnecting.
	Disconnect,
	/// Disconnect the peer and prevent them from reconnecting.
	Disable,
}

/// Kinds of errors which can be encountered in the course of LES.
#[derive(Debug)]
pub enum Error {
	/// An RLP decoding error.
	Rlp(DecoderError),
	/// A network error.
	Network(NetworkError),
	/// Out of buffer.
	BufferEmpty,
	/// Unrecognized packet code.
	UnrecognizedPacket(u8),
	/// Unexpected handshake.
	UnexpectedHandshake,
	/// Peer on wrong network (wrong NetworkId or genesis hash)
	WrongNetwork,
	/// Unknown peer.
	UnknownPeer,
	/// Unsolicited response.
	UnsolicitedResponse,
	/// Not a server.
	NotServer,
	/// Unsupported protocol version.
	UnsupportedProtocolVersion(u8),
	/// Bad protocol version.
	BadProtocolVersion,
}

impl Error {
	/// What level of punishment does this error warrant?
	pub fn punishment(&self) -> Punishment {
		match *self {
			Error::Rlp(_) => Punishment::Disable,
			Error::Network(_) => Punishment::None,
			Error::BufferEmpty => Punishment::Disable,
			Error::UnrecognizedPacket(_) => Punishment::Disconnect,
			Error::UnexpectedHandshake => Punishment::Disconnect,
			Error::WrongNetwork => Punishment::Disable,
			Error::UnknownPeer => Punishment::Disconnect,
			Error::UnsolicitedResponse => Punishment::Disable,
			Error::NotServer => Punishment::Disable,
			Error::UnsupportedProtocolVersion(_) => Punishment::Disable,
			Error::BadProtocolVersion => Punishment::Disable,
		}
	}
}

impl From<DecoderError> for Error {
	fn from(err: DecoderError) -> Self {
		Error::Rlp(err)
	}
}

impl From<NetworkError> for Error {
	fn from(err: NetworkError) -> Self {
		Error::Network(err)
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::Rlp(ref err) => err.fmt(f),
			Error::Network(ref err) => err.fmt(f),
			Error::BufferEmpty => write!(f, "Out of buffer"),
			Error::UnrecognizedPacket(code) => write!(f, "Unrecognized packet: 0x{:x}", code),
			Error::UnexpectedHandshake => write!(f, "Unexpected handshake"),
			Error::WrongNetwork => write!(f, "Wrong network"),
			Error::UnknownPeer => write!(f, "Unknown peer"),
			Error::UnsolicitedResponse => write!(f, "Peer provided unsolicited data"),
			Error::NotServer => write!(f, "Peer not a server."),
			Error::UnsupportedProtocolVersion(pv) => write!(f, "Unsupported protocol version: {}", pv),	
			Error::BadProtocolVersion => write!(f, "Bad protocol version in handshake"),
		}
	}
}