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
    /// Destructive operation that requires explicit confirmation.
    ConfirmationRequired(String),
    /// Resource already exists or state conflict (HTTP 409).
    Conflict(String),
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
            ApiError::ConfirmationRequired(msg) => write!(f, "{msg}"),
            ApiError::Conflict(msg) => write!(f, "Conflict: {msg}"),
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

impl ApiError {
    pub fn kind(&self) -> &'static str {
        match self {
            ApiError::Auth(_) => "auth_error",
            ApiError::NotFound(_) => "not_found",
            ApiError::InvalidInput(_) => "invalid_input",
            ApiError::ConfirmationRequired(_) => "confirmation_required",
            ApiError::Conflict(_) => "conflict",
            ApiError::RateLimit => "rate_limit",
            ApiError::Api { .. } => "api_error",
            ApiError::Http(_) => "http_error",
            ApiError::Other(_) => "error",
        }
    }

    pub fn to_structured_json(&self) -> String {
        let hint: Option<&str> = match self {
            ApiError::Auth(_) => Some("Run 'zoom init' to set up credentials."),
            ApiError::RateLimit => Some("Wait and retry."),
            ApiError::NotFound(_) => Some("Check the ID and try again."),
            ApiError::ConfirmationRequired(_) => Some("Pass --yes to confirm."),
            ApiError::Conflict(_) => {
                Some("The resource already exists or is in a conflicting state.")
            }
            _ => None,
        };
        let retryable = matches!(self, ApiError::RateLimit | ApiError::Http(_));
        let message = self.to_string();
        let mut error_obj = serde_json::json!({
            "kind": self.kind(),
            "message": message,
            "retryable": retryable
        });
        if let Some(h) = hint {
            error_obj["hint"] = serde_json::Value::String(h.to_string());
        }
        serde_json::json!({"error": error_obj}).to_string()
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
    fn api_error_scope_message_is_actionable() {
        // Simulates what parse_zoom_error produces for a code-4711 response.
        let err = ApiError::Api {
            status: 400,
            message: "Missing required OAuth scope: report:read:user:admin\nAdd this scope to your Zoom Server-to-Server OAuth app, then run `zoom init` to update credentials.".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("report:read:user:admin"));
        assert!(msg.contains("zoom init"), "must tell user how to fix it");
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
        assert!(
            ApiError::ConfirmationRequired("x".into())
                .source()
                .is_none()
        );
        assert!(ApiError::Conflict("x".into()).source().is_none());
        assert!(ApiError::RateLimit.source().is_none());
        assert!(ApiError::Other("x".into()).source().is_none());
    }

    #[test]
    fn kind_returns_correct_strings() {
        assert_eq!(ApiError::Auth("x".into()).kind(), "auth_error");
        assert_eq!(ApiError::NotFound("x".into()).kind(), "not_found");
        assert_eq!(ApiError::InvalidInput("x".into()).kind(), "invalid_input");
        assert_eq!(
            ApiError::ConfirmationRequired("x".into()).kind(),
            "confirmation_required"
        );
        assert_eq!(ApiError::Conflict("x".into()).kind(), "conflict");
        assert_eq!(ApiError::RateLimit.kind(), "rate_limit");
        assert_eq!(
            ApiError::Api {
                status: 400,
                message: "x".into()
            }
            .kind(),
            "api_error"
        );
        assert_eq!(ApiError::Other("x".into()).kind(), "error");
    }

    #[test]
    fn to_structured_json_is_valid_json_with_error_kind() {
        let json_str = ApiError::Auth("bad".into()).to_structured_json();
        let val: serde_json::Value = serde_json::from_str(&json_str).expect("valid JSON");
        assert_eq!(val["error"]["kind"], "auth_error");
        assert_eq!(val["error"]["retryable"], false);
        assert!(val["error"]["hint"].is_string());

        let json_str2 = ApiError::RateLimit.to_structured_json();
        let val2: serde_json::Value = serde_json::from_str(&json_str2).expect("valid JSON");
        assert_eq!(val2["error"]["kind"], "rate_limit");
        assert_eq!(val2["error"]["retryable"], true);
    }
}
