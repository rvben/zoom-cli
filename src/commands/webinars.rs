use crate::api::{ApiError, ZoomClient};
use crate::output::{self, OutputConfig};

pub async fn list(client: &mut ZoomClient, out: &OutputConfig, user: &str) -> Result<(), ApiError> {
    let result = client.list_webinars(user).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&result).expect("serialize"));
    } else {
        if result.webinars.is_empty() {
            out.print_message("No webinars found.");
            return Ok(());
        }
        let rows: Vec<Vec<String>> = result
            .webinars
            .iter()
            .map(|w| {
                vec![
                    w.id.to_string(),
                    w.topic.clone(),
                    w.start_time
                        .as_deref()
                        .map(output::format_timestamp)
                        .unwrap_or_else(|| "-".into()),
                    w.duration
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
            out.print_message(&format!("{total} webinar(s) total"));
        }
    }
    Ok(())
}

pub async fn get(
    client: &mut ZoomClient,
    out: &OutputConfig,
    webinar_id: u64,
) -> Result<(), ApiError> {
    let webinar = client.get_webinar(webinar_id).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&webinar).expect("serialize"));
    } else {
        let join_url = webinar.join_url.clone().unwrap_or_else(|| "-".into());
        out.print_data(&output::kv_block(&[
            ("id", webinar.id.to_string()),
            ("topic", webinar.topic.clone()),
            (
                "start_time",
                webinar
                    .start_time
                    .as_deref()
                    .map(output::format_timestamp)
                    .unwrap_or_else(|| "-".into()),
            ),
            (
                "duration",
                webinar
                    .duration
                    .map(|d| format!("{d} min"))
                    .unwrap_or_else(|| "-".into()),
            ),
            (
                "status",
                webinar.status.clone().unwrap_or_else(|| "-".into()),
            ),
            ("join_url", output::hyperlink(&join_url)),
            (
                "agenda",
                webinar.agenda.clone().unwrap_or_else(|| "-".into()),
            ),
        ]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ZoomClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_out() -> OutputConfig {
        OutputConfig::for_test()
    }

    #[tokio::test]
    async fn webinars_list_empty_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/users/me/webinars"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "webinars": [],
                "total_records": 0
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        list(&mut client, &test_out(), "me").await.unwrap();
    }

    #[tokio::test]
    async fn webinars_list_returns_table_data() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/users/me/webinars"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "webinars": [
                    {
                        "id": 12345678,
                        "topic": "Annual Summit",
                        "start_time": "2026-06-01T09:00:00Z",
                        "duration": 120,
                        "type": 5
                    }
                ],
                "total_records": 1
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        list(&mut client, &test_out(), "me").await.unwrap();
    }

    #[tokio::test]
    async fn webinars_get_returns_webinar() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/webinars/12345678"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 12345678,
                "topic": "Annual Summit",
                "start_time": "2026-06-01T09:00:00Z",
                "duration": 120,
                "join_url": "https://zoom.us/j/12345678",
                "status": "waiting",
                "agenda": "Keynote and breakouts",
                "type": 5
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        get(&mut client, &test_out(), 12345678).await.unwrap();
    }

    #[tokio::test]
    async fn webinars_get_not_found_propagates() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/webinars/99999999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Webinar not found"))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        let err = get(&mut client, &test_out(), 99999999).await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }
}
