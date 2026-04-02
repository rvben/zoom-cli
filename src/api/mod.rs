pub mod client;
pub mod types;
pub use client::ZoomClient;

use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Auth(String),
    NotFound(String),
    InvalidInput(String),
    RateLimit,
    Api { status: u16, message: String },
    Http(reqwest::Error),
    Other(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "api error")
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self { ApiError::Http(e) }
}
