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

//! Hyper Server Handler that fetches a file during a request (proxy).

use std::{fs, fmt};
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, Duration};
use util::Mutex;

use hyper::{server, Decoder, Encoder, Next, Method, Control};
use hyper::net::HttpStream;
use hyper::status::StatusCode;

use handlers::{ContentHandler, Redirection};
use handlers::client::{Client, FetchResult};
use apps::redirection_address;
use apps::urlhint::GithubApp;
use apps::manifest::Manifest;

const FETCH_TIMEOUT: u64 = 30;

enum FetchState {
	NotStarted(GithubApp),
	InProgress(mpsc::Receiver<FetchResult>),
	Error(ContentHandler),
	Done(Manifest, Redirection),
}

pub trait ContentValidator {
	type Error: fmt::Debug + fmt::Display;

	fn validate_and_install(&self, app: PathBuf) -> Result<Manifest, Self::Error>;
	fn done(&self, Option<&Manifest>);
}

pub struct FetchControl {
	abort: Arc<AtomicBool>,
	listeners: Mutex<Vec<(Control, mpsc::Sender<FetchState>)>>,
	deadline: Instant,
}

impl Default for FetchControl {
	fn default() -> Self {
		FetchControl {
			abort: Arc::new(AtomicBool::new(false)),
			listeners: Mutex::new(Vec::new()),
			deadline: Instant::now() + Duration::from_secs(FETCH_TIMEOUT),
		}
	}
}

impl FetchControl {
	fn notify<F: Fn() -> FetchState>(&self, status: F) {
		let mut listeners = self.listeners.lock();
		for (control, sender) in listeners.drain(..) {
			if let Err(e) = sender.send(status()) {
				trace!(target: "dapps", "Waiting listener notification failed: {:?}", e);
			} else {
				let _ = control.ready(Next::read());
			}
		}
	}

	fn set_status(&self, status: &FetchState) {
		match *status {
			FetchState::Error(ref handler) => self.notify(|| FetchState::Error(handler.clone())),
			FetchState::Done(ref manifest, ref handler) => self.notify(|| FetchState::Done(manifest.clone(), handler.clone())),
			FetchState::NotStarted(_) | FetchState::InProgress(_) => {},
		}
	}

	pub fn abort(&self) {
		self.abort.store(true, Ordering::SeqCst);
	}

	pub fn to_handler(&self, control: Control) -> Box<server::Handler<HttpStream> + Send> {
		let (tx, rx) = mpsc::channel();
		self.listeners.lock().push((control, tx));

		Box::new(WaitingHandler {
			receiver: rx,
			state: None,
		})
	}
}

pub struct WaitingHandler {
	receiver: mpsc::Receiver<FetchState>,
	state: Option<FetchState>,
}

impl server::Handler<HttpStream> for WaitingHandler {
	fn on_request(&mut self, _request: server::Request<HttpStream>) -> Next {
		Next::wait()
	}

	fn on_request_readable(&mut self, _decoder: &mut Decoder<HttpStream>) -> Next {
		self.state = self.receiver.try_recv().ok();
		Next::write()
	}

	fn on_response(&mut self, res: &mut server::Response) -> Next {
		match self.state {
			Some(FetchState::Done(_, ref mut handler)) => handler.on_response(res),
			Some(FetchState::Error(ref mut handler)) => handler.on_response(res),
			_ => Next::end(),
		}
	}

	fn on_response_writable(&mut self, encoder: &mut Encoder<HttpStream>) -> Next {
		match self.state {
			Some(FetchState::Done(_, ref mut handler)) => handler.on_response_writable(encoder),
			Some(FetchState::Error(ref mut handler)) => handler.on_response_writable(encoder),
			_ => Next::end(),
		}
	}
}

pub struct ContentFetcherHandler<H: ContentValidator> {
	fetch_control: Arc<FetchControl>,
	status: FetchState,
	control: Option<Control>,
	client: Option<Client>,
	using_dapps_domains: bool,
	dapp: H,
}

impl<H: ContentValidator> Drop for ContentFetcherHandler<H> {
	fn drop(&mut self) {
		let manifest = match self.status {
			FetchState::Done(ref manifest, _) => Some(manifest),
			_ => None,
		};
		self.dapp.done(manifest);
	}
}

impl<H: ContentValidator> ContentFetcherHandler<H> {

	pub fn new(
		app: GithubApp,
		control: Control,
		using_dapps_domains: bool,
		handler: H) -> (Self, Arc<FetchControl>) {

		let fetch_control = Arc::new(FetchControl::default());
		let client = Client::new();
		let handler = ContentFetcherHandler {
			fetch_control: fetch_control.clone(),
			control: Some(control),
			client: Some(client),
			status: FetchState::NotStarted(app),
			using_dapps_domains: using_dapps_domains,
			dapp: handler,
		};

		(handler, fetch_control)
	}

