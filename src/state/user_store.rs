use crate::domain::UserId;

pub trait UserStore {
    fn get_username(&self, user_id: UserId) -> Option<String>;
    fn update_user(&mut self, user_id: UserId, username: &str);
}