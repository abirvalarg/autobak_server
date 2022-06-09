use std::{
	fmt::{self, Display},
	task::Poll
};
use anyhow::Result;
use openssl::{ssl::{SslAcceptor, MidHandshakeSslStream, HandshakeError, SslStream}, error::ErrorStack};
use futures::Future;
use super::{Stream, StreamWrap};

pub struct HandshakeFuture {
	state: Option<State>
}

impl HandshakeFuture {
	pub(super) fn new(ssl: &SslAcceptor, stream: StreamWrap) -> Self {
		HandshakeFuture {
			state: Some(State::DidNotBegin {
				ssl: ssl.clone(),
				stream
			})
		}
	}

	fn process_handshake_res(&mut self, res: Result<SslStream<StreamWrap>, HandshakeError<StreamWrap>>) -> Poll<Result<Stream, Error>> {
		match res {
			Ok(mut stream) => {
				stream.get_mut().waker = None;
				Poll::Ready(Ok(Stream::wrap(stream)))
			}
			Err(err) => match err {
				HandshakeError::Failure(_) => Poll::Ready(Err(Error::Handshake)),
				HandshakeError::SetupFailure(err) => Poll::Ready(Err(Error::Setup(err))),
				HandshakeError::WouldBlock(mut stream) => {
					stream.get_mut().waker = None;
					self.state = Some(State::MidWay(stream));
					Poll::Pending
				}
			}
		}
	}
}

impl Future for HandshakeFuture {
	type Output = Result<Stream, Error>;

	fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
		use State::*;

		match self.state.take().unwrap() {
			DidNotBegin { ssl, mut stream } => {
				stream.waker = Some(cx.waker().clone());
				let res = self.process_handshake_res(ssl.accept(stream));
				res
			}
			MidWay(mut stream) => {
				stream.get_mut().waker = Some(cx.waker().clone());
				let res = self.process_handshake_res(stream.handshake());
				res
			}
		}
	}
}

enum State {
	DidNotBegin {
		ssl: SslAcceptor,
		stream: StreamWrap
	},
	MidWay(MidHandshakeSslStream<StreamWrap>)
}

#[derive(Debug)]
pub enum Error {
	Setup(ErrorStack),
	Handshake
}

impl Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use Error::*;
		match self {
			Setup(stack) => write!(f, "Bad ssl setup: {stack}"),
			Handshake => write!(f, "Handshake failed")
		}
	}
}

impl std::error::Error for Error {}
