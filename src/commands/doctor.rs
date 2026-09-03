use crate::api::{ApiError, ZoomClient};
use crate::config::Config;
use crate::output::OutputConfig;

pub async fn run(
    profile: Option<String>,
    offline: bool,
    out: &OutputConfig,
) -> Result<(), ApiError> {
    let config = Config::load(profile)?;
    if !offline {
        let mut client = ZoomClient::new(
            config.account_id.clone(),
            config.client_id.clone(),
            config.client_secret.clone(),
        );
        client.verify_credentials().await?;
    }
    let checks = serde_json::json!([
        {"name": "configuration", "ok": true, "detail": format!("profile '{}'", config.profile)},
        {"name": "credentials", "ok": true, "detail": config.credential_source},
        {"name": "authentication", "ok": true, "detail": if offline { "network check skipped" } else { "OAuth token acquired" }},
    ]);
    if out.json {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "offline": offline,
                "checks": checks,
            }))
            .expect("serialize doctor result"),
        );
    } else {
        out.print_data(if offline {
            "Zoom connection (offline)"
        } else {
            "Zoom connection"
        });
        for check in checks.as_array().expect("checks are an array") {
            out.print_data(&format!(
                "  ✓ {:<16} {}",
                check["name"].as_str().unwrap_or("check"),
                check["detail"].as_str().unwrap_or_default()
            ));
        }
    }
    Ok(())
}
