use crate::api::{ApiError, ZoomClient};
use crate::output::{self, OutputConfig};

pub async fn meetings(
    client: &mut ZoomClient,
    out: &OutputConfig,
    user: &str,
    from: &str,
    to: Option<&str>,
) -> Result<(), ApiError> {
    let result = client.list_user_meeting_reports(user, from, to).await?;

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
                    m.id.map(|id| id.to_string()).unwrap_or_default(),
                    m.topic.clone().unwrap_or_default(),
                    m.start_time
                        .as_deref()
                        .map(output::format_timestamp)
                        .unwrap_or_else(|| "-".into()),
                    m.duration
                        .map(|d| format!("{d} min"))
                        .unwrap_or_else(|| "-".into()),
                    m.participants_count
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "-".into()),
                ]
            })
            .collect();
        out.print_data(&output::table(
            &["ID", "TOPIC", "START TIME", "DURATION", "PARTICIPANTS"],
            &rows,
        ));
        if let Some(total) = result.total_records {
            out.print_message(&format!("{total} meeting(s) total"));
        }
    }
    Ok(())
}

pub async fn participants(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: &str,
) -> Result<(), ApiError> {
    let result = client.list_meeting_participant_reports(meeting_id).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&result).expect("serialize"));
    } else {
        if result.participants.is_empty() {
            out.print_message("No participants found.");
            return Ok(());
        }
        let rows: Vec<Vec<String>> = result
            .participants
            .iter()
            .map(|p| {
                vec![
                    p.name.clone().unwrap_or_default(),
                    p.user_email.clone().unwrap_or_default(),
                    p.join_time
                        .as_deref()
                        .map(output::format_timestamp)
                        .unwrap_or_else(|| "-".into()),
                    p.duration
                        .map(|d| format!("{} min", d / 60))
                        .unwrap_or_else(|| "-".into()),
                ]
            })
            .collect();
        out.print_data(&output::table(
            &["NAME", "EMAIL", "JOIN TIME", "DURATION"],
            &rows,
        ));
        if let Some(total) = result.total_records {
            out.print_message(&format!("{total} participant(s) total"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ZoomClient;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_out() -> OutputConfig {
        OutputConfig {
            json: true,
            quiet: true,
        }
    }

    #[tokio::test]
    async fn reports_meetings_empty_is_ok() {
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
        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        meetings(&mut client, &test_out(), "me", "2026-04-01", None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn reports_meetings_returns_rows() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/report/users/me/meetings"))
            .and(query_param("from", "2026-04-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [
                    {
                        "uuid": "abc==",
                        "id": 123456789,
                        "topic": "Weekly Standup",
                        "start_time": "2026-04-01T09:00:00Z",
                        "duration": 30,
                        "participants_count": 8
                    }
                ],
                "total_records": 1,
                "from": "2026-04-01",
                "to": "2026-04-30"
            })))
            .mount(&server)
            .await;
        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        meetings(&mut client, &test_out(), "me", "2026-04-01", None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn reports_participants_returns_rows() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/report/meetings/123456789/participants"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "participants": [
                    {
                        "name": "Alice",
                        "user_email": "alice@example.com",
                        "join_time": "2026-04-01T09:00:00Z",
                        "duration": 1800
                    }
                ],
                "total_records": 1
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        participants(&mut client, &test_out(), "123456789")
            .await
            .unwrap();
    }
}
