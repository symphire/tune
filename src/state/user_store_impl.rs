use std::collections::HashMap;
use crate::domain::UserId;
use crate::state::user_store::UserStore;

pub struct UserStoreImpl {
    store: HashMap<UserId, String>,
}

impl UserStoreImpl {
    pub fn new() -> UserStoreImpl {
        UserStoreImpl {
            store: HashMap::new(),
        }
    }
}

impl UserStore for UserStoreImpl {
    fn get_username(&self, user_id: UserId) -> Option<String> {
        self.store.get(&user_id).cloned()
    }

    fn update_user(&mut self, user_id: UserId, username: &str) {
        self.store.insert(user_id, username.to_owned());
    }
}