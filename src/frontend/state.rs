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
			Auth | Command => Line,
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
				Command => {
					let (cmd, args) = match line.split_once(' ') {
						Some(res) => (res.0.trim(), res.1.trim()),
						None => (line.trim(), "")
					};
					match cmd {
						"" => Ok(Response::None),
						"list" => self.list(args).await,
						"download" => self.download(args).await,
						_ => Ok(Response::NoCmd)
					}
				},
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
						self.state = ConnectState::Command;
						self.info.audit.log(self.user.as_deref(), self.addr, Event::Auth, true, None).await?;
						Response::Ok(ResponseContent::Empty)
					} else {
						self.state = ConnectState::End;
						self.info.audit.log(Some(&user), self.addr, Event::Auth, false, None).await?;
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

	async fn list(&self, args: &str) -> Result<Response> {
		let args: Vec<&str> = args.split(' ').collect();
		match args[0] {
			"stashes" => Ok(Response::Ok(ResponseContent::Lines(
				self.user.as_ref().unwrap().all_stashes().await?
			))),
			"files" => if args.len() == 2 {
				match self.user.as_ref().unwrap().get_stash(args[1]).await? {
					Some(stash) => {
						self.info.audit.log(self.user.as_deref(), self.addr, Event::List, true, Some(args[1])).await?;
						Ok(Response::Ok(ResponseContent::Lines(
							stash.into_iter().map(|s| format!("{} {}", s.0, s.1.update_time())).collect()
						)))
					}
					None => {
						self.info.audit.log(self.user.as_deref(), self.addr, Event::List, false, Some(args[1])).await?;
						Ok(Response::NoStash)
					}
				}
			} else {
				Ok(Response::BadArgs)
			}
			_ => Ok(Response::BadArgs)
		}
	}

	async fn download(&self, args: &str) -> Result<Response> {
		let res = match args.split_once(' ') {
			Some((stash, path)) => match self.user.as_ref().unwrap().get_stash(stash).await? {
				Some(stash) => match stash.get(path) {
					Some(file) => Ok(Response::Ok(ResponseContent::Binary(file.read()))),
					None => Ok(Response::NoFile)
				}
				None => Ok(Response::NoStash)
			}
			None => Ok(Response::BadArgs)
		};
		match res {
			Ok(ref data) => match data {
				Response::Ok(_) => self.info.audit.log(self.user.as_deref(), self.addr, Event::Download, true, Some(args)).await?,
				_ => self.info.audit.log(self.user.as_deref(), self.addr, Event::Download, false, Some(args)).await?
			}
			_ => ()
		};
		res
	}
}

#[derive(PartialEq)]
enum ConnectState {
	Auth,
	Command,
	End
}

pub enum Expectation {
	Line,
	Nothing
}

pub enum Response {
	None,
	Ok(ResponseContent),
	BadFormat,
	NoAuth,
	NoCmd,
	BadArgs,
	NoStash,
	NoFile
}

pub enum ResponseContent {
	Empty,
	Lines(Vec<String>),
	Binary(Vec<u8>)
}

impl Into<Vec<u8>> for &Response {
	fn into(self) -> Vec<u8> {
		use Response::*;
		use ResponseContent::*;
		match self {
			None => vec![],
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
				Binary(data) => {
					let mut res = Vec::from(format!("ok:b{}\n", data.len()).as_bytes());
					res.extend(data);
					res
				}
			},
			BadFormat => Vec::from(&b"err:format\n"[..]),
			NoAuth => Vec::from(&b"err:auth\n"[..]),
			NoCmd => Vec::from(&b"err:nocommand\n"[..]),
			BadArgs => Vec::from(&b"err:badargs\n"[..]),
			NoStash => Vec::from(&b"err:nostash\n"[..]),
			NoFile => Vec::from(&b"err:nofile\n"[..])
		}
	}
}
