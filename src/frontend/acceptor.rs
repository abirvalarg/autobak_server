use std::{
	sync::Mutex,
	task::{Waker, Poll}, pin::Pin
};

use anyhow::Result;
use async_std::{
	sync::Arc,
	net::{TcpListener, TcpStream}, prelude::StreamExt
};
use futures::{select, future::FusedFuture, Future, FutureExt, pin_mut};

pub struct Acceptor {
	listener: TcpListener,
	stop: Stopper
}

impl Acceptor {
	pub fn new(listener: TcpListener) -> Result<Self> {
		let stop = Stopper::default();
		let ctrlc_stop = stop.clone();
		ctrlc::set_handler(move || ctrlc_stop.stop())?;
		Ok(Acceptor {
			listener, stop
		})
	}

	pub async fn accept(&self) -> Option<Result<TcpStream, std::io::Error>> {
		let mut incoming = self.listener.incoming();
		let fut = incoming.next().fuse();

		pin_mut!(fut);

		select! {
			stream = fut => stream,
			() = self.stop.stop_future() => None
		}
	}
}

#[derive(Clone)]
struct Stopper(Arc<Mutex<InnerStopper>>);

impl Default for Stopper {
	fn default() -> Self {
		Stopper(Arc::new(Mutex::new(InnerStopper {
			is_stopped: false,
			waker: None
		})))
	}
}

impl Stopper {
	fn stop_future(&self) -> StopperFuture {
		StopperFuture(Pin::new(self.0.as_ref()))
	}

	fn stop(&self) {
		let mut inner = self.0.lock().unwrap();
		inner.is_stopped = true;
		if let Some(waker) = inner.waker.take() {
			waker.wake();
		}
	}
}

struct InnerStopper {
	is_stopped: bool,
	waker: Option<Waker>
}

struct StopperFuture<'a>(Pin<&'a Mutex<InnerStopper>>);

impl<'a> Future for StopperFuture<'a> {
	type Output = ();

	fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
		let mut inner = self.0.lock().unwrap();
		if inner.is_stopped {
			Poll::Ready(())
		} else {
			inner.waker = Some(cx.waker().clone());
			Poll::Pending
		}
	}
}

impl FusedFuture for StopperFuture<'_> {
	fn is_terminated(&self) -> bool {
		false
	}
}
