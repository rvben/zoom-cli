use std::future::Future;
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use owo_colors::OwoColorize;

use crate::api::ApiError;
use crate::config;
use crate::output;

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

const OAUTH_URL: &str = "https://marketplace.zoom.us/develop/create";
const SEP: &str = "──────────────────────────────────────";

fn sym_q() -> String {
    "?".green().bold().to_string()
}

fn sym_ok() -> String {
    "✔".green().to_string()
}

fn sym_fail() -> String {
    "✖".red().to_string()
}

fn sym_dim(s: &str) -> String {
    s.dimmed().to_string()
}

/// Prompt with a default value. Returns the default when the user presses Enter.
fn prompt_optional<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    default: &str,
) -> String {
    let _ = write!(writer, "{} {}  [{}]: ", sym_q(), label, sym_dim(default));
    let _ = writer.flush();

    let mut input = String::new();
    reader.read_line(&mut input).unwrap_or(0);
    let trimmed = input.trim().to_owned();
    if trimmed.is_empty() { default.to_owned() } else { trimmed }
}

/// Prompt for a required field, looping until a non-empty value is entered.
/// Returns `None` on EOF or IO error so the caller can abort gracefully.
fn prompt_required<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    hint: &str,
) -> Option<String> {
    loop {
        let _ = write!(writer, "{} {}  {}: ", sym_q(), label, sym_dim(&format!("[{hint}]")));
        let _ = writer.flush();

        let mut input = String::new();
        match reader.read_line(&mut input) {
            Ok(0) | Err(_) => return None,
            Ok(_) => {}
        }
        let trimmed = input.trim().to_owned();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
        let _ = writeln!(writer, "  {} {} is required.", sym_fail(), label);
    }
}

/// Prompt for a credential field during a profile update. Shows the masked
/// current value inline; pressing Enter keeps the existing value. Returns
/// `None` on EOF.
fn prompt_credential_update<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    current: &str,
) -> Option<String> {
    let hint = format!("{} (Enter to keep)", output::mask_credential(current));
    let _ = write!(writer, "{} {}  {}: ", sym_q(), label, sym_dim(&hint));
    let _ = writer.flush();

    let mut input = String::new();
    match reader.read_line(&mut input) {
        Ok(0) | Err(_) => return None,
        Ok(_) => {}
    }
    let trimmed = input.trim().to_owned();
    Some(if trimmed.is_empty() { current.to_owned() } else { trimmed })
}

