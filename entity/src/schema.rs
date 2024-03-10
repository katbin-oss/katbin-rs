use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginPost {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
}
