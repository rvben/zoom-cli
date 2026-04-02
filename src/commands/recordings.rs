use crate::api::{ApiError, ZoomClient};
use crate::output::{self, OutputConfig};

/// Sanitize a string for use as a filename component.
///
/// Passes through alphanumeric characters, `-`, and `_` unchanged (lowercased).
/// All other characters are replaced with `_`.
fn sanitize_path_component(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

pub async fn list(
    client: &mut ZoomClient,
    out: &OutputConfig,
    user: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<(), ApiError> {
    let result = client.list_recordings(user, from, to).await?;
    let recordings = result.recordings.as_deref().unwrap_or_default();

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&result).expect("serialize"));
    } else {
        if recordings.is_empty() {
            out.print_message("No recordings found.");
            return Ok(());
        }
        let rows: Vec<Vec<String>> = recordings
            .iter()
            .map(|r| {
                vec![
                    r.id.to_string(),
                    r.topic.clone(),
                    output::format_timestamp(&r.start_time),
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
            ("id", recording.id.to_string()),
            ("topic", recording.topic.clone()),
            (
                "start_time",
                output::format_timestamp(&recording.start_time),
            ),
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

        let safe_topic = sanitize_path_component(&recording.topic);
        // Use recording_type (e.g. "shared_screen_with_speaker_view") as the
        // filename discriminator; multiple files of the same file_type (e.g.
        // two MP4 tracks) each have a distinct recording_type and won't
        // overwrite each other.
        let discriminator =
            sanitize_path_component(file.recording_type.as_deref().unwrap_or(file_type));
        let filename = format!(
            "{}_{}_{}.{}",
            safe_topic,
            recording.start_time.replace(':', "-").replace('T', "_"),
            discriminator,
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

pub async fn transcript(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: &str,
    out_dir: &str,
) -> Result<(), ApiError> {
    let recording = client.get_recording(meeting_id).await?;
    let files = recording.recording_files.unwrap_or_default();

    let transcript_files: Vec<_> = files
        .iter()
        .filter(|f| matches!(f.file_type.as_deref(), Some("TRANSCRIPT") | Some("CHAT")))
        .collect();

    if transcript_files.is_empty() {
        out.print_message("No transcript files found for this meeting.");
        return Ok(());
    }

    let dir = std::path::Path::new(out_dir);
    if !dir.exists() {
        std::fs::create_dir_all(dir)
            .map_err(|e| ApiError::Other(format!("Cannot create output directory: {e}")))?;
    }

    let mut paths: Vec<String> = Vec::new();

    for file in transcript_files {
        let download_url = match &file.download_url {
            Some(u) => u,
            None => continue,
        };

        let file_type = file.file_type.as_deref().unwrap_or("unknown");
        let ext = file
            .file_extension
            .as_deref()
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_else(|| file_type.to_ascii_lowercase());

        // Sanitize the meeting_id so API-sourced values cannot introduce path
        // traversal components.
        let safe_id = sanitize_path_component(meeting_id);
        let discriminator =
            sanitize_path_component(file.recording_type.as_deref().unwrap_or(file_type));

        let filename = format!("{safe_id}_{discriminator}.{ext}");
        let dest = dir.join(&filename);

        out.print_message(&format!("Downloading {} → {}", file_type, dest.display()));

        let bytes = client.download_recording_file(download_url, &dest).await?;
        out.print_message(&format!("  {:.1} MB written", bytes as f64 / 1_048_576.0));
        paths.push(dest.display().to_string());
    }

    out.print_result(
        &serde_json::json!({"files_downloaded": paths.len(), "paths": paths}),
        &paths
            .iter()
            .map(|p| format!("Downloaded: {p}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    Ok(())
}

pub async fn delete(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: &str,
    trash: bool,
) -> Result<(), ApiError> {
    client.delete_recording(meeting_id, trash).await?;
    let disposition = if trash {
        "moved to trash"
    } else {
        "permanently deleted"
    };
    out.print_result(
        &serde_json::json!({"deleted": true, "meeting_id": meeting_id, "trash": trash}),
        &format!("Recordings for meeting {meeting_id} {disposition}."),
    );
    Ok(())
}

pub async fn control(
    client: &mut ZoomClient,
    out: &OutputConfig,
    meeting_id: u64,
    action: &str,
) -> Result<(), ApiError> {
    client.control_recording(meeting_id, action).await?;
    out.print_result(
        &serde_json::json!({"action": action, "meeting_id": meeting_id}),
        &format!("Recording {action}ed for meeting {meeting_id}."),
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
                "id": 123456789,
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
    async fn recordings_control_start_sends_patch() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/v2/live_meetings/999888777/recordings"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        control(&mut client, &test_out(), 999888777, "start")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn recordings_control_invalid_meeting_returns_error() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/v2/live_meetings/111/recordings"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "code": 3001,
                "message": "Meeting does not exist"
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        let err = control(&mut client, &test_out(), 111, "start")
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::Api { .. }));
    }

    #[tokio::test]
    async fn recordings_delete_moves_to_trash_by_default() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v2/meetings/abc123/recordings"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        delete(&mut client, &test_out(), "abc123", true)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn recordings_delete_permanent_on_no_trash() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v2/meetings/abc123/recordings"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        delete(&mut client, &test_out(), "abc123", false)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn recordings_delete_not_found_propagates() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v2/meetings/nope/recordings"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Meeting not found"))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        let err = delete(&mut client, &test_out(), "nope", true)
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn recordings_transcript_downloads_vtt_file() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/meetings/mtg-abc/recordings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 111222333,
                "topic": "Team Sync",
                "start_time": "2026-04-01T10:00:00Z",
                "duration": 30,
                "recording_files": [
                    {
                        "id": "rf-tr-001",
                        "file_type": "TRANSCRIPT",
                        "file_extension": "VTT",
                        "recording_type": "audio_transcript",
                        "download_url": format!("{}/download/test-transcript.vtt", server.uri()),
                        "status": "completed"
                    }
                ]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/download/test-transcript.vtt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHello world\n"),
            )
            .mount(&server)
            .await;

        let tmp = tempfile::tempdir().unwrap();
        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        transcript(
            &mut client,
            &test_out(),
            "mtg-abc",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap();

        // Verify the file was written to disk
        let dest = tmp.path().join("mtg-abc_audio_transcript.vtt");
        assert!(dest.exists(), "transcript file must be written to disk");
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
