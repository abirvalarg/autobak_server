use std::{
	io::{self, Read, Write, ErrorKind},
	task::{Poll, Waker, Context},
	pin::Pin
};
use async_std::{
	net::TcpStream,
	io::{
		Read as AsyncRead,
		Write as AsyncWrite
	}
};
use openssl::ssl::{SslStream, SslAcceptor};

pub mod handshake;

pub struct Stream {
	inner: SslStream<StreamWrap>
}

impl Stream {
	pub fn new(ssl: &SslAcceptor, stream: TcpStream) -> handshake::HandshakeFuture {
		handshake::HandshakeFuture::new(ssl, StreamWrap::new(stream))
	}

	fn wrap(inner: SslStream<StreamWrap>) -> Self {
		Stream {
			inner
		}
	}
}

impl AsyncRead for Stream {
	fn poll_read(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
		self.inner.get_mut().waker = Some(cx.waker().clone());
		let res = match self.inner.read(buf) {
			Ok(len) => Poll::Ready(Ok(len)),
			Err(ref err) if err.kind() == ErrorKind::WouldBlock => Poll::Pending,
			Err(err) => Poll::Ready(Err(err))
		};
		self.inner.get_mut().waker = None;
		res
	}
}

impl AsyncWrite for Stream {
    fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
			self.inner.get_mut().waker = Some(cx.waker().clone());
			let res = match self.inner.write(buf) {
				Ok(len) => Poll::Ready(Ok(len)),
				Err(ref err) if err.kind() == ErrorKind::WouldBlock => Poll::Pending,
				Err(err) => Poll::Ready(Err(err))
			};
			self.inner.get_mut().waker = None;
			res
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
		self.inner.get_mut().waker = Some(cx.waker().clone());
		let res = match self.inner.flush() {
			Ok(len) => Poll::Ready(Ok(len)),
			Err(ref err) if err.kind() == ErrorKind::WouldBlock => Poll::Pending,
			Err(err) => Poll::Ready(Err(err))
		};
		self.inner.get_mut().waker = None;
		res
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
		Poll::Ready(Ok(()))
    }
}

pub struct StreamWrap {
	stream: TcpStream,
	waker: Option<Waker>
}

impl StreamWrap {
	fn new(stream: TcpStream) -> Self {
		StreamWrap {
			stream,
			waker: None
		}
	}
}

impl Read for StreamWrap {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		match AsyncRead::poll_read(Pin::new(&mut self.stream), &mut Context::from_waker(self.waker.as_ref().unwrap()), buf) {
			Poll::Ready(res) => res,
			Poll::Pending => io::Result::Err(io::Error::from(io::ErrorKind::WouldBlock))
		}
	}
}

impl Write for StreamWrap {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match AsyncWrite::poll_write(Pin::new(&mut self.stream), &mut Context::from_waker(self.waker.as_ref().unwrap()), buf) {
			Poll::Ready(res) => res,
			Poll::Pending => io::Result::Err(io::Error::from(io::ErrorKind::WouldBlock))
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		match AsyncWrite::poll_flush(Pin::new(&mut self.stream), &mut Context::from_waker(self.waker.as_ref().unwrap())) {
			Poll::Ready(res) => res,
			Poll::Pending => io::Result::Err(io::Error::from(io::ErrorKind::WouldBlock))
		}
	}
}
