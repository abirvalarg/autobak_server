use std::collections::HashMap;
use anyhow::Result;
use async_std::sync::{Arc, Weak, Mutex};
use futures::StreamExt;
use sha3::{Sha3_256, Digest};
use sqlx::{MySqlPool, QueryBuilder, Row};

#[derive(Clone)]
pub struct UserPool {
	cache: Arc<Mutex<UserCache>>,
	db: MySqlPool
}

impl UserPool {
	pub fn new(db: &MySqlPool) -> Self {
		UserPool {
			cache: Arc::new(Mutex::new(UserCache {
				name_cache: HashMap::new(),
				id_cache: HashMap::new()
			})),
			db: db.clone()
		}
	}

	pub async fn get(&self, username: &str) -> Result<Option<Arc<User>>> {
		let mut cache = self.cache.lock().await;
		match cache.get(username) {
			Some(user) => Ok(Some(user)),
			None => match User::from_db_username(&self.db, username).await? {
				Some(user) => Ok(Some(cache.add(user))),
				None => Ok(None)
			}
		}
	}
}

struct UserCache {
	name_cache: HashMap<String, Weak<User>>,
	id_cache: HashMap<u64, Weak<User>>
}

impl UserCache {
	fn get(&mut self, username: &str) -> Option<Arc<User>> {
		match self.name_cache.get(username) {
			Some(user) =>  match user.upgrade() {
				Some(user) => Some(user),
				None => {
					self.name_cache.remove(username);
					None
				}
			},
			None => None
		}
	}

	fn get_by_id(&mut self, id: u64) -> Option<Arc<User>> {
		match self.id_cache.get(&id) {
			Some(user) =>  match user.upgrade() {
				Some(user) => Some(user),
				None => {
					self.id_cache.remove(&id);
					None
				}
			},
			None => None
		}
	}

	fn add(&mut self, user: User) -> Arc<User> {
		let user = Arc::new(user);
		self.name_cache.insert(user.username.clone(), Arc::downgrade(&user));
		self.id_cache.insert(user.id, Arc::downgrade(&user));
		user
	}
}

pub struct User {
	db: Option<MySqlPool>,
	id: u64,
	username: String,
	password_hash: String
}

impl User {
	async fn from_db_username(db_pool: &MySqlPool, username: &str) -> Result<Option<Self>> {
		let mut db = db_pool.acquire().await?;
		let mut query = QueryBuilder::new("SELECT id, password FROM user WHERE username=");
		query.push_bind(username);
		let query = query.build();
		let res = match query.fetch(&mut db).next().await {
			Some(res) => {
				let res = res?;
				Ok(Some(User {
					db: Some(db_pool.clone()),
					id: res.get(0),
					username: username.into(),
					password_hash: res.get(1)
				}))
			}
			None => Ok(None)
		};
		res
	}

	pub fn check_password(&self, password: &str) -> bool {
		match self.password_hash.split_once('.') {
			Some((salt, correct_hash)) => {
				let mut hasher = Sha3_256::new();
				let prep = format!("{salt}{password}");
				hasher.update(prep.as_bytes());
				let hash = hasher.finalize();
				format!("{hash:x}") == correct_hash
			}
			None => false
		}
	}
}
