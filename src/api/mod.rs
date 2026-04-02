pub mod client;
pub mod types;

pub use client::ZoomClient;
pub use types::*;

use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    /// Bad credentials or forbidden (401/403).
    Auth(String),
    /// Resource not found (404).
    NotFound(String),
    /// Invalid user input or missing config.
    InvalidInput(String),
    /// HTTP 429 rate limit.
    RateLimit,
    /// Non-2xx response from the Zoom API.
    Api { status: u16, message: String },
    /// Network / TLS error.
    Http(reqwest::Error),
    /// Any other error.
    Other(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Auth(msg) => write!(
                f,
                "Authentication failed: {msg}\nCheck your credentials or run `zoom config show`."
            ),
            ApiError::NotFound(msg) => write!(f, "Not found: {msg}"),
            ApiError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            ApiError::RateLimit => write!(
                f,
                "Rate limited by Zoom (429). Please wait and try again.\nNote: meeting creation is capped at 100 requests/day per user."
            ),
            ApiError::Api { status, message } => write!(f, "API error {status}: {message}"),
            ApiError::Http(e) => write!(f, "HTTP error: {e}"),
            ApiError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ApiError::Http(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Http(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn auth_error_display_includes_guidance() {
        let err = ApiError::Auth("invalid_token".into());
        let msg = err.to_string();
        assert!(msg.contains("Authentication failed"));
        assert!(msg.contains("invalid_token"));
        assert!(msg.contains("credentials"), "should hint at how to fix");
        assert!(
            msg.contains("zoom config show"),
            "should name the command to inspect config"
        );
    }

    #[test]
    fn not_found_error_display_includes_message() {
        let err = ApiError::NotFound("meeting 123456789 not found".into());
        let msg = err.to_string();
        assert!(msg.contains("Not found"));
        assert!(msg.contains("123456789"));
    }

    #[test]
    fn invalid_input_error_display_includes_message() {
        let err = ApiError::InvalidInput("account_id is required".into());
        let msg = err.to_string();
        assert!(msg.contains("Invalid input"));
        assert!(msg.contains("account_id is required"));
    }

    #[test]
    fn rate_limit_error_mentions_daily_cap() {
        let err = ApiError::RateLimit;
        let msg = err.to_string();
        assert!(msg.to_lowercase().contains("rate limit") || msg.contains("Rate limit"));
        assert!(
            msg.contains("100"),
            "should mention the 100/day meeting cap"
        );
    }

    #[test]
    fn api_error_display_includes_status_and_message() {
        let err = ApiError::Api {
            status: 400,
            message: "Invalid parameter: duration".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("400"));
        assert!(msg.contains("Invalid parameter: duration"));
    }

    #[test]
    fn other_error_display_is_verbatim() {
        let err = ApiError::Other("unexpected failure".into());
        assert_eq!(err.to_string(), "unexpected failure");
    }

    #[test]
    fn http_error_source_is_underlying_reqwest_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let reqwest_err = rt.block_on(async {
            reqwest::Client::new()
                .get("http://127.0.0.1:1")
                .send()
                .await
                .unwrap_err()
        });
        let api_err = ApiError::Http(reqwest_err);
        assert!(api_err.source().is_some());
    }

    #[test]
    fn non_http_variants_have_no_source() {
        assert!(ApiError::Auth("x".into()).source().is_none());
        assert!(ApiError::NotFound("x".into()).source().is_none());
        assert!(ApiError::InvalidInput("x".into()).source().is_none());
        assert!(ApiError::RateLimit.source().is_none());
        assert!(ApiError::Other("x".into()).source().is_none());
    }
}
