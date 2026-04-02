use serde::{Deserialize, Serialize};

fn is_none_or_empty(s: &Option<String>) -> bool {
    s.as_deref().is_none_or(str::is_empty)
}

// ── Pagination ────────────────────────────────────────────────────────────────

/// Implemented by every paginated list type so `ZoomClient::get_all_pages` can
/// collect results across multiple API requests transparently.
pub trait Paginated: Sized {
    /// Returns the token for the next page, or `None` / empty when exhausted.
    fn next_page_token(&self) -> Option<&str>;
    /// Merge `next` page's items into `self`, discarding `next`'s metadata.
    fn append_page(&mut self, next: Self);
}

// ── Meeting ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Meeting {
    pub id: u64,
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub meeting_type: Option<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeetingList {
    pub meetings: Vec<Meeting>,
    #[serde(skip_serializing_if = "is_none_or_empty")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<u64>,
    #[serde(skip_serializing)]
    pub page_count: Option<u32>,
    #[serde(skip_serializing)]
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CreateMeetingRequest {
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "type")]
    pub meeting_type: u8,
}

#[derive(Debug, Serialize, Default)]
pub struct UpdateMeetingRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
}

// ── User ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub user_type: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pic_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserList {
    pub users: Vec<User>,
    #[serde(skip_serializing_if = "is_none_or_empty")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<u64>,
    #[serde(skip_serializing)]
    pub page_count: Option<u32>,
    #[serde(skip_serializing)]
    pub page_size: Option<u32>,
}

// ── User management ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CreateUserInfo {
    pub email: String,
    #[serde(rename = "type")]
    pub user_type: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateUserRequest {
    pub action: String,
    pub user_info: CreateUserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserStatusRequest {
    pub action: String,
}

// ── Recording ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloudRecording {
    pub id: u64,
    pub topic: String,
    pub start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_files: Option<Vec<RecordingFile>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordingFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_end: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecordingList {
    // Zoom names this field "meetings" internally; we expose it as "recordings".
    #[serde(rename(deserialize = "meetings", serialize = "recordings"))]
    pub recordings: Option<Vec<CloudRecording>>,
    #[serde(skip_serializing_if = "is_none_or_empty")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

// ── Recording control ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RecordingControlRequest {
    pub action: String,
}

// ── Meeting status ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct MeetingStatusRequest {
    pub action: String,
}

// ── Meeting invitation ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeetingInvitation {
    pub invitation: String,
}

// ── Participants ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Participant {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leave_time: Option<String>,
    // Duration in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ParticipantList {
    pub participants: Vec<Participant>,
    #[serde(skip_serializing_if = "is_none_or_empty")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<u64>,
    #[serde(skip_serializing)]
    pub page_count: Option<u32>,
    #[serde(skip_serializing)]
    pub page_size: Option<u32>,
}

// ── Reports ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeetingReportItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participants_count: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserMeetingReportList {
    pub meetings: Vec<MeetingReportItem>,
    #[serde(skip_serializing_if = "is_none_or_empty")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing)]
    pub page_count: Option<u32>,
    #[serde(skip_serializing)]
    pub page_size: Option<u32>,
}

// ── Meeting participant report ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeetingParticipantReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leave_time: Option<String>,
    /// Participation duration in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MeetingParticipantReportList {
    pub participants: Vec<MeetingParticipantReport>,
    #[serde(skip_serializing_if = "is_none_or_empty")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<u64>,
    #[serde(skip_serializing)]
    pub page_size: Option<u32>,
}

// ── Webinar ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Webinar {
    pub id: u64,
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agenda: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub webinar_type: Option<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WebinarList {
    pub webinars: Vec<Webinar>,
    #[serde(skip_serializing_if = "is_none_or_empty")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_records: Option<u64>,
    #[serde(skip_serializing)]
    pub page_size: Option<u32>,
}

// ── Paginated impls ───────────────────────────────────────────────────────────

impl Paginated for MeetingList {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref().filter(|s| !s.is_empty())
    }

    fn append_page(&mut self, next: Self) {
        self.meetings.extend(next.meetings);
        // Normalize empty string to None — empty token means no more pages.
        self.next_page_token = next.next_page_token.filter(|t| !t.is_empty());
    }
}

impl Paginated for UserList {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref().filter(|s| !s.is_empty())
    }

    fn append_page(&mut self, next: Self) {
        self.users.extend(next.users);
        self.next_page_token = next.next_page_token.filter(|t| !t.is_empty());
    }
}

