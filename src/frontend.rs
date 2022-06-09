use async_std::{
	sync::Arc,
	net::TcpStream,
	io::ReadExt
};
use openssl::ssl::Ssl;
use crate::{debug, error};

pub mod acceptor;
mod stream;
mod state;

pub async fn handle_client(info: Arc<crate::ServerInfo>, client: TcpStream) {
	debug!("Handling connection from {}", client.peer_addr().unwrap());

	let func = async move {
		let mut ssl = Ssl::new(info.ssl.context())?;
		ssl.set_accept_state();
		let mut stream = stream::Stream::new(&info.ssl, client).await?;
		let mut state = state::State::default();

		let mut buffer = [0; 4096];
		let mut buf_len = 0;

		while !state.end() {
			let len = stream.read(&mut buffer[buf_len..]).await?;
			if len == 0 {
				break;
			}

			if state.expects_line() {
				let mut nl_pos = None;
				for check_pos in buf_len..buf_len + len {
					if buffer[check_pos] == b'\n' {
						let start = match nl_pos {
							Some(nl_pos) => nl_pos + 1,
							None => 0
						};
						state.next_piece(&buffer[start..check_pos]).await;
						nl_pos = Some(check_pos);
					}
				}

				match nl_pos {
					Some(nl_pos) => {
						for pos in nl_pos..buf_len + len {
							buffer[pos - nl_pos] = buffer[pos];
						}
						buf_len = buf_len + len - nl_pos;
					}
					None => buf_len += len
				}
			}
		}

		Ok(()) as anyhow::Result<()>
	};

	match func.await {
		Ok(_) => debug!("Stopping a task"),
		Err(err) => error!("Task ended with an error: {err}")
	}
}