	fn close_client(client: &mut Option<Client>) {
		client.take()
			.expect("After client is closed we are going into write, hence we can never close it again")
			.close();
	}

	fn fetch_app(client: &mut Client, app: &GithubApp, abort: Arc<AtomicBool>, control: Control) -> Result<mpsc::Receiver<FetchResult>, String> {
		let res = client.request(app.url(), abort, Box::new(move || {
			trace!(target: "dapps", "Fetching finished.");
			// Ignoring control errors
			let _ = control.ready(Next::read());
		})).map_err(|e| format!("{:?}", e));
		res
	}
}

impl<H: ContentValidator> server::Handler<HttpStream> for ContentFetcherHandler<H> {
	fn on_request(&mut self, request: server::Request<HttpStream>) -> Next {
		let status = if let FetchState::NotStarted(ref app) = self.status {
			Some(match *request.method() {
				// Start fetching content
				Method::Get => {
					trace!(target: "dapps", "Fetching dapp: {:?}", app);
					let control = self.control.take().expect("on_request is called only once, thus control is always Some");
					let client = self.client.as_mut().expect("on_request is called before client is closed.");
					let fetch = Self::fetch_app(client, app, self.fetch_control.abort.clone(), control);
					match fetch {
						Ok(receiver) => FetchState::InProgress(receiver),
						Err(e) => FetchState::Error(ContentHandler::error(
							StatusCode::BadGateway,
							"Unable To Start Dapp Download",
							"Could not initialize download of the dapp. It might be a problem with the remote server.",
							Some(&format!("{}", e)),
						)),
					}
				},
				// or return error
				_ => FetchState::Error(ContentHandler::error(
					StatusCode::MethodNotAllowed,
					"Method Not Allowed",
					"Only <code>GET</code> requests are allowed.",
					None,
				)),
			})
		} else { None };

		if let Some(status) = status {
			self.fetch_control.set_status(&status);
			self.status = status;
		}

		Next::read()
	}

	fn on_request_readable(&mut self, decoder: &mut Decoder<HttpStream>) -> Next {
		let (status, next) = match self.status {
			// Request may time out
			FetchState::InProgress(_) if self.fetch_control.deadline < Instant::now() => {
				trace!(target: "dapps", "Fetching dapp failed because of timeout.");
				let timeout = ContentHandler::error(
					StatusCode::GatewayTimeout,
					"Download Timeout",
					&format!("Could not fetch dapp bundle within {} seconds.", FETCH_TIMEOUT),
					None
				);
				Self::close_client(&mut self.client);
				(Some(FetchState::Error(timeout)), Next::write())
			},
			FetchState::InProgress(ref receiver) => {
				// Check if there is an answer
				let rec = receiver.try_recv();
				match rec {
					// Unpack and validate
					Ok(Ok(path)) => {
						trace!(target: "dapps", "Fetching dapp finished. Starting validation.");
						Self::close_client(&mut self.client);
						// Unpack and verify
						let state = match self.dapp.validate_and_install(path.clone()) {
							Err(e) => {
								trace!(target: "dapps", "Error while validating dapp: {:?}", e);
								FetchState::Error(ContentHandler::error(
									StatusCode::BadGateway,
									"Invalid Dapp",
									"Downloaded bundle does not contain a valid dapp.",
									Some(&format!("{:?}", e))
								))
							},
							Ok(manifest) => {
								let address = redirection_address(self.using_dapps_domains, &manifest.id);
								FetchState::Done(manifest, Redirection::new(&address))
							},
						};
						// Remove temporary zip file
						let _ = fs::remove_file(path);
						(Some(state), Next::write())
					},
					Ok(Err(e)) => {
						warn!(target: "dapps", "Unable to fetch new dapp: {:?}", e);
						let error = ContentHandler::error(
							StatusCode::BadGateway,
							"Download Error",
							"There was an error when fetching the dapp.",
							Some(&format!("{:?}", e)),
						);
						(Some(FetchState::Error(error)), Next::write())
					},
					// wait some more
					_ => (None, Next::wait())
				}
			},
			FetchState::Error(ref mut handler) => (None, handler.on_request_readable(decoder)),
			_ => (None, Next::write()),
		};

		if let Some(status) = status {
			self.fetch_control.set_status(&status);
			self.status = status;
		}

		next
	}

	fn on_response(&mut self, res: &mut server::Response) -> Next {
		match self.status {
			FetchState::Done(_, ref mut handler) => handler.on_response(res),
			FetchState::Error(ref mut handler) => handler.on_response(res),
			_ => Next::end(),
		}
	}

	fn on_response_writable(&mut self, encoder: &mut Encoder<HttpStream>) -> Next {
		match self.status {
			FetchState::Done(_, ref mut handler) => handler.on_response_writable(encoder),
			FetchState::Error(ref mut handler) => handler.on_response_writable(encoder),
			_ => Next::end(),
		}
	}
}
