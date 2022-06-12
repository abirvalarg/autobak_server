use std::net::Ipv4Addr;
use anyhow::Result;
use sqlx::{MySqlPool, query};
use super::user::User;

pub struct Audit(MySqlPool);

impl Audit {
	pub fn new(db: &MySqlPool) -> Self {
		Audit(db.clone())
	}

	pub async fn log(&self, user: Option<&User>, addr: Ipv4Addr, event: Event, success: bool, info: Option<&str>) -> Result<()> {
		let mut db = self.0.acquire().await?;
		let addr = addr.octets().iter().fold(0u32, |res, val| (res << 8) + *val as u32);
		let event: &str = event.into();
		query!("INSERT INTO audit (user, address, event, success, info)
			VALUES (?, ?, ?, ?, ?)",
			user.map(|u| u.id()),
			addr,
			event,
			if success { "Y" } else { "N" },
			info
		).execute(&mut db).await?;
		Ok(())
	}
}

pub enum Event {
	Auth,
	NewStash,
	DeleteStash,
	List,
	Download,
	Upload,
	DeleteFile
}

impl Into<&str> for Event {
	fn into(self) -> &'static str {
		use Event::*;
		match self {
			Auth => "AUTH",
			NewStash => "NEW_STASH",
			DeleteStash => "DELETE_STASH",
			List => "LIST",
			Download => "DOWNLOAD",
			Upload => "UPLOAD",
			DeleteFile => "DELETE_FILE"
		}
	}
}
