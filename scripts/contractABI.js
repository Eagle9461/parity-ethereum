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

// Rust/Parity ABI struct autogenerator.
// By Gav Wood, 2016.

String.prototype.replaceAll = function(f, t) { return this.split(f).join(t); }
String.prototype.toSnake = function(){
	return this.replace(/([A-Z])/g, function($1){return "_"+$1.toLowerCase();});
};

function makeContractFile(name, json, prefs) {
	return `// Autogenerated from JSON contract definition using Rust contract convertor.

use std::string::String;
use std::result::Result;
use std::fmt;
use {util, ethabi};
use util::{FixedHash, Uint};

${convertContract(name, json, prefs)}
`;
}

function convertContract(name, json, prefs) {
	return `${prefs._pub ? "pub " : ""}struct ${name} {
	contract: ethabi::Contract,
	address: util::Address,
	do_call: Box<Fn(util::Address, Vec<u8>) -> Result<Vec<u8>, String> + Send + 'static>,
}
impl ${name} {
	pub fn new<F>(address: util::Address, do_call: F) -> Self where F: Fn(util::Address, Vec<u8>) -> Result<Vec<u8>, String> + Send + 'static {
		${name} {
			contract: ethabi::Contract::new(ethabi::Interface::load(b"${JSON.stringify(json.filter(a => a.type == 'function')).replaceAll('"', '\\"')}").expect("JSON is autogenerated; qed")),
			address: address,
			do_call: Box::new(do_call),
		}
	}
	fn as_string<T: fmt::Debug>(e: T) -> String { format!("{:?}", e) }
	${json.filter(x => x.type == 'function').map(x => convertFunction(x, prefs)).join("\n")}
}`;
}

function mapType(name, type, _prefs) {
	let prefs = _prefs || {};
	var m;
	if ((m = type.match(/^bytes(\d+)$/)) != null && m[1] <= 32) {
		if (prefs['string'])
			return `&str`;
		else
			return `&util::H${m[1] * 8}`;
	}
	if ((m = type.match(/^(u?)int(\d+)$/)) != null) {
		var n = [8, 16, 32, 64, 128, 160, 256].filter(i => m[2] <= i)[0];
		if (n) {
			if (n <= 64)
				return `${m[1] == 'u' ? 'u' : 'i'}${n}`;
			if (m[1] == 'u')
				return `util::U${n}`;
			// ERROR - unsupported integer (signed > 32 or unsigned > 256)
		}
	}
	if (type == "address")
		return "&util::Address";
	if (type == "bool")
		return "bool";
	if (type == "string")
		return "&str";
	if (type == "bytes")
		return "&[u8]";

	console.log(`Unsupported argument type: ${type} (${name})`);
}

function mapReturnType(name, type, _prefs) {
	let prefs = _prefs || {};
	var m;
	if ((m = type.match(/^bytes(\d+)$/)) != null && m[1] <= 32) {
		if (prefs['string'])
			return `String`;
		else
			return `util::H${m[1] * 8}`;
	}
	if ((m = type.match(/^(u?)int(\d+)$/)) != null) {
		var n = [8, 16, 32, 64, 128, 160, 256].filter(i => m[2] <= i)[0];
		if (n) {
			if (n <= 64)
				return `${m[1] == 'u' ? 'u' : 'i'}${n}`;
			if (m[1] == 'u')
				return `util::U${n}`;
			// ERROR - unsupported integer (signed > 32 or unsigned > 256)
		}
	}
	if (type == "address")
		return "util::Address";
	if (type == "bool")
		return "bool";
	if (type == "string")
		return "String";
	if (type == "bytes")
		return "Vec<u8>";

	console.log(`Unsupported argument type: ${type} (${name})`);
}

