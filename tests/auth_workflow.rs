use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn zoom(config_home: &TempDir) -> Command {
    let mut command = Command::cargo_bin("zoom").expect("zoom binary");
    command
        .env("XDG_CONFIG_HOME", config_home.path())
        .env_remove("ZOOM_ACCOUNT_ID")
        .env_remove("ZOOM_CLIENT_ID")
        .env_remove("ZOOM_CLIENT_SECRET")
        .env_remove("ZOOM_PROFILE");
    command
}

fn write_profiles(config_home: &TempDir) {
    let directory = config_home.path().join("zoom-cli");
    std::fs::create_dir_all(&directory).expect("config directory");
    std::fs::write(
        directory.join("config.toml"),
        r#"active_profile = "default"

[default]
account_id = "default-account"
client_id = "default-client"
client_secret = "default-secret"

[work]
account_id = "work-account"
client_id = "work-client"
client_secret = "work-secret"
"#,
    )
    .expect("config file");
}

fn stdout_json(output: std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON stdout")
}

#[test]
fn canonical_profile_auth_and_config_workflow() {
    let config_home = TempDir::new().expect("temp config home");
    write_profiles(&config_home);

    let profiles = stdout_json(
        zoom(&config_home)
            .args(["profile", "list", "--output", "json"])
            .output()
            .expect("profile list"),
    );
    assert_eq!(profiles["total"], 2);
    assert_eq!(profiles["items"][0]["name"], "default");
    assert_eq!(profiles["items"][0]["active"], true);
    assert_eq!(profiles["items"][1]["name"], "work");

    let selected = stdout_json(
        zoom(&config_home)
            .args(["profile", "use", "work", "--output", "json"])
            .output()
            .expect("profile use"),
    );
    assert_eq!(selected["profile"], "work");
    assert_eq!(selected["active"], true);

    let status = stdout_json(
        zoom(&config_home)
            .args(["auth", "status", "--offline", "--output", "json"])
            .output()
            .expect("auth status"),
    );
    assert_eq!(status["profile"], "work");
    assert_eq!(status["status"], "configured");
    assert_eq!(status["verified"], false);
    assert_eq!(status["credential_source"], "config-file");

    let doctor = stdout_json(
        zoom(&config_home)
            .args(["doctor", "--offline", "--output", "json"])
            .output()
            .expect("doctor"),
    );
    assert_eq!(doctor["ok"], true);
    assert_eq!(doctor["offline"], true);
    assert_eq!(doctor["checks"].as_array().map(Vec::len), Some(3));

    let logout = stdout_json(
        zoom(&config_home)
            .args(["auth", "logout", "--output", "json"])
            .output()
            .expect("auth logout"),
    );
    assert_eq!(logout["profile"], "work");
    assert_eq!(logout["credential_removed"], true);

    let config = std::fs::read_to_string(config_home.path().join("zoom-cli/config.toml"))
        .expect("updated config");
    let document: toml::Value = toml::from_str(&config).expect("valid TOML");
    assert_eq!(
        document["work"]["account_id"].as_str(),
        Some("work-account")
    );
    assert!(document["work"].get("client_secret").is_none());
    assert_eq!(
        document["default"]["client_secret"].as_str(),
        Some("default-secret")
    );

    let config_path = stdout_json(
        zoom(&config_home)
            .args(["config", "path", "--output", "json"])
            .output()
            .expect("config path"),
    );
    assert_eq!(
        config_path["config_path"].as_str(),
        Some(
            config_home
                .path()
                .join("zoom-cli/config.toml")
                .to_string_lossy()
                .as_ref()
        )
    );
}

#[test]
fn auth_login_and_init_share_the_setup_flow() {
    let config_home = TempDir::new().expect("temp config home");

    let login = stdout_json(
        zoom(&config_home)
            .args(["auth", "login", "--profile", "work", "--output", "json"])
            .output()
            .expect("auth login"),
    );
    let init = stdout_json(
        zoom(&config_home)
            .args(["init", "--profile", "work", "--output", "json"])
            .output()
            .expect("init"),
    );

    assert_eq!(login["requiredCredentials"], init["requiredCredentials"]);
    assert_eq!(login["requiredScopes"], init["requiredScopes"]);
    assert_eq!(login["configPath"], init["configPath"]);
}

#[test]
fn profile_remove_requires_yes_and_updates_active_profile() {
    let config_home = TempDir::new().expect("temp config home");
    write_profiles(&config_home);

    zoom(&config_home)
        .args(["profile", "use", "work"])
        .assert()
        .success();

    let denied = zoom(&config_home)
        .args(["profile", "remove", "work", "--output", "json"])
        .output()
        .expect("profile remove without confirmation");
    assert_eq!(denied.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&denied.stderr).expect("structured error");
    assert_eq!(error["error"]["kind"], "confirmation_required");

    let removed = stdout_json(
        zoom(&config_home)
            .args(["profile", "remove", "work", "--yes", "--output", "json"])
            .output()
            .expect("profile remove"),
    );
    assert_eq!(removed["profile"], "work");
    assert_eq!(removed["removed"], true);

    let shown = stdout_json(
        zoom(&config_home)
            .args(["config", "show", "--output", "json"])
            .output()
            .expect("config show"),
    );
    assert_eq!(shown["active_profile"], "default");
    assert!(shown["profiles"].get("work").is_none());
}

#[test]
fn schema_advertises_the_standard_auth_contract() {
    let config_home = TempDir::new().expect("temp config home");
    let schema = stdout_json(zoom(&config_home).arg("schema").output().expect("schema"));
    let names = schema["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .filter_map(|command| command["name"].as_str())
        .collect::<Vec<_>>();
    for required in [
        "auth login",
        "auth status",
        "auth logout",
        "profile list",
        "profile use",
        "profile remove",
        "config show",
        "config path",
        "doctor",
    ] {
        assert!(names.contains(&required), "schema missing {required}");
    }
}
