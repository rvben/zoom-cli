use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::ApiError;
use super::types::{self, *};

const ZOOM_API_BASE: &str = "https://api.zoom.us/v2";
const ZOOM_OAUTH_BASE: &str = "https://zoom.us";

/// Maximum number of attempts before giving up on a rate-limited request.
/// 4 attempts = initial + 3 retries.
const MAX_RETRY_ATTEMPTS: u32 = 4;

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

    /// Send a request with automatic token refresh on 401 (exactly one refresh).
    ///
    /// This is the inner layer: it handles expired tokens but not rate limiting.
    async fn send_once(
        &mut self,
        build: &impl Fn(&reqwest::Client, &str) -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, ApiError> {
        let token = self.ensure_token().await?.to_owned();
        let resp = build(&self.http, &token).send().await?;
        if resp.status().as_u16() == 401 {
            // Token may have expired — discard it, fetch a fresh one, retry once.
            self.token = None;
            let token = self.ensure_token().await?.to_owned();
            return Ok(build(&self.http, &token).send().await?);
        }
        Ok(resp)
    }

    /// Send a request, retrying on HTTP 429 with exponential backoff and
    /// refreshing the bearer token transparently on HTTP 401.
    ///
    /// The two retry concerns are independent:
    /// - **Expired token (401):** handled by `send_once`, which refreshes and
    ///   retries exactly once. This does not consume a rate-limit retry slot.
    /// - **Rate limiting (429):** retried up to `MAX_RETRY_ATTEMPTS` times with
    ///   exponential backoff (1 s → 2 s → 4 s, max 60 s). The `Retry-After`
    ///   response header is honoured when present.
    async fn send_with_retry(
        &mut self,
        build: impl Fn(&reqwest::Client, &str) -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, ApiError> {
        let mut delay = std::time::Duration::from_secs(1);
        for attempt in 0..MAX_RETRY_ATTEMPTS {
            let resp = self.send_once(&build).await?;
            let is_last = attempt + 1 >= MAX_RETRY_ATTEMPTS;
            if resp.status().as_u16() != 429 || is_last {
                return Ok(resp);
            }
            let wait = retry_after_duration(&resp).unwrap_or(delay);
            tokio::time::sleep(wait).await;
            delay = (delay * 2).min(std::time::Duration::from_secs(60));
        }
        // Every iteration either returns or sleeps and loops; the loop always
        // terminates via the early return on the last attempt.
        unreachable!()
    }

    async fn get<T: DeserializeOwned>(&mut self, path: &str) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .send_with_retry(|http, token| http.get(&url).bearer_auth(token))
            .await?;
        self.handle_response(resp).await
    }

    async fn get_with_query<T: DeserializeOwned>(
        &mut self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .send_with_retry(|http, token| http.get(&url).bearer_auth(token).query(params))
            .await?;
        self.handle_response(resp).await
    }

    /// Fetches all pages of a paginated endpoint, merging results into one value.
    async fn get_all_pages<T>(&mut self, path: &str, base_params: &[(&str, &str)]) -> Result<T, ApiError>
    where
        T: DeserializeOwned + types::Paginated,
    {
        let mut result: T = self.get_with_query(path, base_params).await?;
        loop {
            let token = match result.next_page_token() {
                Some(t) if !t.is_empty() => t.to_owned(),
                _ => break,
            };
            let mut params = base_params.to_vec();
            params.push(("next_page_token", token.as_str()));
            let next: T = self.get_with_query(path, &params).await?;
            result.append_page(next);
        }
        Ok(result)
    }

    async fn post<T: DeserializeOwned, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .send_with_retry(|http, token| http.post(&url).bearer_auth(token).json(body))
            .await?;
        self.handle_response(resp).await
    }

    async fn patch<B: Serialize>(&mut self, path: &str, body: &B) -> Result<(), ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .send_with_retry(|http, token| http.patch(&url).bearer_auth(token).json(body))
            .await?;
        self.handle_empty_response(resp).await
    }

    async fn put<B: Serialize>(&mut self, path: &str, body: &B) -> Result<(), ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .send_with_retry(|http, token| http.put(&url).bearer_auth(token).json(body))
            .await?;
        self.handle_empty_response(resp).await
    }

    async fn delete(&mut self, path: &str) -> Result<(), ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .send_with_retry(|http, token| http.delete(&url).bearer_auth(token))
            .await?;
        self.handle_empty_response(resp).await
    }

    async fn delete_with_query(
        &mut self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<(), ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .send_with_retry(|http, token| http.delete(&url).bearer_auth(token).query(params))
            .await?;
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
        let mut params: Vec<(&str, &str)> = vec![("page_size", "300")];
        let mt_owned;
        if let Some(mt) = meeting_type {
            mt_owned = mt.to_owned();
            params.push(("type", mt_owned.as_str()));
        }
        self.get_all_pages(&path, &params).await
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

    pub async fn end_meeting(&mut self, meeting_id: u64) -> Result<(), ApiError> {
        self.put(
            &format!("/meetings/{meeting_id}/status"),
            &MeetingStatusRequest { action: "end".into() },
        )
        .await
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    pub async fn list_users(&mut self, status: Option<&str>) -> Result<UserList, ApiError> {
        let mut params: Vec<(&str, &str)> = vec![("page_size", "300")];
        let st_owned;
        if let Some(st) = status {
            st_owned = st.to_owned();
            params.push(("status", st_owned.as_str()));
        }
        self.get_all_pages("/users", &params).await
    }

    pub async fn get_user(&mut self, user_id: &str) -> Result<User, ApiError> {
        self.get(&format!("/users/{user_id}")).await
    }

    // ── Participants ─────────────────────────────────────────────────────────

    pub async fn list_past_meeting_participants(
        &mut self,
        meeting_id: &str,
    ) -> Result<ParticipantList, ApiError> {
        let encoded_id = encode_meeting_id(meeting_id);
        self.get_all_pages(
            &format!("/past_meetings/{encoded_id}/participants"),
            &[("page_size", "300")],
        )
        .await
    }

    // ── Recordings ───────────────────────────────────────────────────────────

    pub async fn list_recordings(
        &mut self,
        user_id: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<RecordingList, ApiError> {
        let path = format!("/users/{user_id}/recordings");
        let mut params: Vec<(&str, &str)> = vec![("page_size", "300")];
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
        self.get_all_pages(&path, &params).await
    }

    /// Delete all cloud recording files for a meeting.
    ///
    /// `trash`: when `true`, moves files to the trash (recoverable for 30 days);
    /// when `false`, permanently deletes them immediately.
    pub async fn delete_recording(&mut self, meeting_id: &str, trash: bool) -> Result<(), ApiError> {
        let encoded_id = encode_meeting_id(meeting_id);
        let action = if trash { "trash" } else { "delete" };
        self.delete_with_query(
            &format!("/meetings/{encoded_id}/recordings"),
            &[("action", action)],
        )
        .await
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
        let encoded_id = encode_meeting_id(meeting_id);
        self.get(&format!("/meetings/{encoded_id}/recordings"))
            .await
    }

    /// Download a recording file to disk.
    ///
    /// Uses `send_with_retry` so expired tokens are refreshed and rate-limit
    /// retries apply, matching all other API calls. Writes to a `.download`
    /// temp file and renames atomically on success so a failed or interrupted
    /// download never leaves a partial file at the destination path.
    pub async fn download_recording_file(
        &mut self,
        download_url: &str,
        dest_path: &std::path::Path,
    ) -> Result<u64, ApiError> {
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        let url = download_url.to_owned();
        let resp = self
            .send_with_retry(|http, token| http.get(&url).bearer_auth(token))
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

        // Stream into a temp file; rename to the final path only on success.
        let tmp_path = dest_path.with_extension("download");
        let write_result: Result<u64, ApiError> = async {
            let mut file = tokio::fs::File::create(&tmp_path).await.map_err(|e| {
                ApiError::Other(format!("Cannot create file {}: {e}", tmp_path.display()))
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
        .await;

        match write_result {
            Ok(bytes) => {
                tokio::fs::rename(&tmp_path, dest_path).await.map_err(|e| {
                    ApiError::Other(format!("Cannot finalize download: {e}"))
                })?;
                Ok(bytes)
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&tmp_path).await;
                Err(e)
            }
        }
    }

    // ── Reports ───────────────────────────────────────────────────────────────

    pub async fn list_user_meeting_reports(
        &mut self,
        user_id: &str,
        from: &str,
        to: Option<&str>,
    ) -> Result<UserMeetingReportList, ApiError> {
        let mut params: Vec<(&str, &str)> = vec![("from", from), ("page_size", "300")];
        let to_owned;
        if let Some(t) = to {
            to_owned = t.to_owned();
            params.push(("to", to_owned.as_str()));
        }
        self.get_all_pages(&format!("/report/users/{user_id}/meetings"), &params)
            .await
    }

    // ── Webinars ──────────────────────────────────────────────────────────────

    pub async fn list_webinars(&mut self, user_id: &str) -> Result<WebinarList, ApiError> {
        let path = format!("/users/{user_id}/webinars");
        self.get_all_pages(&path, &[("page_size", "300")]).await
    }

    pub async fn get_webinar(&mut self, webinar_id: u64) -> Result<Webinar, ApiError> {
        self.get(&format!("/webinars/{webinar_id}")).await
    }
}

/// Percent-encode a Zoom meeting ID for use in URL path segments.
///
/// Zoom meeting UUIDs can contain `/` (base64 chars). When a UUID begins with
/// `/` or contains `//`, the Zoom API gateway decodes the path before routing,
/// so a single-encoded slash (`%2F`) would be decoded back to `/` and corrupt
/// the URL. Such UUIDs must be **double-encoded**: `/` → `%252F`, so that after
/// one decode pass the API handler sees `%2F` and correctly treats it as data.
fn encode_meeting_id(id: &str) -> String {
    if id.starts_with('/') || id.contains("//") {
        id.replace('/', "%252F")
    } else {
        id.replace('/', "%2F")
    }
}

/// Parse the `Retry-After` header as a delay duration.
///
/// Zoom uses integer seconds. Caps at 60 s to avoid extremely long waits from
/// misconfigured or adversarial responses.
fn retry_after_duration(resp: &reqwest::Response) -> Option<std::time::Duration> {
    let secs: u64 = resp
        .headers()
        .get("retry-after")?
        .to_str()
        .ok()?
        .parse()
        .ok()?;
    Some(std::time::Duration::from_secs(secs.min(60)))
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
    async fn rate_limit_response_is_retried_and_succeeds() {
        let server = MockServer::start().await;

        // First request: 429 with Retry-After: 0 (instant retry in tests).
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .respond_with(
                ResponseTemplate::new(429).insert_header("retry-after", "0"),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Second request: 200.
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [{"id": 1, "topic": "After retry"}],
                "total_records": 1
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_meetings("me", None).await.unwrap();
        assert_eq!(list.meetings.len(), 1, "result from the retry attempt");
        assert_eq!(list.meetings[0].topic, "After retry");
    }

    #[tokio::test]
    async fn rate_limit_then_expired_token_does_not_panic() {
        // Regression test: 429 on first attempts then 401 on the last attempt
        // previously hit the unreachable!() branch and panicked.
        let server = MockServer::start().await;

        // OAuth endpoint for token refresh triggered by the 401.
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "fresh-token",
                "token_type": "bearer",
                "expires_in": 3599
            })))
            .mount(&server)
            .await;

        // First three requests: 429 (rate limited).
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
            .up_to_n_times(3)
            .mount(&server)
            .await;

        // After the 429 retries, the next attempt gets a 401 (expired token).
        // send_once refreshes the token and retries — that retry also returns 401
        // (genuinely bad credentials), which is returned as ApiError::Auth.
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid token"))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let err = client.list_meetings("me", None).await.unwrap_err();
        // Must not panic; must surface as an auth error.
        assert!(matches!(err, ApiError::Auth(_)));
    }

    #[tokio::test]
    async fn rate_limit_exhausted_returns_rate_limit_error() {
        let server = MockServer::start().await;

        // All requests return 429 — retries exhausted.
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .respond_with(
                ResponseTemplate::new(429).insert_header("retry-after", "0"),
            )
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let err = client.list_meetings("me", None).await.unwrap_err();
        assert!(matches!(err, ApiError::RateLimit));
    }

    #[tokio::test]
    async fn expired_token_is_refreshed_transparently() {
        let server = MockServer::start().await;

        // OAuth endpoint — called when the cached token is cleared after a 401.
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "fresh-token",
                "token_type": "bearer",
                "expires_in": 3599
            })))
            .mount(&server)
            .await;

        // First request: 401 (expired token).
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(401).set_body_string("token expired"))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Second request: same endpoint, fresh token — succeeds.
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .and(header("authorization", "Bearer fresh-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [{"id": 1, "topic": "After refresh"}],
                "total_records": 1
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_meetings("me", None).await.unwrap();
        assert_eq!(list.meetings[0].topic, "After refresh");
    }

    #[tokio::test]
    async fn retry_after_header_is_parsed() {
        let server = MockServer::start().await;

        // Return 429 with a Retry-After header once, then 200.
        Mock::given(method("GET"))
            .and(path("/v2/users"))
            .respond_with(
                ResponseTemplate::new(429).insert_header("retry-after", "0"),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "users": [], "total_records": 0
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        // Should succeed after the retry.
        client.list_users(None).await.unwrap();
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

    #[tokio::test]
    async fn end_meeting_sends_put_with_action() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/v2/meetings/123456/status"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        let mut client = mock_client(&server).await;
        client.end_meeting(123456).await.unwrap();
    }

    #[tokio::test]
    async fn list_past_meeting_participants_returns_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/past_meetings/abc123/participants"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "participants": [
                    {"name": "Alice", "user_email": "alice@example.com", "duration": 1800}
                ],
                "total_records": 1
            })))
            .mount(&server)
            .await;
        let mut client = mock_client(&server).await;
        let list = client.list_past_meeting_participants("abc123").await.unwrap();
        assert_eq!(list.participants.len(), 1);
        assert_eq!(list.participants[0].name, Some("Alice".into()));
    }

    #[tokio::test]
    async fn list_user_meeting_reports_sends_from_param() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/report/users/me/meetings"))
            .and(query_param("from", "2026-04-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [],
                "total_records": 0,
                "from": "2026-04-01",
                "to": "2026-04-30"
            })))
            .mount(&server)
            .await;
        let mut client = mock_client(&server).await;
        let list = client.list_user_meeting_reports("me", "2026-04-01", None).await.unwrap();
        assert_eq!(list.meetings.len(), 0);
    }

    #[tokio::test]
    async fn list_meetings_follows_next_page_token() {
        let server = MockServer::start().await;

        // First page returns a next_page_token.
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .and(query_param("page_size", "300"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [{"id": 1, "topic": "Page 1 Meeting"}],
                "total_records": 2,
                "next_page_token": "token-abc"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Second page (identified by next_page_token) returns no further token.
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .and(query_param("next_page_token", "token-abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [{"id": 2, "topic": "Page 2 Meeting"}],
                "total_records": 2,
                "next_page_token": ""
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_meetings("me", None).await.unwrap();

        assert_eq!(list.meetings.len(), 2, "both pages must be merged");
        assert_eq!(list.meetings[0].topic, "Page 1 Meeting");
        assert_eq!(list.meetings[1].topic, "Page 2 Meeting");
        assert!(list.next_page_token.is_none(), "exhausted token must be absent");
    }

    #[test]
    fn encode_meeting_id_single_encodes_plain_uuids() {
        assert_eq!(encode_meeting_id("abc123"), "abc123");
        assert_eq!(encode_meeting_id("abc/def"), "abc%2Fdef");
    }

    #[test]
    fn encode_meeting_id_double_encodes_leading_slash() {
        // UUID starting with '/' must be double-encoded so the API gateway
        // does not consume the slash during path decoding.
        assert_eq!(encode_meeting_id("/abc"), "%252Fabc");
        assert_eq!(encode_meeting_id("/abc/def"), "%252Fabc%252Fdef");
    }

    #[test]
    fn encode_meeting_id_double_encodes_double_slash() {
        assert_eq!(encode_meeting_id("abc//def"), "abc%252F%252Fdef");
        assert_eq!(encode_meeting_id("4444AAAiAAAAAiAA//AA=="), "4444AAAiAAAAAiAA%252F%252FAA==");
    }

    #[tokio::test]
    async fn get_recording_double_encodes_uuid_with_double_slash() {
        let server = MockServer::start().await;
        // The path must contain %252F%252F (double-encoded), not %2F%2F.
        Mock::given(method("GET"))
            .and(path("/v2/meetings/abc%252F%252Fdef/recordings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 123,
                "topic": "Double-slash UUID meeting",
                "start_time": "2026-04-01T10:00:00Z",
                "duration": 30,
                "recording_files": []
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let rec = client.get_recording("abc//def").await.unwrap();
        assert_eq!(rec.topic, "Double-slash UUID meeting");
    }

    #[tokio::test]
    async fn list_users_follows_next_page_token() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/users"))
            .and(query_param("page_size", "300"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "users": [{"id": "u1", "email": "a@example.com"}],
                "total_records": 2,
                "next_page_token": "page2-token"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/users"))
            .and(query_param("next_page_token", "page2-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "users": [{"id": "u2", "email": "b@example.com"}],
                "total_records": 2,
                "next_page_token": ""
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_users(None).await.unwrap();

        assert_eq!(list.users.len(), 2, "both pages must be merged");
        assert_eq!(list.users[0].email, "a@example.com");
        assert_eq!(list.users[1].email, "b@example.com");
    }

    #[tokio::test]
    async fn list_participants_follows_next_page_token() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/past_meetings/mtg123/participants"))
            .and(query_param("page_size", "300"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "participants": [{"name": "Alice"}],
                "total_records": 2,
                "next_page_token": "p2"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/past_meetings/mtg123/participants"))
            .and(query_param("next_page_token", "p2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "participants": [{"name": "Bob"}],
                "total_records": 2,
                "next_page_token": ""
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_past_meeting_participants("mtg123").await.unwrap();

        assert_eq!(list.participants.len(), 2);
        assert_eq!(list.participants[0].name, Some("Alice".into()));
        assert_eq!(list.participants[1].name, Some("Bob".into()));
    }

    #[tokio::test]
    async fn delete_recording_sends_delete_with_action_trash() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v2/meetings/abc123/recordings"))
            .and(query_param("action", "trash"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        client.delete_recording("abc123", true).await.unwrap();
    }

    #[tokio::test]
    async fn delete_recording_sends_delete_with_action_delete() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v2/meetings/abc123/recordings"))
            .and(query_param("action", "delete"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        client.delete_recording("abc123", false).await.unwrap();
    }

    #[tokio::test]
    async fn list_webinars_returns_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/users/me/webinars"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "webinars": [
                    {
                        "id": 12345678,
                        "topic": "Product Launch",
                        "start_time": "2026-05-01T14:00:00Z",
                        "duration": 60,
                        "type": 5
                    }
                ],
                "total_records": 1
            })))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let list = client.list_webinars("me").await.unwrap();
        assert_eq!(list.webinars.len(), 1);
        assert_eq!(list.webinars[0].topic, "Product Launch");
    }

    #[tokio::test]
    async fn get_webinar_returns_404_as_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/webinars/99999999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Webinar not found"))
            .mount(&server)
            .await;

        let mut client = mock_client(&server).await;
        let err = client.get_webinar(99999999).await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }
}
