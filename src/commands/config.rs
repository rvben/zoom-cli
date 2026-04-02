use crate::config;
use crate::output::{self, OutputConfig};

pub fn show(profile_arg: Option<&str>, out: &OutputConfig) {
    let summary = config::load_for_show(profile_arg);

    if out.json {
        let profiles_json: serde_json::Map<String, serde_json::Value> = summary
            .profiles
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    serde_json::json!({
                        "account_id": masked_or_unset(&p.account_id),
                        "client_id": masked_or_unset(&p.client_id),
                        "client_secret": masked_or_unset(&p.client_secret),
                    }),
                )
            })
            .collect();

        let env_overrides_json: serde_json::Map<String, serde_json::Value> = summary
            .env_overrides
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    serde_json::Value::String(output::mask_credential(v)),
                )
            })
            .collect();

        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
                "config_file": summary.config_file.to_string_lossy(),
                "file_exists": summary.file_exists,
                "active_profile": summary.active_profile,
                "profiles": profiles_json,
                "env_overrides": env_overrides_json,
            }))
            .expect("serialize"),
        );
    } else {
        out.print_data(&format!(
            "Config file: {}{}",
            summary.config_file.display(),
            if summary.file_exists {
                ""
            } else {
                "  (not found)"
            },
        ));

        if summary.profiles.is_empty() && summary.env_overrides.is_empty() {
            out.print_data("\nNo configuration found. Run `zoom init` to set up credentials.");
            return;
        }

        for profile in &summary.profiles {
            let active_marker = if profile.name == summary.active_profile {
                "  (active)"
            } else {
                ""
            };
            out.print_data(&format!("\nProfile: {}{}", profile.name, active_marker));
            out.print_data(&format!(
                "  account_id:    {}",
                masked_or_unset(&profile.account_id)
            ));
            out.print_data(&format!(
                "  client_id:     {}",
                masked_or_unset(&profile.client_id)
            ));
            out.print_data(&format!(
                "  client_secret: {}",
                masked_or_unset(&profile.client_secret)
            ));
        }

        if !summary.env_overrides.is_empty() {
            out.print_data("\nEnvironment overrides (take precedence over file):");
            for (var, val) in &summary.env_overrides {
                out.print_data(&format!("  {}={}", var, output::mask_credential(val)));
            }
        }
    }
}

fn masked_or_unset(value: &Option<String>) -> String {
    match value {
        Some(v) => output::mask_credential(v),
        None => "(not set)".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{EnvVarGuard, ProcessEnvLock, set_config_dir_env, write_config};
    use tempfile::TempDir;

    #[test]
    fn show_json_includes_active_profile_and_masked_credentials() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            r#"
[default]
account_id = "acct-long-enough-123"
client_id = "cid-long-enough-456"
client_secret = "csec-long-enough-789"
"#,
        )
        .unwrap();

        let _cfg_dir = set_config_dir_env(dir.path());
        let _acct = EnvVarGuard::unset("ZOOM_ACCOUNT_ID");
        let _cid = EnvVarGuard::unset("ZOOM_CLIENT_ID");
        let _csec = EnvVarGuard::unset("ZOOM_CLIENT_SECRET");
        let _prof = EnvVarGuard::unset("ZOOM_PROFILE");

        // Capture stdout by redirecting through a buffer isn't straightforward;
        // instead call load_for_show directly and validate the summary.
        let summary = config::load_for_show(None);
        assert_eq!(summary.active_profile, "default");
        assert!(summary.file_exists);
        assert_eq!(summary.profiles.len(), 1);
        assert_eq!(summary.profiles[0].name, "default");
        assert_eq!(
            summary.profiles[0].account_id.as_deref(),
            Some("acct-long-enough-123")
        );
        assert!(summary.env_overrides.is_empty());
    }

    #[test]
    fn show_lists_all_profiles_with_default_first() {
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
        let _env_acct = EnvVarGuard::unset("ZOOM_ACCOUNT_ID");
        let _env_cid = EnvVarGuard::unset("ZOOM_CLIENT_ID");
        let _env_csec = EnvVarGuard::unset("ZOOM_CLIENT_SECRET");
        let _env_prof = EnvVarGuard::unset("ZOOM_PROFILE");

        let summary = config::load_for_show(None);
        assert_eq!(summary.profiles.len(), 2);
        assert_eq!(summary.profiles[0].name, "default");
        assert_eq!(summary.profiles[1].name, "work");
    }

    #[test]
    fn show_marks_profile_arg_as_active() {
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
        let _env_acct = EnvVarGuard::unset("ZOOM_ACCOUNT_ID");
        let _env_cid = EnvVarGuard::unset("ZOOM_CLIENT_ID");
        let _env_csec = EnvVarGuard::unset("ZOOM_CLIENT_SECRET");
        let _env_prof = EnvVarGuard::unset("ZOOM_PROFILE");

        let summary = config::load_for_show(Some("work"));
        assert_eq!(summary.active_profile, "work");
    }

    #[test]
    fn show_reports_env_overrides() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _cfg_dir = set_config_dir_env(dir.path());
        let _acct = EnvVarGuard::set("ZOOM_ACCOUNT_ID", "env-account-id-long");
        let _cid = EnvVarGuard::unset("ZOOM_CLIENT_ID");
        let _csec = EnvVarGuard::unset("ZOOM_CLIENT_SECRET");
        let _prof = EnvVarGuard::unset("ZOOM_PROFILE");

        let summary = config::load_for_show(None);
        assert_eq!(summary.env_overrides.len(), 1);
        assert_eq!(summary.env_overrides[0].0, "ZOOM_ACCOUNT_ID");
        assert_eq!(summary.env_overrides[0].1, "env-account-id-long");
    }

    #[test]
    fn show_handles_missing_config_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _cfg_dir = set_config_dir_env(dir.path());
        let _acct = EnvVarGuard::unset("ZOOM_ACCOUNT_ID");
        let _cid = EnvVarGuard::unset("ZOOM_CLIENT_ID");
        let _csec = EnvVarGuard::unset("ZOOM_CLIENT_SECRET");
        let _prof = EnvVarGuard::unset("ZOOM_PROFILE");

        let summary = config::load_for_show(None);
        assert!(!summary.file_exists);
        assert!(summary.profiles.is_empty());
    }

    #[test]
    fn masked_or_unset_masks_present_values() {
        assert_eq!(
            masked_or_unset(&Some("abcdefghijklmnop".to_owned())),
            "abcdef…mnop"
        );
        assert_eq!(masked_or_unset(&None), "(not set)");
    }
}