function convertToken(name, type, _prefs) {
	let prefs = _prefs || {};
	var m;
	if ((m = type.match(/^bytes(\d+)$/)) != null && m[1] <= 32) {
		if (prefs['string'])
			return `ethabi::Token::FixedBytes(${name}.as_bytes().to_owned())`;
		else
			return `ethabi::Token::FixedBytes(${name}.as_ref().to_owned())`;
	}
	if ((m = type.match(/^(u?)int(\d+)$/)) != null) {
		var n = [8, 16, 32, 64, 128, 160, 256].filter(i => m[2] <= i)[0];
		if (n) {
			if (m[1] == 'u')
				return `ethabi::Token::Uint({ let mut r = [0u8; 32]; ${n <= 64 ? "util::U256::from(" + name + " as u64)" : name}.to_big_endian(&mut r); r })`;
			else if (n <= 32)
				return `ethabi::Token::Int(pad_i32(${name} as i32))`;
			// ERROR - unsupported integer (signed > 32 or unsigned > 256)
		}
	}
	if (type == "address")
		return `ethabi::Token::Address(${name}.clone().0)`;
	if (type == "bool")
		return `ethabi::Token::Bool(${name})`;
	if (type == "string")
		return `ethabi::Token::String(${name}.to_owned())`;
	if (type == "bytes")
		return `ethabi::Token::Bytes(${name}.to_owned())`;

	console.log(`Unsupported argument type: ${type} (${name})`);
}

function tokenType(name, type, _prefs) {
	let prefs = _prefs || {};
	var m;
	if ((m = type.match(/^bytes(\d+)$/)) != null && m[1] <= 32)
		return `${name}.to_fixed_bytes()`;
	if ((m = type.match(/^(u?)int(\d+)$/)) != null) {
		return `${name}.to_${m[1]}int()`;
	}
	if (type == "address")
		return `${name}.to_address()`;
	if (type == "bool")
		return `${name}.to_bool()`;
	if (type == "string")
		return `${name}.to_string()`;
	if (type == "bytes")
		return `${name}.to_bytes()`;

	// ERROR - unsupported
}

function tokenCoerce(name, type, _prefs) {
	let prefs = _prefs || {};
	var m;
	if ((m = type.match(/^bytes(\d+)$/)) != null && m[1] <= 32) {
		if (prefs['string'])
			return `String::from_utf8(${name}).unwrap_or_else(String::new)`;
		else
			return `util::H${m[1] * 8}::from_slice(${name}.as_ref())`;
	}
	if ((m = type.match(/^(u?)int(\d+)$/)) != null) {
		var n = [8, 16, 32, 64, 128, 160, 256].filter(i => m[2] <= i)[0];
		if (n && m[1] == 'u')
			return `util::U${n <= 64 ? 256 : n}::from(${name}.as_ref())` + (n <= 64 ? `.as_u64() as u${n}` : '');
		// ERROR - unsupported integer (signed or unsigned > 256)
	}
	if (type == "address")
		return `util::Address::from(${name})`;
	if (type == "bool")
		return `${name}`;
	if (type == "string")
		return `${name}`;
	if (type == "bytes")
		return `${name}`;

	console.log(`Unsupported return type: ${type} (${name})`);
}

function tokenExtract(expr, type, _prefs) {
	return `{ let r = ${expr}; let r = try!(${tokenType('r', type, _prefs)}.ok_or("Invalid type returned")); ${tokenCoerce('r', type, _prefs)} }`;
}

