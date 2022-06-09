use std::fmt::{self, Display};
use anyhow::Result;

pub struct Args {
	pub exec: String,
	pub config: Option<String>
}

impl Args {
	pub fn from_cmd() -> Result<Self> {
		use NextArg::*;

		let mut args = std::env::args();
		let res = Args {
			exec: args.next().unwrap(),
			config: None
		};

		let  (res, next) = args.try_fold((res, Flag), |(args, next), arg| {
			match next {
				Flag => match arg.as_str() {
					"-c" | "--cfg" => Ok((args, Config)),
					_ => Err(Error::UnknownFlag(arg))
				},
				Config => Ok((Args { config: Some(arg), ..args }, Flag))
			}
		})?;

		if next == Flag {
			Ok(res)
		} else {
			Err(anyhow::Error::new(Error::TokenExpected(next)))
		}
	}
}

#[derive(Debug, PartialEq)]
pub enum NextArg {
	Flag,
	Config
}

impl Into<&str> for &NextArg {
	fn into(self) -> &'static str {
		use NextArg::*;
		match self {
			Flag => "flag",
			Config => "path to config file"
		}
	}
}

#[derive(Debug)]
pub enum Error {
	UnknownFlag(String),
	TokenExpected(NextArg)
}

impl Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use Error::*;
		match self {
			UnknownFlag(flag) => write!(f, "unknown flag \"{flag}\""),
			TokenExpected(token) => {
				let token: &str = token.into();
				write!(f, "didn't find an expected token: {token}")
			}
		}
	}
}

impl std::error::Error for Error {}
