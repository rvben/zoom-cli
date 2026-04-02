use crate::api::types::{CreateMeetingRequest, UpdateMeetingRequest};
use crate::api::{ApiError, ZoomClient};
use crate::output::{self, OutputConfig};

pub async fn list(
    client: &mut ZoomClient,
    out: &OutputConfig,
    user: &str,
    meeting_type: Option<&str>,
) -> Result<(), ApiError> {
    let result = client.list_meetings(user, meeting_type).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&result).expect("serialize"));
    } else {
        if result.meetings.is_empty() {
            out.print_message("No meetings found.");
            return Ok(());
        }
        let rows: Vec<Vec<String>> = result
            .meetings
            .iter()
            .map(|m| {
                vec![
                    m.id.to_string(),
                    m.topic.clone(),
                    m.start_time
                        .as_deref()
                        .map(output::format_timestamp)
                        .unwrap_or_else(|| "-".into()),
                    m.duration
                        .map(|d| format!("{d} min"))
                        .unwrap_or_else(|| "-".into()),
                ]
            })
            .collect();
        out.print_data(&output::table(
            &["ID", "TOPIC", "START TIME", "DURATION"],
            &rows,
        ));
        if let Some(total) = result.total_records {
            out.print_message(&format!("{total} meeting(s) total"));
        }
    }
    Ok(())
}

pub async fn get(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: u64,
) -> Result<(), ApiError> {
    let meeting = client.get_meeting(meeting_id).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&meeting).expect("serialize"));
    } else {
        let join_url = meeting.join_url.clone().unwrap_or_else(|| "-".into());
        out.print_data(&output::kv_block(&[
            ("id", meeting.id.to_string()),
            ("topic", meeting.topic.clone()),
            (
                "start_time",
                meeting
                    .start_time
                    .as_deref()
                    .map(output::format_timestamp)
                    .unwrap_or_else(|| "-".into()),
            ),
            (
                "duration",
                meeting
                    .duration
                    .map(|d| format!("{d} min"))
                    .unwrap_or_else(|| "-".into()),
            ),
            (
                "status",
                meeting.status.clone().unwrap_or_else(|| "-".into()),
            ),
            ("join_url", output::hyperlink(&join_url)),
        ]));
    }
    Ok(())
}

pub async fn create(
    client: &mut ZoomClient,
    out: &OutputConfig,
    topic: String,
    duration: Option<u32>,
    start: Option<String>,
    password: Option<String>,
) -> Result<(), ApiError> {
    let meeting_type = if start.is_some() { 2 } else { 1 };
    let req = CreateMeetingRequest {
        topic,
        start_time: start,
        duration,
        password,
        meeting_type,
    };
    let meeting = client.create_meeting("me", req).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&meeting).expect("serialize"));
    } else {
        let join_url = meeting.join_url.clone().unwrap_or_else(|| "-".into());
        out.print_result(
            &serde_json::json!({}),
            &format!(
                "Meeting created: {} (ID: {})\nJoin URL: {}",
                meeting.topic,
                meeting.id,
                output::hyperlink(&join_url)
            ),
        );
    }
    Ok(())
}

pub async fn update(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: u64,
    topic: Option<String>,
    duration: Option<u32>,
    start: Option<String>,
) -> Result<(), ApiError> {
    let req = UpdateMeetingRequest {
        topic,
        duration,
        start_time: start,
    };
    client.update_meeting(meeting_id, req).await?;

    out.print_result(
        &serde_json::json!({"updated": true, "id": meeting_id}),
        &format!("Meeting {meeting_id} updated."),
    );
    Ok(())
}

pub async fn delete(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: u64,
) -> Result<(), ApiError> {
    client.delete_meeting(meeting_id).await?;

    out.print_result(
        &serde_json::json!({"deleted": true, "id": meeting_id}),
        &format!("Meeting {meeting_id} deleted."),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ZoomClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_out() -> OutputConfig {
        OutputConfig {
            json: true,
            quiet: true,
        }
    }

    #[tokio::test]
    async fn meetings_list_empty_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/users/me/meetings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [], "total_records": 0
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        list(&mut client, &test_out(), "me", None).await.unwrap();
    }

    #[tokio::test]
    async fn meetings_create_returns_meeting() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/users/me/meetings"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 123456789,
                "topic": "New Meeting",
                "join_url": "https://zoom.us/j/123456789",
                "start_url": "https://zoom.us/s/123456789"
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        create(
            &mut client,
            &test_out(),
            "New Meeting".into(),
            Some(30),
            None,
            None,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn meetings_delete_succeeds_on_204() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v2/meetings/111222333"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        delete(&mut client, &test_out(), 111222333).await.unwrap();
    }

    #[tokio::test]
    async fn meetings_get_not_found_propagates() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/meetings/999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        let err = get(&mut client, &test_out(), 999).await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn meetings_update_sends_patch_and_returns_ok() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/v2/meetings/123"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        update(
            &mut client,
            &test_out(),
            123,
            Some("Updated".into()),
            None,
            None,
        )
        .await
        .unwrap();
    }
}
