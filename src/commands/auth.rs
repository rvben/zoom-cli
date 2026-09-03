use crate::api::{ApiError, ZoomClient};
use crate::config::{self, Config};
use crate::output::OutputConfig;

pub async fn login(profile: Option<String>) -> Result<(), ApiError> {
    super::init::init(profile).await
}

pub async fn status(
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
    out.print_result(
        &serde_json::json!({
            "profile": config.profile,
            "status": if offline { "configured" } else { "ok" },
            "configured": true,
            "verified": !offline,
            "credential_source": config.credential_source,
        }),
        &format!(
            "Profile '{}' is {} ({}).",
            config.profile,
            if offline {
                "configured; network not checked"
            } else {
                "authenticated"
            },
            config.credential_source
        ),
    );
    Ok(())
}

pub fn logout(profile: Option<&str>, out: &OutputConfig) -> Result<(), ApiError> {
    let profile = config::selected_profile_name(profile)?;
    let removed = config::remove_profile_secret(&config::config_path(), &profile)?;
    out.print_result(
        &serde_json::json!({
            "profile": profile,
            "logged_out": true,
            "credential_removed": removed,
            "environment_override": std::env::var_os("ZOOM_CLIENT_SECRET").is_some(),
        }),
        &format!("Logged out profile '{profile}'."),
    );
    Ok(())
}