function convertFunction(json, _prefs) {
	let prefs = (_prefs || {})[json.name] || (_prefs || {})['_'] || {};
	let snakeName = json.name.toSnake();
	let params = json.inputs.map((x, i) => (x.name ? x.name.toSnake() : ("_" + (i + 1))) + ": " + mapType(x.name, x.type, prefs[x.name]));
	let returns = json.outputs.length != 1 ? "(" + json.outputs.map(x => mapReturnType(x.name, x.type, prefs[x.name])).join(", ") + ")" : mapReturnType(json.outputs[0].name, json.outputs[0].type, prefs[json.outputs[0].name]); 
	return `
	/// Auto-generated from: \`${JSON.stringify(json)}\`
	#[allow(dead_code)]
	pub fn ${snakeName}(&self${params.length > 0 ? ', ' + params.join(", ") : ''}) -> Result<${returns}, String> { 
		let call = self.contract.function("${json.name}".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![${json.inputs.map((x, i) => convertToken(x.name ? x.name.toSnake() : ("_" + (i + 1)), x.type, prefs[x.name])).join(', ')}]
		).map_err(Self::as_string)?;
		${json.outputs.length > 0 ? 'let output = ' : ''}call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		${json.outputs.length > 0 ? 'let mut result = output.into_iter().rev().collect::<Vec<_>>();' : ''}
		Ok((${json.outputs.map((o, i) => tokenExtract('result.pop().ok_or("Invalid return arity")?', o.type, prefs[o.name])).join(', ')})) 
	}`;
}

