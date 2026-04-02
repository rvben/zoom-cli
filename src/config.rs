use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::api::ApiError;

#[derive(Debug, Deserialize, Default, Clone)]
struct RawProfile {
    pub account_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    default: RawProfile,
    #[serde(flatten)]
    profiles: BTreeMap<String, RawProfile>,
}

/// Resolved credentials for the active profile.
#[derive(Debug, Clone)]
pub struct Config {
    pub account_id: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Config {
    /// Load config with priority: env vars > config file profile.
    pub fn load(profile_arg: Option<String>) -> Result<Self, ApiError> {
        let file_profile = load_file_profile(profile_arg.as_deref())?;

        let account_id = env_var("ZOOM_ACCOUNT_ID")
            .or_else(|| normalize(file_profile.account_id))
            .ok_or_else(|| {
                ApiError::InvalidInput(
                    "No account_id configured. Run 'zoom init' or set ZOOM_ACCOUNT_ID.".into(),
                )
            })?;

        let client_id = env_var("ZOOM_CLIENT_ID")
            .or_else(|| normalize(file_profile.client_id))
            .ok_or_else(|| {
                ApiError::InvalidInput(
                    "No client_id configured. Run 'zoom init' or set ZOOM_CLIENT_ID.".into(),
                )
            })?;

        let client_secret = env_var("ZOOM_CLIENT_SECRET")
            .or_else(|| normalize(file_profile.client_secret))
            .ok_or_else(|| {
                ApiError::InvalidInput(
                    "No client_secret configured. Run 'zoom init' or set ZOOM_CLIENT_SECRET."
                        .into(),
                )
            })?;

        Ok(Self {
            account_id,
            client_id,
            client_secret,
        })
    }
}

/// Per-profile credential values as stored in the config file.
pub struct ProfileSummary {
    pub name: String,
    pub account_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

/// Full configuration state for display — no credential resolution or validation.
pub struct ConfigSummary {
    pub config_file: PathBuf,
    pub file_exists: bool,
    /// The profile that will be used (from --profile arg, ZOOM_PROFILE, or "default").
    pub active_profile: String,
    /// All profiles found in the config file, "default" first.
    pub profiles: Vec<ProfileSummary>,
    /// Environment variables that are set and will override file values.
    /// Each entry is `(var_name, raw_value)`.
    pub env_overrides: Vec<(&'static str, String)>,
}

/// Read all config state for display without resolving or validating credentials.
pub fn load_for_show(profile_arg: Option<&str>) -> ConfigSummary {
    let path = config_path();
    let file_exists = path.exists();

    let active_profile = profile_arg
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| env_var("ZOOM_PROFILE"))
        .unwrap_or_else(|| "default".to_owned());

    let profiles = read_all_profiles(&path);

    let mut env_overrides = Vec::new();
    for var in ["ZOOM_ACCOUNT_ID", "ZOOM_CLIENT_ID", "ZOOM_CLIENT_SECRET"] {
        if let Some(v) = normalize(std::env::var(var).ok()) {
            env_overrides.push((var, v));
        }
    }

    ConfigSummary {
        config_file: path,
        file_exists,
        active_profile,
        profiles,
        env_overrides,
    }
}

/// Read the raw credential values for a specific profile, for use when updating.
///
/// Returns `None` if the config file does not exist, cannot be parsed, or does
/// not contain the requested profile with all three credentials present.
pub fn read_profile_credentials(
    path: &Path,
    profile_name: &str,
) -> Option<(String, String, String)> {
    let content = std::fs::read_to_string(path).ok()?;
    let raw: RawConfig = toml::from_str(&content).ok()?;

    let p = if profile_name == "default" {
        raw.default
    } else {
        raw.profiles.get(profile_name)?.clone()
    };

    Some((
        normalize(p.account_id)?,
        normalize(p.client_id)?,
        normalize(p.client_secret)?,
    ))
}

fn read_all_profiles(path: &Path) -> Vec<ProfileSummary> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let raw: RawConfig = match toml::from_str(&content) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut profiles = Vec::new();

    // "default" is deserialized into the dedicated field, not the flatten map.
    if raw.default.account_id.is_some()
        || raw.default.client_id.is_some()
        || raw.default.client_secret.is_some()
    {
        profiles.push(ProfileSummary {
            name: "default".to_owned(),
            account_id: raw.default.account_id,
            client_id: raw.default.client_id,
            client_secret: raw.default.client_secret,
        });
    }

    // BTreeMap iteration is already in alphabetical order.
    for (name, p) in raw.profiles {
        profiles.push(ProfileSummary {
            name,
            account_id: p.account_id,
            client_id: p.client_id,
            client_secret: p.client_secret,
        });
    }

    profiles
}

pub fn config_path() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("zoom-cli")
        .join("config.toml")
}

fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
    }
}

