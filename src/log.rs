use std::{
	fmt::{self, Display},
	sync::{
		Mutex,
		mpsc::{SyncSender, Receiver, sync_channel}
	},
	thread::{spawn, JoinHandle},
	io::Write
};
use chrono::Local;
use anyhow::Result;
use fs2::FileExt;
use crate::config::Config;

lazy_static::lazy_static! {
	static ref LOG_SINK: Mutex<Option<SyncSender<(LogLevel, String)>>> = Mutex::new(None);
}

pub fn log(level: LogLevel, msg: &str) {
	let mut sink = LOG_SINK.lock().unwrap();
	if let Some(ref mut sink) = *sink {
		sink.send((level, msg.into())).unwrap();
	}
}

#[macro_export]
macro_rules! debug {
	( $fmt:literal $(, $args:expr )* ) => {
		crate::log::log(crate::log::LogLevel::Debug, &format!($fmt $(, $args)* ))
	};
}

#[macro_export]
macro_rules! info {
	( $fmt:literal $(, $args:expr )* ) => {
		crate::log::log(crate::log::LogLevel::Info, &format!($fmt $(, $args)* ))
	};
}

#[macro_export]
macro_rules! warning {
	( $fmt:literal $(, $args:expr )* ) => {
		crate::log::log(crate::log::LogLevel::Warning, &format!($fmt $(, $args)* ))
	};
}

#[macro_export]
macro_rules! error {
	( $fmt:literal $(, $args:expr )* ) => {
		crate::log::log(crate::log::LogLevel::Error, &format!($fmt $(, $args)* ))
	};
}

#[macro_export]
macro_rules! critical {
	( $fmt:literal $(, $args:expr )* ) => {
		crate::log::log(crate::log::LogLevel::Critical, &format!($fmt $(, $args)* ))
	};
}

pub fn log_main<Output: Write>(mut output: Output, rx: Receiver<(LogLevel, String)>, file_level: LogLevel, term_level: Option<LogLevel>) {
	for (level, msg) in rx {
		let msg = format!("[{}] [{level}] {msg}\n", Local::now());
		if level >= file_level {
			output.write_all(msg.as_bytes()).unwrap();
		}
		if let Some(term_level) = term_level {
			if level >= term_level {
				print!("{msg}");
			}
		}
	}
}

pub fn start(cfg: &Config) -> Result<JoinHandle<()>> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(!cfg.overwrite_log)
		.truncate(cfg.overwrite_log)
        .write(true)
        .open(&cfg.log_path)?;
	file.try_lock_exclusive()?;
    let (tx, rx) = sync_channel(100);
	*LOG_SINK.lock().unwrap() = Some(tx);
	let log_level = cfg.log_level;
	let term_log_level = cfg.term_log_level;
	Ok(spawn(move || log_main(file, rx, log_level, term_log_level)))
}

pub fn stop() {
	LOG_SINK.lock().unwrap().take();
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum LogLevel {
	Debug,
	Info,
	Warning,
	Error,
	Critical
}

impl Into<&str> for LogLevel {
	fn into(self) -> &'static str {
		use LogLevel::*;
		match self {
			Debug => "debug",
			Info => "info",
			Warning => "Warning",
			Error => "Error",
			Critical => "CRITICAL"
		}
	}
}

impl Display for LogLevel {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let text: &str = self.clone().into();
		write!(f, "{text}")
	}
}

impl TryFrom<&str> for LogLevel {
	type Error = LevelParseError;

	fn try_from(value: &str) -> Result<Self, <Self as TryFrom<&str>>::Error> {
		use LogLevel::*;
		match value.to_lowercase().as_str() {
			"debug" => Ok(Debug),
			"info" => Ok(Info),
			"warning" => Ok(Warning),
			"error" => Ok(Error),
			"critical" => Ok(Critical),
			_ => Err(LevelParseError(value.into()))
		}
	}
}

#[derive(Debug)]
pub struct LevelParseError(String);

impl Display for LevelParseError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "\"{}\" is not a valid log level", self.0)
	}
}

impl std::error::Error for LevelParseError {}
