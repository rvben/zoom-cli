use crate::api::ApiError;

pub struct Config {
    pub account_id: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Config {
    pub fn load(_profile: Option<String>) -> Result<Self, ApiError> {
        todo!()
    }
}