impl Paginated for RecordingList {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref().filter(|s| !s.is_empty())
    }

    fn append_page(&mut self, next: Self) {
        match (self.recordings.as_mut(), next.recordings) {
            (Some(dst), Some(src)) => dst.extend(src),
            (None, Some(src)) => self.recordings = Some(src),
            _ => {}
        }
        self.next_page_token = next.next_page_token.filter(|t| !t.is_empty());
    }
}

impl Paginated for ParticipantList {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref().filter(|s| !s.is_empty())
    }

    fn append_page(&mut self, next: Self) {
        self.participants.extend(next.participants);
        self.next_page_token = next.next_page_token.filter(|t| !t.is_empty());
    }
}

impl Paginated for UserMeetingReportList {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref().filter(|s| !s.is_empty())
    }

    fn append_page(&mut self, next: Self) {
        self.meetings.extend(next.meetings);
        self.next_page_token = next.next_page_token.filter(|t| !t.is_empty());
    }
}

impl Paginated for MeetingParticipantReportList {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref().filter(|s| !s.is_empty())
    }
    fn append_page(&mut self, next: Self) {
        self.participants.extend(next.participants);
        self.next_page_token = next.next_page_token;
    }
}

impl Paginated for WebinarList {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref().filter(|s| !s.is_empty())
    }

    fn append_page(&mut self, next: Self) {
        self.webinars.extend(next.webinars);
        self.next_page_token = next.next_page_token.filter(|t| !t.is_empty());
    }
}

