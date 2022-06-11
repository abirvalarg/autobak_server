use std::{
	collections::HashMap,
	fmt::{self, Display},
	net::{SocketAddr, IpAddr, Ipv4Addr}
};
use anyhow::Result;
use crate::log::LogLevel;

#[derive(Debug)]
pub struct Config {
	pub log_path: String,
	pub log_level: LogLevel,
	pub term_log_level: Option<LogLevel>,
	pub overwrite_log: bool,
	pub host: SocketAddr,
	pub certificate: String,
	pub key: String,
	pub db_host: String,
	pub db_port: u16,
	pub db_name: String,
	pub db_user: String,
	pub db_password: String,
	pub db_ssl: bool
}

impl Default for Config {
	fn default() -> Self {
		Config {
			log_path: "server.log".into(),
			log_level: LogLevel::Info,
			term_log_level: None,
			overwrite_log: false,
			host: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 46278),
			certificate: "cert.crt".into(),
			key: "cert.key".into(),
			db_host: "".into(),
			db_port: 3306,
			db_name: "".into(),
			db_user: "".into(),
			db_password: "".into(),
			db_ssl: false
		}
	}
}

impl Config {
	pub fn load(path: &str) -> Result<Self> {
		let raw = raw_config(path)?;
		raw.iter().try_fold(Config::default(), |cfg, (opt, val)| {
			match opt.to_lowercase().as_str() {
				"logpath" => Ok(Config { log_path: val.clone(), ..cfg }),
				"loglevel" => Ok(Config { log_level: val.as_str().try_into()?, ..cfg }),
				"termloglevel" => Ok(Config { term_log_level: Some(val.as_str().try_into()?), ..cfg }),
				"overwritelog" => Ok(Config { overwrite_log: val.parse()?, ..cfg }),
				"host" => Ok(Config { host: val.parse()?, ..cfg }),
				"cert" | "certificate" => Ok(Config { certificate: val.clone(), ..cfg }),
				"key" => Ok(Config { key: val.clone(), ..cfg }),
				"dbhost" => Ok(Config { db_host: val.clone(), ..cfg }),
				"dbport" => Ok(Config { db_port: val.parse()?, ..cfg }),
				"dbname" => Ok(Config { db_name: val.clone(), ..cfg }),
				"dbuser" => Ok(Config { db_user: val.clone(), ..cfg }),
				"dbpassword" => Ok(Config { db_password: val.clone(), ..cfg }),
				"dbssl" => Ok(Config { db_ssl: val.parse()?, ..cfg }),
				_ => Err(anyhow::Error::from(Error::UnknownOption(opt.clone())))
			}
		})
	}
}

fn raw_config(path: &str) -> Result<HashMap<String, String>> {
	let content = std::fs::read_to_string(path)?;
	let mut res = HashMap::new();
	for line in content.lines() {
		let line: &str = {
			if let Some((line, _)) = line.split_once('#') {
				line
			} else {
				line
			}
		}.trim();
		if line != "" {
			match line.split_once(' ') {
				Some((opt, val)) => res.insert(opt.trim().into(), val.trim().into()),
				None => res.insert(line.into(), "".into())
			};
		}
	}
	Ok(res)
}

#[derive(Debug)]
pub enum Error {
	UnknownOption(String)
}

impl Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use Error::*;
		match self {
			UnknownOption(opt) => write!(f, "unknown option in config: {opt}")
		}
	}
}

impl std::error::Error for Error {}
