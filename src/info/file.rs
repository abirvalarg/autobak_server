use crate::{
	config::Config,
	warning
};

pub struct File {
	id: u64,
	update_time: u64,
	path: String
}

impl File {
	pub fn new(id: u64, update_time: u64) -> Option<Self> {
		let path = format!("{}/{id}", Config::get().storage_path);
		match std::fs::metadata(&path) {
			Ok(meta) => if meta.is_file() {
				Some(File {
					id, update_time, path,
				})
			} else {
				warning!("File <{path}> doesn't seem to be a file");
				None
			}
			Err(err) => {
				warning!("Can't get access to stored file <{path}>: {err}");
				None
			}
		}
	}

	pub fn id(&self) -> u64 {
		self.id
	}

	pub fn update_time(&self) -> u64 {
		self.update_time
	}

	pub fn read(&self) -> Vec<u8> {
		std::fs::read(&self.path).unwrap()
	}
}