fn load_file_profile(profile: Option<&str>) -> Result<RawProfile, ApiError> {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(RawProfile::default()),
        Err(e) => return Err(ApiError::Other(format!("Failed to read config: {e}"))),
    };

    let raw: RawConfig = toml::from_str(&content)
        .map_err(|e| ApiError::Other(format!("Failed to parse config: {e}")))?;

    let profile_name = profile
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| env_var("ZOOM_PROFILE"));

    match profile_name {
        None => Ok(raw.default),
        Some(name) if name == "default" => Ok(raw.default),
        Some(name) => {
            let available: Vec<&str> = raw.profiles.keys().map(String::as_str).collect();
            raw.profiles.get(&name).cloned().ok_or_else(|| {
                ApiError::Other(format!(
                    "Profile '{name}' not found. Available: {}",
                    if available.is_empty() {
                        "none".to_owned()
                    } else {
                        available.join(", ")
                    }
                ))
            })
        }
    }
}

fn env_var(name: &str) -> Option<String> {
    normalize(std::env::var(name).ok())
}

fn normalize(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let t = v.trim().to_owned();
        if t.is_empty() { None } else { Some(t) }
    })
}

/// Write (or overwrite) a single profile in the config file, preserving other profiles.
///
/// Creates the config directory and file if they don't exist, then sets
/// permissions to 0600 on unix.
pub fn write_profile(
    path: &Path,
    profile_name: &str,
    account_id: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<(), ApiError> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(ApiError::Other(format!("Failed to read config: {e}"))),
    };

    let mut table: toml::Table = if content.trim().is_empty() {
        toml::Table::new()
    } else {
        toml::from_str(&content)
            .map_err(|e| ApiError::Other(format!("Failed to parse config: {e}")))?
    };

    let mut profile = toml::Table::new();
    profile.insert(
        "account_id".into(),
        toml::Value::String(account_id.to_owned()),
    );
    profile.insert(
        "client_id".into(),
        toml::Value::String(client_id.to_owned()),
    );
    profile.insert(
        "client_secret".into(),
        toml::Value::String(client_secret.to_owned()),
    );
    table.insert(profile_name.to_owned(), toml::Value::Table(profile));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ApiError::Other(format!("Cannot create config directory: {e}")))?;
    }

    let serialized = toml::to_string_pretty(&table)
        .map_err(|e| ApiError::Other(format!("Failed to serialize config: {e}")))?;
    // On Unix, create the file with mode 0o600 in a single syscall so there is
    // no window between creation (with a permissive umask) and chmod.
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| ApiError::Other(format!("Failed to write config: {e}")))?;
        file.write_all(serialized.as_bytes())
            .map_err(|e| ApiError::Other(format!("Failed to write config: {e}")))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, serialized)
            .map_err(|e| ApiError::Other(format!("Failed to write config: {e}")))?;
    }

    Ok(())
}

fn write_config_file(path: &Path, content: &str) -> Result<(), ApiError> {
    use std::io::Write;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| ApiError::Other(format!("Cannot write config: {e}")))?;
        file.write_all(content.as_bytes())
            .map_err(|e| ApiError::Other(format!("Write error: {e}")))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, content.as_bytes())
            .map_err(|e| ApiError::Other(format!("Cannot write config: {e}")))?;
    }
    Ok(())
}

/// Remove a named profile from the config file.
///
/// Returns `Ok(())` if removed, `Err(ApiError::NotFound)` if the profile
/// doesn't exist, and `Err(ApiError::Other(...))` for IO/parse failures.
pub fn delete_profile(path: &Path, profile_name: &str) -> Result<(), ApiError> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ApiError::NotFound(format!(
                "Config file not found: {}",
                path.display()
            )));
        }
        Err(e) => return Err(ApiError::Other(format!("Failed to read config: {e}"))),
    };

    let mut table: toml::Table =
        toml::from_str(&content).map_err(|e| ApiError::Other(format!("Invalid config: {e}")))?;

    // Both "default" and named profiles are stored as top-level TOML keys.
    let existed = table.remove(profile_name).is_some();

    if !existed {
        return Err(ApiError::NotFound(format!(
            "Profile '{}' not found.",
            profile_name
        )));
    }

    let new_content = toml::to_string_pretty(&table)
        .map_err(|e| ApiError::Other(format!("Failed to serialize config: {e}")))?;

    write_config_file(path, &new_content)?;
    Ok(())
}

