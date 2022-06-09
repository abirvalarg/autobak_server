use crate::{debug, error};

pub struct State {
	state: ConnectState
}

impl State {
	pub fn expects_line(&self) -> bool {
		use ConnectState::*;
		match self.state {
			Auth => true,
			End => false
		}
	}

	pub fn end(&self) -> bool {
		self.state == ConnectState::End
	}

	pub async fn next_piece(&mut self, buffer: &[u8]) {
		match String::from_utf8(buffer.into()) {
			Ok(line) => debug!("\"{line}\""),
			Err(err) => {
				error!("Recieved an invalid UTF8 string: {err}");
				self.state = ConnectState::End;
			}
		}
	}
}

impl Default for State {
	fn default() -> Self {
		State {
			state: ConnectState::Auth
		}
	}
}

#[derive(PartialEq)]
enum ConnectState {
	Auth,
	End
}
