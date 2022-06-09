use std::{
	sync::{
		Arc,
		Mutex,
		mpsc::{Sender, Receiver, channel}
	},
	thread::{JoinHandle, spawn}
};
use mio::{Poll, Waker, Token};
use anyhow::Result;

const STOP: Token = Token(0);

lazy_static::lazy_static! {
	static ref POLL_SINK: Arc<Mutex<Option<Sender<Request>>>> = Arc::new(Mutex::new(None));
}

struct Request {}

fn poll_main(mut poll: Poll, rx: Receiver<Request>) {}

pub fn start() -> Result<(JoinHandle<()>, Waker)> {
	let poll = Poll::new()?;
	let waker = Waker::new(poll.registry(), STOP)?;
	let (tx, rx) = channel();
	*POLL_SINK.lock().unwrap() = Some(tx);
	let handle = spawn(move || poll_main(poll, rx));
	Ok((handle, waker))
}
