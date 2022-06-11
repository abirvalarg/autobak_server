use std::{
	fmt::{self, Display},
	net::IpAddr
};
use async_std::{
	sync::Arc,
	net::TcpStream,
	io::{ReadExt, WriteExt}
};
use openssl::ssl::Ssl;
use crate::{debug, error};
use state::Expectation;

pub mod acceptor;
mod stream;
mod state;

pub async fn handle_client(info: Arc<crate::ServerInfo>, client: TcpStream) {
	debug!("Handling connection from {}", client.peer_addr().unwrap());

	let func = async move {
		let addr = client.peer_addr()?;
		let addr = match addr.ip() {
			IpAddr::V4(ip) => ip,
			IpAddr::V6(_) => return Err(UnsupportenAddr.into())
		};
		let mut ssl = Ssl::new(info.ssl.context())?;
		ssl.set_accept_state();
		let mut stream = stream::Stream::new(&info.ssl, client).await?;
		let mut state = state::State::new(info.clone(), addr);

		let mut buffer = [0; 4096];
		let mut buf_len = 0;

		while !state.end() {
			let len = stream.read(&mut buffer[buf_len..]).await?;
			if len == 0 {
				break;
			}

			match state.expects() {
				Expectation::Line => {
					let mut nl_pos = None;
					for check_pos in buf_len..buf_len + len {
						if buffer[check_pos] == b'\n' {
							let start = match nl_pos {
								Some(nl_pos) => nl_pos + 1,
								None => 0
							};
							let res = state.next_piece(&buffer[start..check_pos]).await;
							match res {
								Ok(res) => stream.write_all(&(&res).into() as &Vec<_>).await?,
								Err(err) => {
									stream.write_all(b"err:server\n").await?;
									return Err(err);
								}
							}
							nl_pos = Some(check_pos);
						}
					}

					match nl_pos {
						Some(nl_pos) => {
							for pos in nl_pos + 1..buf_len + len {
								buffer[pos - nl_pos - 1] = buffer[pos];
							}
							buf_len = buf_len + len - nl_pos - 1;
						}
						None => buf_len += len
					}
				}
				Expectation::Nothing => break
			}
		}

		Ok(()) as anyhow::Result<()>
	};

	match func.await {
		Ok(_) => debug!("Stopping a task"),
		Err(err) => error!("Task ended with an error: {err}")
	}
}

#[derive(Debug)]
pub struct UnsupportenAddr;

impl Display for UnsupportenAddr {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "IPv6 is not supported yet")
	}
}

impl std::error::Error for UnsupportenAddr {}
