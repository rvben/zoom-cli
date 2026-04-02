use serde::{Deserialize, Serialize};

// ── Meeting ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Meeting {
    pub id: u64,
    pub topic: String,
    pub start_time: Option<String>,
    pub duration: Option<u32>,
    pub join_url: Option<String>,
    pub start_url: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "type")]
    pub meeting_type: Option<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeetingList {
    pub meetings: Vec<Meeting>,
    pub next_page_token: Option<String>,
    pub total_records: Option<u64>,
    pub page_count: Option<u32>,
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
    pub display_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub user_type: Option<u8>,
    pub created_at: Option<String>,
    pub last_login_time: Option<String>,
    pub pic_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserList {
    pub users: Vec<User>,
    pub next_page_token: Option<String>,
    pub total_records: Option<u64>,
    pub page_count: Option<u32>,
    pub page_size: Option<u32>,
}

// ── Recording ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloudRecording {
    pub id: String,
    pub topic: String,
    pub start_time: String,
    pub duration: Option<u32>,
    pub total_size: Option<u64>,
    pub recording_count: Option<u32>,
    pub recording_files: Option<Vec<RecordingFile>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordingFile {
    pub id: Option<String>,
    pub file_type: Option<String>,
    pub file_size: Option<u64>,
    pub play_url: Option<String>,
    pub download_url: Option<String>,
    pub status: Option<String>,
    pub recording_type: Option<String>,
    pub recording_start: Option<String>,
    pub recording_end: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecordingList {
    pub meetings: Option<Vec<CloudRecording>>,
    pub next_page_token: Option<String>,
    pub total_records: Option<u64>,
    pub from: Option<String>,
    pub to: Option<String>,
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
        assert!(json.get("start_time").is_none(), "None fields must be omitted");
        assert!(json.get("password").is_none(), "None fields must be omitted");
    }
}
