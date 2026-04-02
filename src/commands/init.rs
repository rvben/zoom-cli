use std::future::Future;
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use owo_colors::OwoColorize;

use crate::api::ApiError;
use crate::config;

const CORE_SCOPES: &[&str] = &[
    "meeting:read:list_meetings:master",
    "meeting:read:meeting:master",
    "meeting:write:meeting:master",
    "recording:read:list_user_recordings:master",
    "user:read:user:master",
    "user:read:list_users:master",
];

const OPTIONAL_SCOPES: &[&str] = &[
    "meeting:write:meeting:admin",
    "meeting:read:list_past_meeting_participants:admin",
    "report:read:user:admin",
    "recording:write:recording:master",
];

fn sym_q() -> String {
    "?".green().to_string()
}

fn sym_ok() -> String {
    "✔".green().to_string()
}

fn sym_fail() -> String {
    "✖".red().to_string()
}

fn mask_credential(s: &str) -> String {
    if s.len() <= 10 {
        return "•".repeat(s.len());
    }
    format!("{}…{}", &s[..6], &s[s.len() - 4..])
}

fn prompt_optional<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    default: &str,
) -> String {
    write!(writer, "{} {} ({}): ", sym_q(), label, default.dimmed()).unwrap();
    writer.flush().unwrap();

    let mut input = String::new();
    reader.read_line(&mut input).unwrap_or(0);
    let trimmed = input.trim().to_owned();
    if trimmed.is_empty() { default.to_owned() } else { trimmed }
}

fn prompt_required<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    hint: &str,
) -> String {
    loop {
        write!(writer, "{} {} [{}]: ", sym_q(), label, hint.dimmed()).unwrap();
        writer.flush().unwrap();

        let mut input = String::new();
        reader.read_line(&mut input).unwrap_or(0);
        let trimmed = input.trim().to_owned();
        if !trimmed.is_empty() {
            return trimmed;
        }
        writeln!(writer, "  {} {} is required.", sym_fail(), label).unwrap();
    }
}

fn prompt_confirm<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    default_yes: bool,
) -> bool {
    let hint = if default_yes { "Y/n" } else { "y/N" };
    write!(writer, "{} {} [{}]: ", sym_q(), label, hint.dimmed()).unwrap();
    writer.flush().unwrap();

    let mut input = String::new();
    reader.read_line(&mut input).unwrap_or(0);
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default_yes,
    }
}

fn print_json_schema(config_path: &Path) {
    let path_str = config_path.to_string_lossy();
    let schema = serde_json::json!({
        "configPath": path_str,
        "tokenInstructions": {
            "steps": [
                "Go to https://marketplace.zoom.us/develop/create",
                "Click 'Build App', choose 'Server-to-Server OAuth'",
                "Add the required scopes (see requiredScopes)",
                "Activate the app",
                "Copy Account ID, Client ID, and Client Secret from the app credentials page"
            ]
        },
        "requiredCredentials": ["account_id", "client_id", "client_secret"],
        "requiredScopes": CORE_SCOPES,
        "optionalScopes": OPTIONAL_SCOPES,
        "example": {
            "configFile": path_str,
            "format": "[default]\naccount_id = \"YOUR_ACCOUNT_ID\"\nclient_id = \"YOUR_CLIENT_ID\"\nclient_secret = \"YOUR_CLIENT_SECRET\""
        }
    });
    println!("{}", serde_json::to_string_pretty(&schema).expect("serialize"));
}

fn load_existing_profile_names(config_path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let table: toml::Table = match toml::from_str(&content) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    table.keys().cloned().collect()
}

/// Interactive init flow with injectable IO and validator for testing.
///
/// `validate` receives (account_id, client_id, client_secret) and returns
/// `Some(display_name)` on success or `None` on auth failure.
pub async fn run_init<R, W, Fut>(
    reader: &mut R,
    writer: &mut W,
    config_path: &Path,
    profile_arg: Option<&str>,
    validate: impl Fn(String, String, String) -> Fut,
) -> Result<(), ApiError>
where
    R: BufRead,
    W: Write,
    Fut: Future<Output = Option<String>>,
{
    let existing = load_existing_profile_names(config_path);

    if existing.is_empty() {
        writeln!(writer, "\nWelcome to zoom-cli!\n").unwrap();
        writeln!(
            writer,
            "This tool authenticates via a Zoom Server-to-Server OAuth app."
        )
        .unwrap();
        writeln!(writer, "You'll need to create one at https://marketplace.zoom.us\n").unwrap();
    } else {
        writeln!(
            writer,
            "\nUpdating zoom-cli config — existing profiles: {}\n",
            existing.join(", ")
        )
        .unwrap();
    }

    writeln!(writer, "Set up your Zoom Server-to-Server OAuth app:").unwrap();
    writeln!(writer, "  1. https://marketplace.zoom.us/develop/create").unwrap();
    writeln!(writer, "  2. Build App → Server-to-Server OAuth").unwrap();
    writeln!(writer, "  3. Add scopes:\n").unwrap();
    writeln!(writer, "     Core (required):").unwrap();
    for scope in CORE_SCOPES {
        writeln!(writer, "       • {scope}").unwrap();
    }
    writeln!(writer, "\n     Optional (end/participants/reports/recording-control):").unwrap();
    for scope in OPTIONAL_SCOPES {
        writeln!(writer, "       • {scope}").unwrap();
    }
    writeln!(writer, "\n  4. Activate the app\n").unwrap();

    write!(writer, "Press Enter when your app is ready (Ctrl+C to abort)... ").unwrap();
    writer.flush().unwrap();
    let mut _buf = String::new();
    reader.read_line(&mut _buf).unwrap_or(0);
    writeln!(writer).unwrap();

    let profile_name = if let Some(p) = profile_arg {
        p.to_owned()
    } else {
        prompt_optional(reader, writer, "Profile name", "default")
    };

    let account_id = prompt_required(reader, writer, "Account ID", "from app credentials");
    let client_id = prompt_required(reader, writer, "Client ID", "from app credentials");
    let client_secret = prompt_required(reader, writer, "Client Secret", "from app credentials");

    writeln!(writer).unwrap();
    writeln!(writer, "  Profile:       {}", profile_name.bold()).unwrap();
    writeln!(writer, "  Account ID:    {}", mask_credential(&account_id)).unwrap();
    writeln!(writer, "  Client ID:     {}", mask_credential(&client_id)).unwrap();
    writeln!(writer, "  Client Secret: {}\n", mask_credential(&client_secret)).unwrap();

    write!(writer, "{} Validating credentials... ", sym_q()).unwrap();
    writer.flush().unwrap();
    let validation = validate(account_id.clone(), client_id.clone(), client_secret.clone()).await;

    let save = match validation {
        Some(display_name) => {
            writeln!(writer, "{} Connected as {}", sym_ok(), display_name.bold()).unwrap();
            true
        }
        None => {
            writeln!(writer, "{} Could not validate credentials.", sym_fail()).unwrap();
            prompt_confirm(reader, writer, "Save anyway?", false)
        }
    };

    writeln!(writer).unwrap();

    if !save {
        writeln!(writer, "Aborted. Config not saved.").unwrap();
        writer.flush().unwrap();
        return Ok(());
    }

    config::write_profile(
        config_path,
        &profile_name,
        &account_id,
        &client_id,
        &client_secret,
    )?;

    writeln!(
        writer,
        "{} Config saved to {}\n",
        sym_ok(),
        config_path.display().to_string().bold()
    )
    .unwrap();
    writeln!(writer, "Run: {}", "zoom users me".bold()).unwrap();
    writer.flush().unwrap();

    Ok(())
}

