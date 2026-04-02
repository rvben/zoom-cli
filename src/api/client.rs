pub struct ZoomClient {
    pub account_id: String,
    pub client_id: String,
    pub client_secret: String,
}

impl ZoomClient {
    pub fn new(account_id: String, client_id: String, client_secret: String) -> Self {
        Self { account_id, client_id, client_secret }
    }
}
