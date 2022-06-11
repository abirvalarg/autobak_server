use std::net::Ipv4Addr;
use anyhow::Result;
use async_std::sync::Arc;
use crate::info::audit::Event;
#[allow(unused_imports)]
use crate::{debug, error};

#[allow(dead_code)]
pub struct State {
	info: Arc<crate::ServerInfo>,
	state: ConnectState,
	user: Option<Arc<crate::info::user::User>>,
	addr: Ipv4Addr
}

impl State {
	pub fn new(info: Arc<crate::ServerInfo>, addr: Ipv4Addr) -> Self {
		State {
			info,
			state: ConnectState::Auth,
			user: None,
			addr
		}
	}

	pub fn expects(&self) -> Expectation {
		use ConnectState::*;
		use Expectation::*;
		match self.state {
			Auth | Echo => Line,
			End => Nothing
		}
	}

	pub fn end(&self) -> bool {
		self.state == ConnectState::End
	}

	pub async fn next_piece(&mut self, buffer: &[u8]) -> Result<Response> {
		use ConnectState::*;
		match String::from_utf8(buffer.into()) {
			Ok(line) => match self.state {
				Auth => self.try_login(&line).await,
				Echo => Ok(Response::Ok(ResponseContent::Lines(vec![line]))),
				End => Ok(Response::BadFormat)
			},
			Err(err) => {
				error!("Recieved an invalid UTF8 string: {err}");
				self.state = ConnectState::End;
				Ok(Response::BadFormat)
			}
		}
	}

	async fn try_login(&mut self, request: &str) -> Result<Response> {
		Ok(match request.split_once(' ') {
			Some((username, password)) => {
				match self.info.users.get(username).await? {
					Some(user) => if user.check_password(password) {
						self.user = Some(user);
						self.state = ConnectState::Echo;
						self.info.audit.log(self.user.as_deref(), self.addr, Event::Auth, true).await?;
						Response::Ok(ResponseContent::Empty)
					} else {
						self.state = ConnectState::End;
						self.info.audit.log(Some(&user), self.addr, Event::Auth, false).await?;
						Response::NoAuth
					}
					None => {
						self.state = ConnectState::End;
						Response::NoAuth
					}
				}
			}
			None => {
				self.state = ConnectState::End;
				Response::BadFormat
			}
		})
	}
}

#[derive(PartialEq)]
enum ConnectState {
	Auth,
	Echo,
	End
}

pub enum Expectation {
	Line,
	Nothing
}

pub enum Response {
	Ok(ResponseContent),
	BadFormat,
	NoAuth
}

pub enum ResponseContent {
	Empty,
	Lines(Vec<String>)
}

impl Into<Vec<u8>> for &Response {
	fn into(self) -> Vec<u8> {
		use Response::*;
		use ResponseContent::*;
		match self {
			Ok(content) => match content {
				Empty => Vec::from(&b"ok:0\n"[..]),
				Lines(lines) => {
					let fake_lines = lines.iter().fold(
						0,
						|count, line| count + line.chars().fold(
							0,
							|count, ch| count + if ch == '\n' { 1 } else { 0 }
						)
					);

					let res = format!("ok:l{}\n", lines.len() + fake_lines);
					let res = lines.iter().fold(res, |res, line| res + line + "\n");
					Vec::from(res.as_bytes())
				}
			},
			BadFormat => Vec::from(&b"err:format\n"[..]),
			NoAuth => Vec::from(&b"err:auth\n"[..])
		}
	}
}