/// Entry point from main — uses real stdin/stdout and live API validation.
pub async fn init(profile_arg: Option<String>) -> Result<(), ApiError> {
    let config_path = config::config_path();

    if !std::io::stdout().is_terminal() {
        print_json_schema(&config_path);
        return Ok(());
    }

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let mut writer = std::io::BufWriter::new(stdout.lock());

    run_init(
        &mut reader,
        &mut writer,
        &config_path,
        profile_arg.as_deref(),
        |account_id, client_id, client_secret| async move {
            let mut client =
                crate::api::ZoomClient::new(account_id, client_id, client_secret);
            match client.get_user("me").await {
                Ok(user) => Some(user.display_name.unwrap_or(user.email)),
                Err(_) => None,
            }
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tempfile::TempDir;

    use super::*;

    fn fake_path(dir: &TempDir) -> std::path::PathBuf {
        dir.path().join("config.toml")
    }

    #[tokio::test]
    async fn init_writes_config_on_valid_credentials() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);

        // Enter: ready, default profile, account, client_id, secret
        let input = b"\n\ntest-account-id\ntest-client-id\ntest-client-secret\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            None,
            |a, b, c| async move {
                let _ = (a, b, c);
                Some("Alice Smith".into())
            },
        )
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("account_id"));
        assert!(saved.contains("test-account-id"));
        assert!(saved.contains("test-client-id"));
        assert!(saved.contains("test-client-secret"));
    }

    #[tokio::test]
    async fn init_uses_default_profile_name_when_empty() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);

        let input = b"\n\ntest-acct\ntest-cid\ntest-csec\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            None,
            |a, b, c| async move {
                let _ = (a, b, c);
                Some("Test User".into())
            },
        )
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("[default]"), "should use 'default' profile name");
    }

    #[tokio::test]
    async fn init_with_profile_arg_skips_profile_prompt() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);

        // One fewer line (no profile name prompt)
        let input = b"\ntest-acct\ntest-cid\ntest-csec\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            Some("work"),
            |a, b, c| async move {
                let _ = (a, b, c);
                Some("Alice".into())
            },
        )
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("[work]"));
    }

    #[tokio::test]
    async fn init_aborts_when_validation_fails_and_user_declines_save() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);

        let input = b"\n\ntest-acct\ntest-cid\ntest-csec\nn\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            None,
            |a, b, c| async move {
                let _ = (a, b, c);
                None
            },
        )
        .await
        .unwrap();

        assert!(!path.exists(), "config should not be written after abort");
    }

    #[tokio::test]
    async fn init_saves_when_validation_fails_but_user_forces_save() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);

        let input = b"\n\ntest-acct\ntest-cid\ntest-csec\ny\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            None,
            |a, b, c| async move {
                let _ = (a, b, c);
                None
            },
        )
        .await
        .unwrap();

        assert!(path.exists(), "config should be saved when user chooses to save anyway");
    }

    #[tokio::test]
    async fn init_overwrites_existing_profile() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        std::fs::write(
            &path,
            "[default]\naccount_id = \"old\"\nclient_id = \"old\"\nclient_secret = \"old\"\n",
        )
        .unwrap();

        let input = b"\n\nnew-account\nnew-client\nnew-secret\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            None,
            |a, b, c| async move {
                let _ = (a, b, c);
                Some("Alice".into())
            },
        )
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("new-account"));
        assert!(!saved.contains("\"old\""), "old values should be overwritten");
    }

    #[test]
    fn mask_credential_masks_long_values() {
        assert_eq!(mask_credential("abcdefghijklmnop"), "abcdef…mnop");
    }

    #[test]
    fn mask_credential_dots_short_values() {
        assert_eq!(mask_credential("short"), "•••••");
        assert_eq!(mask_credential(""), "");
    }
}