// ── Auth ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meeting_deserializes_from_zoom_api_shape() {
        let json = r#"{
            "id": 123456789,
            "topic": "Weekly Standup",
            "start_time": "2026-04-03T09:00:00Z",
            "duration": 30,
            "join_url": "https://zoom.us/j/123456789",
            "start_url": "https://zoom.us/s/123456789",
            "status": "waiting",
            "created_at": "2026-04-02T10:00:00Z",
            "type": 2
        }"#;
        let m: Meeting = serde_json::from_str(json).unwrap();
        assert_eq!(m.id, 123456789);
        assert_eq!(m.topic, "Weekly Standup");
        assert_eq!(m.duration, Some(30));
        assert_eq!(m.meeting_type, Some(2));
    }

    #[test]
    fn meeting_serializes_skipping_null_fields() {
        let m = Meeting {
            id: 1,
            topic: "Test".into(),
            start_time: None,
            duration: Some(30),
            join_url: None,
            start_url: None,
            status: None,
            created_at: None,
            password: None,
            meeting_type: Some(2),
        };
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert!(v.get("start_time").is_none(), "null fields must be omitted");
        assert!(v.get("join_url").is_none(), "null fields must be omitted");
        assert_eq!(v["duration"], 30);
    }

    #[test]
    fn meeting_list_deserializes_with_pagination() {
        let json = r#"{
            "meetings": [
                {"id": 111, "topic": "A"},
                {"id": 222, "topic": "B"}
            ],
            "total_records": 2,
            "page_count": 1,
            "page_size": 30
        }"#;
        let list: MeetingList = serde_json::from_str(json).unwrap();
        assert_eq!(list.meetings.len(), 2);
        assert_eq!(list.total_records, Some(2));
        assert!(list.next_page_token.is_none());
    }

    #[test]
    fn meeting_list_serializes_without_page_internals() {
        let list = MeetingList {
            meetings: vec![],
            next_page_token: Some("".into()),
            total_records: Some(0),
            page_count: Some(1),
            page_size: Some(100),
        };
        let v: serde_json::Value = serde_json::to_value(&list).unwrap();
        assert!(
            v.get("page_count").is_none(),
            "page internals must be hidden"
        );
        assert!(
            v.get("page_size").is_none(),
            "page internals must be hidden"
        );
        assert!(
            v.get("next_page_token").is_none(),
            "empty token must be omitted"
        );
        assert_eq!(v["total_records"], 0);
    }

    #[test]
    fn user_deserializes_from_zoom_api_shape() {
        let json = r#"{
            "id": "user-abc-123",
            "email": "alice@example.com",
            "display_name": "Alice Smith",
            "first_name": "Alice",
            "last_name": "Smith",
            "status": "active",
            "type": 2
        }"#;
        let u: User = serde_json::from_str(json).unwrap();
        assert_eq!(u.id, "user-abc-123");
        assert_eq!(u.email, "alice@example.com");
        assert_eq!(u.display_name, Some("Alice Smith".into()));
        assert_eq!(u.user_type, Some(2));
    }

    #[test]
    fn user_serializes_skipping_null_fields() {
        let u = User {
            id: "u1".into(),
            email: "a@b.com".into(),
            display_name: Some("Alice".into()),
            first_name: None,
            last_name: None,
            status: Some("active".into()),
            user_type: Some(2),
            created_at: None,
            last_login_time: None,
            pic_url: None,
        };
        let v: serde_json::Value = serde_json::to_value(&u).unwrap();
        assert!(v.get("first_name").is_none());
        assert!(v.get("pic_url").is_none());
        assert_eq!(v["display_name"], "Alice");
    }

    #[test]
    fn recording_file_deserializes_with_optional_fields() {
        let json = r#"{
            "id": "rec-file-001",
            "file_type": "MP4",
            "file_size": 104857600,
            "download_url": "https://zoom.us/rec/download/abc123",
            "status": "completed",
            "recording_type": "shared_screen_with_speaker_view"
        }"#;
        let f: RecordingFile = serde_json::from_str(json).unwrap();
        assert_eq!(f.file_type, Some("MP4".into()));
        assert_eq!(f.file_size, Some(104857600));
    }

    #[test]
    fn recording_list_deserializes_meetings_key() {
        let json = r#"{
            "meetings": [
                {"id": 1, "topic": "Test", "start_time": "2026-01-01T10:00:00Z"}
            ],
            "total_records": 1,
            "from": "2026-01-01",
            "to": "2026-04-01",
            "next_page_token": ""
        }"#;
        let list: RecordingList = serde_json::from_str(json).unwrap();
        assert_eq!(list.recordings.as_ref().unwrap().len(), 1);
        assert_eq!(list.total_records, Some(1));
    }

    #[test]
    fn recording_list_serializes_recordings_key() {
        let list = RecordingList {
            recordings: Some(vec![]),
            next_page_token: Some("".into()),
            total_records: Some(0),
            from: Some("2026-01-01".into()),
            to: Some("2026-04-01".into()),
        };
        let v: serde_json::Value = serde_json::to_value(&list).unwrap();
        assert!(
            v.get("meetings").is_none(),
            "must serialize as 'recordings', not 'meetings'"
        );
        assert!(v.get("recordings").is_some());
        assert!(
            v.get("next_page_token").is_none(),
            "empty token must be omitted"
        );
    }

    #[test]
    fn token_response_deserializes() {
        let json = r#"{
            "access_token": "eyJhbGc...",
            "token_type": "bearer",
            "expires_in": 3599
        }"#;
        let t: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(t.access_token, "eyJhbGc...");
        assert_eq!(t.expires_in, 3599);
    }

    #[test]
    fn create_meeting_request_serializes_skipping_nones() {
        let req = CreateMeetingRequest {
            topic: "Demo".into(),
            start_time: None,
            duration: Some(45),
            password: None,
            meeting_type: 2,
        };
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["topic"], "Demo");
        assert_eq!(json["duration"], 45);
        assert_eq!(json["type"], 2);
        assert!(
            json.get("start_time").is_none(),
            "None fields must be omitted"
        );
        assert!(
            json.get("password").is_none(),
            "None fields must be omitted"
        );
    }

    #[test]
    fn participant_list_deserializes() {
        let json = r#"{
            "participants": [
                {
                    "id": "u1",
                    "name": "Alice",
                    "user_email": "alice@example.com",
                    "join_time": "2026-04-01T10:00:00Z",
                    "leave_time": "2026-04-01T11:00:00Z",
                    "duration": 3600
                }
            ],
            "total_records": 1,
            "page_size": 300,
            "next_page_token": ""
        }"#;
        let list: ParticipantList = serde_json::from_str(json).unwrap();
        assert_eq!(list.participants.len(), 1);
        assert_eq!(list.participants[0].name, Some("Alice".into()));
        assert_eq!(list.participants[0].duration, Some(3600));
        // page internals and empty token must not serialize
        let v: serde_json::Value = serde_json::to_value(&list).unwrap();
        assert!(v.get("page_size").is_none());
        assert!(v.get("next_page_token").is_none());
    }

    #[test]
    fn meeting_list_paginated_append_merges_items() {
        let mut first = MeetingList {
            meetings: vec![Meeting {
                id: 1,
                topic: "First".into(),
                start_time: None,
                duration: None,
                join_url: None,
                start_url: None,
                status: None,
                created_at: None,
                password: None,
                meeting_type: None,
            }],
            next_page_token: Some("tok".into()),
            total_records: Some(2),
            page_count: None,
            page_size: None,
        };
        let second = MeetingList {
            meetings: vec![Meeting {
                id: 2,
                topic: "Second".into(),
                start_time: None,
                duration: None,
                join_url: None,
                start_url: None,
                status: None,
                created_at: None,
                password: None,
                meeting_type: None,
            }],
            next_page_token: Some("".into()),
            total_records: Some(2),
            page_count: None,
            page_size: None,
        };
        first.append_page(second);
        assert_eq!(first.meetings.len(), 2);
        assert_eq!(first.next_page_token(), None, "empty token is exhausted");
        assert_eq!(
            first.total_records,
            Some(2),
            "total_records preserved from first page"
        );
    }

    #[test]
    fn recording_list_paginated_append_merges_option_vec() {
        let mut first = RecordingList {
            recordings: Some(vec![CloudRecording {
                id: 1,
                topic: "Rec 1".into(),
                start_time: "2026-01-01T10:00:00Z".into(),
                duration: None,
                total_size: None,
                recording_count: None,
                recording_files: None,
            }]),
            next_page_token: Some("t".into()),
            total_records: Some(2),
            from: None,
            to: None,
        };
        let second = RecordingList {
            recordings: Some(vec![CloudRecording {
                id: 2,
                topic: "Rec 2".into(),
                start_time: "2026-01-02T10:00:00Z".into(),
                duration: None,
                total_size: None,
                recording_count: None,
                recording_files: None,
            }]),
            next_page_token: None,
            total_records: Some(2),
            from: None,
            to: None,
        };
        first.append_page(second);
        assert_eq!(first.recordings.as_ref().unwrap().len(), 2);
        assert_eq!(first.next_page_token(), None);
    }

    #[test]
    fn webinar_list_deserializes_and_serializes() {
        let json = r#"{
            "webinars": [
                {
                    "id": 12345678,
                    "topic": "Product Launch",
                    "start_time": "2026-05-01T14:00:00Z",
                    "duration": 60,
                    "type": 5
                }
            ],
            "total_records": 1,
            "page_size": 300
        }"#;
        let list: WebinarList = serde_json::from_str(json).unwrap();
        assert_eq!(list.webinars.len(), 1);
        assert_eq!(list.webinars[0].id, 12345678);
        assert_eq!(list.webinars[0].topic, "Product Launch");
        assert_eq!(list.webinars[0].webinar_type, Some(5));
        // page_size must not appear in output
        let v: serde_json::Value = serde_json::to_value(&list).unwrap();
        assert!(v.get("page_size").is_none());
    }

    #[test]
    fn webinar_list_paginated_append_merges_items() {
        let mut first = WebinarList {
            webinars: vec![Webinar {
                id: 1,
                topic: "First".into(),
                start_time: None,
                duration: None,
                join_url: None,
                agenda: None,
                status: None,
                webinar_type: None,
            }],
            next_page_token: Some("tok".into()),
            total_records: Some(2),
            page_size: None,
        };
        let second = WebinarList {
            webinars: vec![Webinar {
                id: 2,
                topic: "Second".into(),
                start_time: None,
                duration: None,
                join_url: None,
                agenda: None,
                status: None,
                webinar_type: None,
            }],
            next_page_token: Some("".into()),
            total_records: Some(2),
            page_size: None,
        };
        first.append_page(second);
        assert_eq!(first.webinars.len(), 2);
        assert_eq!(first.next_page_token(), None);
    }

    #[test]
    fn meeting_report_list_deserializes() {
        let json = r#"{
            "meetings": [
                {
                    "uuid": "abc==",
                    "id": 123,
                    "topic": "Standup",
                    "start_time": "2026-04-01T09:00:00Z",
                    "duration": 30,
                    "participants_count": 5
                }
            ],
            "total_records": 1,
            "from": "2026-04-01",
            "to": "2026-04-30"
        }"#;
        let list: UserMeetingReportList = serde_json::from_str(json).unwrap();
        assert_eq!(list.meetings.len(), 1);
        assert_eq!(list.meetings[0].participants_count, Some(5));
    }
}
