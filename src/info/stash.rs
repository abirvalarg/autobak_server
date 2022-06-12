use std::collections::HashMap;
use anyhow::Result;
use sqlx::{MySqlPool, query};
use super::file::File;

pub struct Stash {
	files: HashMap<String, (u64, u64)>
}

impl Stash {
	pub async fn new(db: &MySqlPool, id: u64) -> Result<Self> {
		let mut db = db.acquire().await?;
		let query = query!(
			"SELECT id, name, update_time FROM file WHERE stash=?",
			id
		);
		let mut files = HashMap::new();
		for res in query.fetch_all(&mut db).await? {
			files.insert(res.name, (res.id, res.update_time));
		}
		Ok(Stash { files })
	}
}

impl<'a> IntoIterator for &'a Stash {
	type Item = (&'a str, File);
	type IntoIter = Files<'a>;

	fn into_iter(self) -> Self::IntoIter {
		Files(self.files.iter())
	}
}

pub struct Files<'a>(<&'a HashMap<String, (u64, u64)> as IntoIterator>::IntoIter);

impl<'a> Iterator for Files<'a> {
    type Item = (&'a str, File);

    fn next(&mut self) -> Option<Self::Item> {
		match self.0.next() {
			Some((name, (id, upd))) => match File::new(*id, *upd) {
				Some(file) => Some((name, file)),
				None => self.next()
			}
			None => None
		}
    }
}