jsonabi = [{"constant":false,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_newOwner","type":"address"}],"name":"resetClientOwner","outputs":[],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_release","type":"bytes32"}],"name":"isLatest","outputs":[{"name":"","type":"bool"}],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_txid","type":"bytes32"}],"name":"rejectTransaction","outputs":[],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_newOwner","type":"address"}],"name":"setOwner","outputs":[],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_number","type":"uint32"},{"name":"_name","type":"bytes32"},{"name":"_hard","type":"bool"},{"name":"_spec","type":"bytes32"}],"name":"proposeFork","outputs":[],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_client","type":"bytes32"}],"name":"removeClient","outputs":[],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_release","type":"bytes32"}],"name":"release","outputs":[{"name":"o_forkBlock","type":"uint32"},{"name":"o_track","type":"uint8"},{"name":"o_semver","type":"uint24"},{"name":"o_critical","type":"bool"}],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_checksum","type":"bytes32"}],"name":"build","outputs":[{"name":"o_release","type":"bytes32"},{"name":"o_platform","type":"bytes32"}],"payable":false,"type":"function"},{"constant":false,"inputs":[],"name":"rejectFork","outputs":[],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"","type":"bytes32"}],"name":"client","outputs":[{"name":"owner","type":"address"},{"name":"required","type":"bool"}],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_newOwner","type":"address"}],"name":"setClientOwner","outputs":[],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"","type":"uint32"}],"name":"fork","outputs":[{"name":"name","type":"bytes32"},{"name":"spec","type":"bytes32"},{"name":"hard","type":"bool"},{"name":"ratified","type":"bool"},{"name":"requiredCount","type":"uint256"}],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_release","type":"bytes32"},{"name":"_platform","type":"bytes32"},{"name":"_checksum","type":"bytes32"}],"name":"addChecksum","outputs":[],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_txid","type":"bytes32"}],"name":"confirmTransaction","outputs":[{"name":"txSuccess","type":"uint256"}],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"","type":"bytes32"}],"name":"proxy","outputs":[{"name":"requiredCount","type":"uint256"},{"name":"to","type":"address"},{"name":"data","type":"bytes"},{"name":"value","type":"uint256"},{"name":"gas","type":"uint256"}],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_owner","type":"address"}],"name":"addClient","outputs":[],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"","type":"address"}],"name":"clientOwner","outputs":[{"name":"","type":"bytes32"}],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_txid","type":"bytes32"},{"name":"_to","type":"address"},{"name":"_data","type":"bytes"},{"name":"_value","type":"uint256"},{"name":"_gas","type":"uint256"}],"name":"proposeTransaction","outputs":[{"name":"txSuccess","type":"uint256"}],"payable":false,"type":"function"},{"constant":true,"inputs":[],"name":"grandOwner","outputs":[{"name":"","type":"address"}],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_release","type":"bytes32"},{"name":"_forkBlock","type":"uint32"},{"name":"_track","type":"uint8"},{"name":"_semver","type":"uint24"},{"name":"_critical","type":"bool"}],"name":"addRelease","outputs":[],"payable":false,"type":"function"},{"constant":false,"inputs":[],"name":"acceptFork","outputs":[],"payable":false,"type":"function"},{"constant":true,"inputs":[],"name":"clientsRequired","outputs":[{"name":"","type":"uint32"}],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_release","type":"bytes32"}],"name":"track","outputs":[{"name":"","type":"uint8"}],"payable":false,"type":"function"},{"constant":false,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_r","type":"bool"}],"name":"setClientRequired","outputs":[],"payable":false,"type":"function"},{"constant":true,"inputs":[],"name":"latestFork","outputs":[{"name":"","type":"uint32"}],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_track","type":"uint8"}],"name":"latestInTrack","outputs":[{"name":"","type":"bytes32"}],"payable":false,"type":"function"},{"constant":true,"inputs":[{"name":"_client","type":"bytes32"},{"name":"_release","type":"bytes32"},{"name":"_platform","type":"bytes32"}],"name":"checksum","outputs":[{"name":"","type":"bytes32"}],"payable":false,"type":"function"},{"constant":true,"inputs":[],"name":"proposedFork","outputs":[{"name":"","type":"uint32"}],"payable":false,"type":"function"},{"inputs":[],"payable":false,"type":"constructor"},{"payable":true,"type":"fallback"},{"anonymous":false,"inputs":[{"indexed":true,"name":"from","type":"address"},{"indexed":false,"name":"value","type":"uint256"},{"indexed":false,"name":"data","type":"bytes"}],"name":"Received","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"txid","type":"bytes32"},{"indexed":true,"name":"to","type":"address"},{"indexed":false,"name":"data","type":"bytes"},{"indexed":false,"name":"value","type":"uint256"},{"indexed":false,"name":"gas","type":"uint256"}],"name":"TransactionProposed","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"txid","type":"bytes32"}],"name":"TransactionConfirmed","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"txid","type":"bytes32"}],"name":"TransactionRejected","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"txid","type":"bytes32"},{"indexed":false,"name":"success","type":"bool"}],"name":"TransactionRelayed","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"number","type":"uint32"},{"indexed":true,"name":"name","type":"bytes32"},{"indexed":false,"name":"spec","type":"bytes32"},{"indexed":false,"name":"hard","type":"bool"}],"name":"ForkProposed","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"number","type":"uint32"}],"name":"ForkAcceptedBy","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"number","type":"uint32"}],"name":"ForkRejectedBy","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"forkNumber","type":"uint32"}],"name":"ForkRejected","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"forkNumber","type":"uint32"}],"name":"ForkRatified","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"forkBlock","type":"uint32"},{"indexed":false,"name":"release","type":"bytes32"},{"indexed":false,"name":"track","type":"uint8"},{"indexed":false,"name":"semver","type":"uint24"},{"indexed":true,"name":"critical","type":"bool"}],"name":"ReleaseAdded","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"release","type":"bytes32"},{"indexed":true,"name":"platform","type":"bytes32"},{"indexed":false,"name":"checksum","type":"bytes32"}],"name":"ChecksumAdded","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":false,"name":"owner","type":"address"}],"name":"ClientAdded","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"}],"name":"ClientRemoved","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":true,"name":"old","type":"address"},{"indexed":true,"name":"now","type":"address"}],"name":"ClientOwnerChanged","type":"event"},{"anonymous":false,"inputs":[{"indexed":true,"name":"client","type":"bytes32"},{"indexed":false,"name":"now","type":"bool"}],"name":"ClientRequiredChanged","type":"event"},{"anonymous":false,"inputs":[{"indexed":false,"name":"old","type":"address"},{"indexed":false,"name":"now","type":"address"}],"name":"OwnerChanged","type":"event"}];

makeContractFile("Operations", jsonabi, {"_pub": true, "_": {"_client": {"string": true}, "_platform": {"string": true}}});
