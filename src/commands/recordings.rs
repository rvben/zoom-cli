use crate::api::{ApiError, ZoomClient};
use crate::output::{self, OutputConfig};

pub async fn list(
    client: &mut ZoomClient,
    out: &OutputConfig,
    user: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<(), ApiError> {
    let result = client.list_recordings(user, from, to).await?;
    let recordings = result.meetings.unwrap_or_default();

    if out.json {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
                "recordings": recordings,
                "totalRecords": result.total_records
            }))
            .expect("serialize"),
        );
    } else {
        if recordings.is_empty() {
            out.print_message("No recordings found.");
            return Ok(());
        }
        let rows: Vec<Vec<String>> = recordings
            .iter()
            .map(|r| {
                vec![
                    r.id.clone(),
                    r.topic.clone(),
                    r.start_time.clone(),
                    r.duration
                        .map(|d| format!("{d} min"))
                        .unwrap_or_else(|| "-".into()),
                    r.recording_files
                        .as_ref()
                        .map(|f| f.len().to_string())
                        .unwrap_or_else(|| "0".into()),
                ]
            })
            .collect();
        out.print_data(&output::table(
            &["ID", "TOPIC", "START TIME", "DURATION", "FILES"],
            &rows,
        ));
        if let Some(total) = result.total_records {
            out.print_message(&format!("{total} recording(s) total"));
        }
    }
    Ok(())
}

pub async fn get(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: &str,
) -> Result<(), ApiError> {
    let recording = client.get_recording(meeting_id).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&recording).expect("serialize"));
    } else {
        out.print_data(&output::kv_block(&[
            ("id", recording.id.clone()),
            ("topic", recording.topic.clone()),
            ("start_time", recording.start_time.clone()),
            (
                "duration",
                recording
                    .duration
                    .map(|d| format!("{d} min"))
                    .unwrap_or_else(|| "-".into()),
            ),
            (
                "files",
                recording
                    .recording_files
                    .as_ref()
                    .map(|f| f.len().to_string())
                    .unwrap_or_else(|| "0".into()),
            ),
        ]));
        if let Some(files) = &recording.recording_files
            && !files.is_empty()
        {
            out.print_data("\nFiles:");
            for f in files {
                let file_type = f.file_type.clone().unwrap_or_else(|| "unknown".into());
                let size = f
                    .file_size
                    .map(|s| format!("{:.1} MB", s as f64 / 1_048_576.0))
                    .unwrap_or_else(|| "-".into());
                out.print_data(&format!(
                    "  {} {} {}",
                    file_type,
                    size,
                    f.download_url.as_deref().unwrap_or("-")
                ));
            }
        }
    }
    Ok(())
}

pub async fn download(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: &str,
    out_dir: &str,
) -> Result<(), ApiError> {
    let recording = client.get_recording(meeting_id).await?;
    let files = recording.recording_files.unwrap_or_default();

    if files.is_empty() {
        out.print_message("No recording files found for this meeting.");
        return Ok(());
    }

    let dir = std::path::Path::new(out_dir);
    if !dir.exists() {
        std::fs::create_dir_all(dir)
            .map_err(|e| ApiError::Other(format!("Cannot create output directory: {e}")))?;
    }

    let mut downloaded = 0usize;
    for file in &files {
        let download_url = match &file.download_url {
            Some(u) => u,
            None => continue,
        };

        let file_type = file.file_type.as_deref().unwrap_or("unknown");
        let ext = match file_type {
            "MP4" => "mp4",
            "M4A" => "m4a",
            "CHAT" => "txt",
            "TRANSCRIPT" => "vtt",
            "TIMELINE" => "json",
            _ => "bin",
        };

        let safe_topic: String = recording
            .topic
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let filename = format!(
            "{}_{}_{}.{}",
            safe_topic,
            recording.start_time.replace(':', "-").replace('T', "_"),
            file_type.to_lowercase(),
            ext
        );
        let dest = dir.join(&filename);

        out.print_message(&format!("Downloading {} → {}", file_type, dest.display()));

        let bytes = client.download_recording_file(download_url, &dest).await?;
        out.print_message(&format!("  {:.1} MB written", bytes as f64 / 1_048_576.0));
        downloaded += 1;
    }

    out.print_result(
        &serde_json::json!({"downloaded": downloaded, "meeting_id": meeting_id, "out_dir": out_dir}),
        &format!("{downloaded} file(s) downloaded to {out_dir}"),
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
    async fn recordings_list_empty_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/users/me/recordings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "meetings": [],
                "total_records": 0
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        list(&mut client, &test_out(), "me", None, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn recordings_get_returns_recording_with_files() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/meetings/abc123/recordings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "abc123",
                "topic": "Design Review",
                "start_time": "2026-04-01T10:00:00Z",
                "duration": 45,
                "recording_files": [
                    {
                        "id": "rf-001",
                        "file_type": "MP4",
                        "file_size": 52428800,
                        "download_url": "https://zoom.us/rec/download/abc123.mp4",
                        "status": "completed"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        get(&mut client, &test_out(), "abc123").await.unwrap();
    }

    #[tokio::test]
    async fn recordings_get_not_found_propagates() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/meetings/nope/recordings"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        let err = get(&mut client, &test_out(), "nope").await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }
}
