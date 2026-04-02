use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::ApiError;
use super::types::*;

const ZOOM_API_BASE: &str = "https://api.zoom.us/v2";
const ZOOM_OAUTH_BASE: &str = "https://zoom.us";

pub struct ZoomClient {
    http: reqwest::Client,
    base_url: String,
    oauth_base_url: String,
    account_id: String,
    client_id: String,
    client_secret: String,
    token: Option<String>,
}

impl ZoomClient {
    pub fn new(account_id: String, client_id: String, client_secret: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            http,
            base_url: ZOOM_API_BASE.to_owned(),
            oauth_base_url: ZOOM_OAUTH_BASE.to_owned(),
            account_id,
            client_id,
            client_secret,
            token: None,
        }
    }

    /// For tests: skip OAuth flow, use a pre-provided token and a mock base URL.
    #[cfg(test)]
    pub fn new_for_test(base_url: String, oauth_base_url: String, token: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("failed to build HTTP client");
        Self {
            http,
            base_url,
            oauth_base_url,
            account_id: "test-account".into(),
            client_id: "test-client".into(),
            client_secret: "test-secret".into(),
            token: Some(token),
        }
    }

    async fn ensure_token(&mut self) -> Result<&str, ApiError> {
        if self.token.is_none() {
            let token = self.fetch_token().await?;
            self.token = Some(token);
        }
        Ok(self.token.as_deref().unwrap())
    }

    async fn fetch_token(&self) -> Result<String, ApiError> {
        let creds = BASE64.encode(format!("{}:{}", self.client_id, self.client_secret));
        let url = format!(
            "{}/oauth/token?grant_type=account_credentials&account_id={}",
            self.oauth_base_url, self.account_id
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Basic {creds}"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ApiError::Auth(
                "Failed to obtain access token. Check account_id, client_id, client_secret.".into(),
            ));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let token_resp: TokenResponse = resp.json().await?;
        Ok(token_resp.access_token)
    }

    async fn get<T: DeserializeOwned>(&mut self, path: &str) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let token = self.ensure_token().await?.to_owned();
        let resp = self.http.get(&url).bearer_auth(&token).send().await?;
        self.handle_response(resp).await
    }

    async fn get_with_query<T: DeserializeOwned>(
        &mut self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let token = self.ensure_token().await?.to_owned();
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .query(params)
            .send()
            .await?;
        self.handle_response(resp).await
    }

    async fn post<T: DeserializeOwned, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let token = self.ensure_token().await?.to_owned();
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(body)
            .send()
            .await?;
        self.handle_response(resp).await
    }

    async fn patch<B: Serialize>(&mut self, path: &str, body: &B) -> Result<(), ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let token = self.ensure_token().await?.to_owned();
        let resp = self
            .http
            .patch(&url)
            .bearer_auth(&token)
            .json(body)
            .send()
            .await?;
        self.handle_empty_response(resp).await
    }

    async fn delete(&mut self, path: &str) -> Result<(), ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let token = self.ensure_token().await?.to_owned();
        let resp = self.http.delete(&url).bearer_auth(&token).send().await?;
        self.handle_empty_response(resp).await
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, ApiError> {
        let status = resp.status();
        match status.as_u16() {
            200..=299 => Ok(resp.json::<T>().await?),
            401 | 403 => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::Auth(body))
            }
            404 => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::NotFound(body))
            }
            429 => Err(ApiError::RateLimit),
            _ => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::Api {
                    status: status.as_u16(),
                    message: body,
                })
            }
        }
    }

    async fn handle_empty_response(&self, resp: reqwest::Response) -> Result<(), ApiError> {
        let status = resp.status();
        match status.as_u16() {
            200..=299 => Ok(()),
            401 | 403 => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::Auth(body))
            }
            404 => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::NotFound(body))
            }
            429 => Err(ApiError::RateLimit),
            _ => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::Api {
                    status: status.as_u16(),
                    message: body,
                })
            }
        }
    }

    // ── Meetings ──────────────────────────────────────────────────────────────

    pub async fn list_meetings(
        &mut self,
        user_id: &str,
        meeting_type: Option<&str>,
    ) -> Result<MeetingList, ApiError> {
        let path = format!("/users/{user_id}/meetings");
        let mut params: Vec<(&str, &str)> = vec![("page_size", "100")];
        let mt_owned;
        if let Some(mt) = meeting_type {
            mt_owned = mt.to_owned();
            params.push(("type", mt_owned.as_str()));
        }
        self.get_with_query(&path, &params).await
    }

    pub async fn get_meeting(&mut self, meeting_id: u64) -> Result<Meeting, ApiError> {
        self.get(&format!("/meetings/{meeting_id}")).await
    }

    pub async fn create_meeting(
        &mut self,
        user_id: &str,
        req: CreateMeetingRequest,
    ) -> Result<Meeting, ApiError> {
        self.post(&format!("/users/{user_id}/meetings"), &req).await
    }

    pub async fn update_meeting(
        &mut self,
        meeting_id: u64,
        req: UpdateMeetingRequest,
    ) -> Result<(), ApiError> {
        self.patch(&format!("/meetings/{meeting_id}"), &req).await
    }

    pub async fn delete_meeting(&mut self, meeting_id: u64) -> Result<(), ApiError> {
        self.delete(&format!("/meetings/{meeting_id}")).await
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    pub async fn list_users(&mut self, status: Option<&str>) -> Result<UserList, ApiError> {
        let mut params: Vec<(&str, &str)> = vec![("page_size", "300")];
        let st_owned;
        if let Some(st) = status {
            st_owned = st.to_owned();
            params.push(("status", st_owned.as_str()));
        }
        self.get_with_query("/users", &params).await
    }

    pub async fn get_user(&mut self, user_id: &str) -> Result<User, ApiError> {
        self.get(&format!("/users/{user_id}")).await
    }

    // ── Recordings ───────────────────────────────────────────────────────────

    pub async fn list_recordings(
        &mut self,
        user_id: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<RecordingList, ApiError> {
        let path = format!("/users/{user_id}/recordings");
        let mut params: Vec<(&str, &str)> = vec![("page_size", "30")];
        let from_owned;
        let to_owned;
        if let Some(f) = from {
            from_owned = f.to_owned();
            params.push(("from", from_owned.as_str()));
        }
        if let Some(t) = to {
            to_owned = t.to_owned();
            params.push(("to", to_owned.as_str()));
        }
        self.get_with_query(&path, &params).await
    }

    /// Control recording state for a live meeting (start/stop/pause/resume).
    pub async fn control_recording(
        &mut self,
        meeting_id: u64,
        action: &str,
    ) -> Result<(), ApiError> {
        let req = RecordingControlRequest {
            action: action.to_owned(),
        };
        self.patch(&format!("/live_meetings/{meeting_id}/recordings"), &req)
            .await
    }

    pub async fn get_recording(&mut self, meeting_id: &str) -> Result<CloudRecording, ApiError> {
        // Zoom meeting UUIDs can contain '/' (base64 chars); encode it so the
        // character is not interpreted as a path separator.
        let encoded_id = meeting_id.replace('/', "%2F");
        self.get(&format!("/meetings/{encoded_id}/recordings"))
            .await
    }

    /// Download a recording file to disk. Handles Zoom's auth-required downloads.
    pub async fn download_recording_file(
        &mut self,
        download_url: &str,
        dest_path: &std::path::Path,
    ) -> Result<u64, ApiError> {
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        let token = self.ensure_token().await?.to_owned();
        let resp = self
            .http
            .get(download_url)
            .bearer_auth(&token)
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ApiError::Auth(
                "Not authorized to download this recording".into(),
            ));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let mut file = tokio::fs::File::create(dest_path).await.map_err(|e| {
            ApiError::Other(format!("Cannot create file {}: {e}", dest_path.display()))
        })?;

        let mut bytes_written: u64 = 0;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)
                .await
                .map_err(|e| ApiError::Other(format!("Write error: {e}")))?;
            bytes_written += chunk.len() as u64;
        }
        file.flush()
            .await
            .map_err(|e| ApiError::Other(format!("Flush error: {e}")))?;

        Ok(bytes_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_client(server: &MockServer) -> ZoomClient {
        ZoomClient::new_for_test(
            format!("{}/v2", server.uri()),
            server.uri(),
            "test-token".into(),
        )
    }

    #[tokio::test]
    async fn fetch_token_returns_access_token_on_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "eyJhbGciOiJSUzI1NiJ9.test",
                "token_type": "bearer",
                "expires_in": 3599
            })))
            .mount(&server)
            .await;

        let client = ZoomClient {
            http: reqwest::Client::new(),
            base_url: format!("{}/v2", server.uri()),
            oauth_base_url: server.uri(),
            account_id: "acct123".into(),
            client_id: "cid".into(),
            client_secret: "csec".into(),
            token: None,
        };

        let token = client.fetch_token().await.unwrap();
        assert_eq!(token, "eyJhbGciOiJSUzI1NiJ9.test");
    }

    #[tokio::test]
    async fn fetch_token_returns_auth_error_on_401() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = ZoomClient {
            http: reqwest::Client::new(),
            base_url: format!("{}/v2", server.uri()),
            oauth_base_url: server.uri(),
            account_id: "acct".into(),
            client_id: "cid".into(),
            client_secret: "csec".into(),
            token: None,
        };

        let err = client.fetch_token().await.unwrap_err();
        assert!(matches!(err, ApiError::Auth(_)));
    }

    #[tokio::test]
    async fn list_meetings_returns_meeting_list() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [
                    {"id": 111111111, "topic": "Standup", "duration": 15}
                ],
                "total_records": 1,
                "page_size": 100
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_meetings("me", None).await.unwrap();
        assert_eq!(list.meetings.len(), 1);
        assert_eq!(list.meetings[0].topic, "Standup");
    }

    #[tokio::test]
    async fn get_meeting_returns_404_as_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/meetings/999999999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Meeting not found"))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let err = client.get_meeting(999999999).await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn delete_meeting_returns_ok_on_204() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/v2/meetings/123456789"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        client.delete_meeting(123456789).await.unwrap();
    }

    #[tokio::test]
    async fn list_meetings_with_type_filter_sends_query_param() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .and(query_param("type", "scheduled"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [],
                "total_records": 0
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_meetings("me", Some("scheduled")).await.unwrap();
        assert_eq!(list.meetings.len(), 0);
    }

    #[tokio::test]
    async fn rate_limit_response_returns_rate_limit_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let err = client.list_meetings("me", None).await.unwrap_err();
        assert!(matches!(err, ApiError::RateLimit));
    }

    #[tokio::test]
    async fn list_users_sends_correct_request() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "users": [
                    {
                        "id": "user-123",
                        "email": "alice@example.com",
                        "display_name": "Alice"
                    }
                ],
                "total_records": 1
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_users(None).await.unwrap();
        assert_eq!(list.users.len(), 1);
        assert_eq!(list.users[0].email, "alice@example.com");
    }
}