fn prompt_confirm<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
    default_yes: bool,
) -> bool {
    let hint = if default_yes { "Y/n" } else { "y/N" };
    let _ = write!(writer, "{} {}  [{}]: ", sym_q(), label, sym_dim(hint));
    let _ = writer.flush();

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
///
/// The flow adapts to context:
/// - **First-ever setup** (no config file): defaults to "default" profile,
///   shows OAuth setup URL, prompts credentials.
/// - **Config exists, no `--profile` flag**: shows existing profiles and asks
///   whether to update an existing one or add a new one.
/// - **`--profile` given**: updates that profile if it exists, otherwise adds it.
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
    let _ = writeln!(writer, "\nzoom-cli");
    let _ = writeln!(writer, "{SEP}\n");

    let existing_profiles = load_existing_profile_names(config_path);
    let is_first_setup = existing_profiles.is_empty();

    // Determine the target profile and whether this is an update or a new entry.
    let (profile_name, is_update) = if let Some(p) = profile_arg {
        let is_update = existing_profiles.contains(&p.to_owned());
        (p.to_owned(), is_update)
    } else if is_first_setup {
        // First run: silently use "default" — no need to ask.
        ("default".to_owned(), false)
    } else {
        // Config exists: show what we have and ask what to do.
        if existing_profiles.len() == 1 {
            let p = &existing_profiles[0];
            let acct = config::read_profile_credentials(config_path, p)
                .map(|(a, _, _)| format!("  {}", output::mask_credential(&a)))
                .unwrap_or_default();
            let _ = writeln!(writer, "  Profile: {}{}\n", p.bold(), sym_dim(&acct));
        } else {
            let _ = writeln!(writer, "  Profiles:");
            for p in &existing_profiles {
                let acct = config::read_profile_credentials(config_path, p)
                    .map(|(a, _, _)| format!("  {}", output::mask_credential(&a)))
                    .unwrap_or_default();
                let _ = writeln!(writer, "    {}{}", p, sym_dim(&acct));
            }
            let _ = writeln!(writer);
        }

        let action = prompt_optional(reader, writer, "Action  [update/add]", "update");
        let _ = writeln!(writer);

        if action.trim().eq_ignore_ascii_case("add") {
            let Some(name) = prompt_required(reader, writer, "Profile name", "e.g. work") else {
                let _ = writeln!(writer, "\nAborted.");
                return Ok(());
            };
            (name, false)
        } else {
            // update (default)
            if existing_profiles.len() == 1 {
                (existing_profiles[0].clone(), true)
            } else {
                let options = existing_profiles.join("/");
                let chosen = prompt_optional(
                    reader,
                    writer,
                    &format!("Profile  [{}]", options),
                    &existing_profiles[0],
                );
                let profile = chosen.trim().to_owned();
                if !existing_profiles.contains(&profile) {
                    let _ = writeln!(writer, "\n  {} Unknown profile '{}'.", sym_fail(), profile);
                    return Ok(());
                }
                (profile, true)
            }
        }
    };

    // For new profiles, show where to create the OAuth app — no gate, just context.
    if !is_update {
        let _ = writeln!(writer, "  {}", sym_dim("Create a Server-to-Server OAuth app at:"));
        let _ = writeln!(writer, "  {}\n", sym_dim(OAUTH_URL));
    }

    // Prompt for credentials.
    let (account_id, client_id, client_secret) = if is_update {
        let (cur_acct, cur_cid, cur_csec) =
            config::read_profile_credentials(config_path, &profile_name)
                .expect("update mode requires existing credentials");
        let Some(account_id) =
            prompt_credential_update(reader, writer, "Account ID", &cur_acct)
        else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        let Some(client_id) =
            prompt_credential_update(reader, writer, "Client ID", &cur_cid)
        else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        let Some(client_secret) =
            prompt_credential_update(reader, writer, "Client Secret", &cur_csec)
        else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        (account_id, client_id, client_secret)
    } else {
        let Some(account_id) =
            prompt_required(reader, writer, "Account ID", "from app credentials")
        else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        let Some(client_id) =
            prompt_required(reader, writer, "Client ID", "from app credentials")
        else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        let Some(client_secret) =
            prompt_required(reader, writer, "Client Secret", "from app credentials")
        else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        (account_id, client_id, client_secret)
    };

    // Inline credential verification.
    let _ = write!(writer, "\n  Verifying credentials...");
    let _ = writer.flush();
    let validation = validate(account_id.clone(), client_id.clone(), client_secret.clone()).await;

    let save = match validation {
        Some(display_name) => {
            let _ = writeln!(writer, " {} Connected as {}", sym_ok(), display_name.bold());
            true
        }
        None => {
            let _ = writeln!(writer, " {} Could not validate credentials.", sym_fail());
            prompt_confirm(reader, writer, "Save anyway?", false)
        }
    };

    if !save {
        let _ = writeln!(writer, "\nAborted. Config not saved.");
        let _ = writer.flush();
        return Ok(());
    }

    config::write_profile(
        config_path,
        &profile_name,
        &account_id,
        &client_id,
        &client_secret,
    )?;

    let run_cmd = if profile_name == "default" {
        "zoom users me".to_owned()
    } else {
        format!("zoom --profile {} users me", profile_name)
    };

    let _ = writeln!(writer, "\n{SEP}");
    let _ = writeln!(
        writer,
        "  {} Config saved to {}",
        sym_ok(),
        sym_dim(&config_path.display().to_string()),
    );
    let _ = writeln!(writer, "  Run: {}", run_cmd.bold());
    let _ = writer.flush();

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

        // First setup: no action or profile prompts — go straight to credentials.
        let input = b"test-account-id\ntest-client-id\ntest-client-secret\n";
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

        // First setup silently defaults to "default" profile.
        let input = b"test-acct\ntest-cid\ntest-csec\n";
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

        // --profile given: no action prompt, go straight to credentials.
        let input = b"test-acct\ntest-cid\ntest-csec\n";
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

        let input = b"test-acct\ntest-cid\ntest-csec\nn\n";
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

        let input = b"test-acct\ntest-cid\ntest-csec\ny\n";
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

        // Config exists: \n accepts the "update" default at the action prompt,
        // then new values replace each credential.
        let input = b"\nnew-account\nnew-client\nnew-secret\n";
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

    #[tokio::test]
    async fn init_update_keeps_fields_when_enter_pressed() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        std::fs::write(
            &path,
            "[default]\naccount_id = \"keep-acct\"\nclient_id = \"keep-cid\"\nclient_secret = \"keep-csec\"\n",
        )
        .unwrap();

        // \n accepts "update" default at action prompt; subsequent \n's keep
        // each current credential value unchanged.
        let input = b"\n\n\n\n";
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
        assert!(saved.contains("keep-acct"), "kept account_id");
        assert!(saved.contains("keep-cid"), "kept client_id");
        assert!(saved.contains("keep-csec"), "kept client_secret");
    }

    #[tokio::test]
    async fn init_update_does_not_show_oauth_instructions() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        std::fs::write(
            &path,
            "[default]\naccount_id = \"acct\"\nclient_id = \"cid\"\nclient_secret = \"csec\"\n",
        )
        .unwrap();

        let input = b"\nnew-acct\nnew-cid\nnew-csec\n";
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

        let output = String::from_utf8_lossy(&writer);
        assert!(
            !output.contains("marketplace.zoom.us/develop/create"),
            "update mode must not show OAuth setup instructions"
        );
        assert!(output.contains("Profile:"), "should list the existing profile");
        assert!(output.contains("Action"), "should show the action prompt");
        assert!(output.contains("Enter to keep"), "credential prompts should show keep hint");
    }

    #[tokio::test]
    async fn init_adds_new_profile_to_existing_config() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        std::fs::write(
            &path,
            "[default]\naccount_id = \"def-acct\"\nclient_id = \"def-cid\"\nclient_secret = \"def-csec\"\n",
        )
        .unwrap();

        // --profile work (not existing): goes straight to credentials (new profile flow).
        let input = b"work-acct\nwork-cid\nwork-csec\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            Some("work"),
            |a, b, c| async move {
                let _ = (a, b, c);
                Some("Bob".into())
            },
        )
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("[default]"), "default profile preserved");
        assert!(saved.contains("[work]"), "new profile added");
        assert!(saved.contains("work-acct"));
        assert!(saved.contains("def-acct"), "existing credentials untouched");

        let output = String::from_utf8_lossy(&writer);
        assert!(
            !output.contains("Press Enter when your app is ready"),
            "must not show the old wait gate"
        );
    }

    #[tokio::test]
    async fn init_aborts_gracefully_on_eof_during_required_prompt() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);

        // First setup: EOF immediately on the Account ID prompt.
        let input = b"";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            None,
            |a, b, c| async move {
                let _ = (a, b, c);
                Some("Unreachable".into())
            },
        )
        .await
        .unwrap();

        assert!(!path.exists(), "config must not be written on aborted input");
        let output = String::from_utf8_lossy(&writer);
        assert!(output.contains("Aborted"), "should print an abort message");
    }

    #[tokio::test]
    async fn init_outro_includes_profile_flag_for_non_default_profiles() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);

        let input = b"work-acct\nwork-cid\nwork-csec\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            Some("work"),
            |a, b, c| async move {
                let _ = (a, b, c);
                Some("Bob".into())
            },
        )
        .await
        .unwrap();

        let output = String::from_utf8_lossy(&writer);
        assert!(
            output.contains("--profile work"),
            "outro should include --profile flag for non-default profiles"
        );
    }

    #[tokio::test]
    async fn init_action_add_prompts_for_new_profile_name() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        std::fs::write(
            &path,
            "[default]\naccount_id = \"def-acct\"\nclient_id = \"def-cid\"\nclient_secret = \"def-csec\"\n",
        )
        .unwrap();

        // Choose "add" action, then supply a profile name and credentials.
        let input = b"add\nstaging\nstg-acct\nstg-cid\nstg-csec\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(
            &mut reader,
            &mut writer,
            &path,
            None,
            |a, b, c| async move {
                let _ = (a, b, c);
                Some("Carol".into())
            },
        )
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("[default]"), "default profile preserved");
        assert!(saved.contains("[staging]"), "new profile added");
        assert!(saved.contains("stg-acct"));

        let output = String::from_utf8_lossy(&writer);
        assert!(
            output.contains(OAUTH_URL),
            "add flow should show OAuth URL"
        );
    }
}