pub fn schema_config_path_description() -> &'static str {
    #[cfg(not(target_os = "windows"))]
    {
        "~/.config/zoom-cli/config.toml (or $XDG_CONFIG_HOME/zoom-cli/config.toml)"
    }
    #[cfg(target_os = "windows")]
    {
        "%APPDATA%\\zoom-cli\\config.toml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{EnvVarGuard, ProcessEnvLock, set_config_dir_env, write_config};
    use tempfile::TempDir;

    fn clear_zoom_env() -> (EnvVarGuard, EnvVarGuard, EnvVarGuard, EnvVarGuard) {
        (
            EnvVarGuard::unset("ZOOM_ACCOUNT_ID"),
            EnvVarGuard::unset("ZOOM_CLIENT_ID"),
            EnvVarGuard::unset("ZOOM_CLIENT_SECRET"),
            EnvVarGuard::unset("ZOOM_PROFILE"),
        )
    }

    #[test]
    fn load_reads_default_profile_from_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"
[default]
account_id = "acct-001"
client_id = "cid-001"
client_secret = "csec-001"
"#,
        )
        .unwrap();

        let _cfg_dir = set_config_dir_env(dir.path());
        let _env = clear_zoom_env();

        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.account_id, "acct-001");
        assert_eq!(cfg.client_id, "cid-001");
        assert_eq!(cfg.client_secret, "csec-001");
    }

    #[test]
    fn load_env_vars_override_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"
[default]
account_id = "file-account"
client_id = "file-client"
client_secret = "file-secret"
"#,
        )
        .unwrap();

        let _cfg_dir = set_config_dir_env(dir.path());
        let _acct = EnvVarGuard::set("ZOOM_ACCOUNT_ID", "env-account");
        let _cid = EnvVarGuard::unset("ZOOM_CLIENT_ID");
        let _csec = EnvVarGuard::unset("ZOOM_CLIENT_SECRET");
        let _prof = EnvVarGuard::unset("ZOOM_PROFILE");

        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.account_id, "env-account", "env var must win over file");
        assert_eq!(
            cfg.client_id, "file-client",
            "file value used when env absent"
        );
    }

    #[test]
    fn load_blank_env_vars_fall_back_to_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"
[default]
account_id = "acct"
client_id = "cid"
client_secret = "csec"
"#,
        )
        .unwrap();

        let _cfg_dir = set_config_dir_env(dir.path());
        let _acct = EnvVarGuard::set("ZOOM_ACCOUNT_ID", "   ");
        let _cid = EnvVarGuard::set("ZOOM_CLIENT_ID", "");
        let _csec = EnvVarGuard::unset("ZOOM_CLIENT_SECRET");
        let _prof = EnvVarGuard::unset("ZOOM_PROFILE");

        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.account_id, "acct");
        assert_eq!(cfg.client_id, "cid");
    }

    #[test]
    fn load_missing_credentials_returns_error() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _cfg_dir = set_config_dir_env(dir.path());
        let _env = clear_zoom_env();

        let err = Config::load(None).unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput(_)));
        assert!(err.to_string().contains("account_id"));
    }

    #[test]
    fn load_named_profile_from_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"
[default]
account_id = "def-acct"
client_id = "def-cid"
client_secret = "def-csec"

[work]
account_id = "work-acct"
client_id = "work-cid"
client_secret = "work-csec"
"#,
        )
        .unwrap();

        let _cfg_dir = set_config_dir_env(dir.path());
        let _env = clear_zoom_env();

        let cfg = Config::load(Some("work".into())).unwrap();
        assert_eq!(cfg.account_id, "work-acct");
        assert_eq!(cfg.client_id, "work-cid");
    }

    #[test]
    fn load_zoom_profile_env_selects_named_profile() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"
[default]
account_id = "def-acct"
client_id = "def-cid"
client_secret = "def-csec"

[staging]
account_id = "staging-acct"
client_id = "staging-cid"
client_secret = "staging-csec"
"#,
        )
        .unwrap();

        let _cfg_dir = set_config_dir_env(dir.path());
        let _acct = EnvVarGuard::unset("ZOOM_ACCOUNT_ID");
        let _cid = EnvVarGuard::unset("ZOOM_CLIENT_ID");
        let _csec = EnvVarGuard::unset("ZOOM_CLIENT_SECRET");
        let _prof = EnvVarGuard::set("ZOOM_PROFILE", "staging");

        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.account_id, "staging-acct");
    }

    #[test]
    fn load_unknown_profile_returns_descriptive_error() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"
[work]
account_id = "w-acct"
client_id = "w-cid"
client_secret = "w-csec"
"#,
        )
        .unwrap();

        let _cfg_dir = set_config_dir_env(dir.path());
        let _env = clear_zoom_env();

        let err = Config::load(Some("nonexistent".into())).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("nonexistent"));
        assert!(msg.contains("work"), "error should list available profiles");
    }

    #[test]
    fn load_invalid_toml_returns_error() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "account_id = [invalid").unwrap();

        let _cfg_dir = set_config_dir_env(dir.path());
        let _env = clear_zoom_env();

        let err = Config::load(None).unwrap_err();
        assert!(matches!(err, ApiError::Other(_)));
        assert!(err.to_string().contains("parse"));
    }

    #[test]
    fn missing_config_file_yields_informative_missing_field_error() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _cfg_dir = set_config_dir_env(dir.path());
        let _env = clear_zoom_env();

        let err = Config::load(None).unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput(_)));
    }
}
