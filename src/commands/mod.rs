pub mod config;
pub mod init;
pub mod meetings;
pub mod recordings;
pub mod reports;
pub mod users;
pub mod webinars;

fn schema_doc() -> serde_json::Value {
    serde_json::json!({
        "clispec": "0.2",
        "name": "zoom",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "CLI for the Zoom API",
        "global_args": [
            {
                "name": "--output",
                "type": "string",
                "description": "Output format: auto (default), text, json",
                "default": "auto"
            },
            {
                "name": "--quiet",
                "type": "boolean",
                "description": "Suppress non-data output"
            },
            {
                "name": "--profile",
                "type": "string",
                "description": "Config profile to use"
            },
            {
                "name": "--json",
                "type": "boolean",
                "description": "Output as JSON (alias for --output=json)"
            }
        ],
        "commands": [
            {
                "name": "meetings list",
                "description": "List meetings for a user",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "integer"},
                    {"name": "topic", "type": "string"},
                    {"name": "start_time", "type": "string"},
                    {"name": "duration", "type": "integer"},
                    {"name": "join_url", "type": "string"},
                    {"name": "status", "type": "string"}
                ],
                "args": [
                    {"name": "--user", "type": "string", "default": "me", "description": "User ID or 'me'"},
                    {"name": "--type", "type": "string", "description": "Filter: scheduled|live|upcoming"},
                    {"name": "--limit", "type": "integer", "description": "Maximum number of results"},
                    {"name": "--offset", "type": "integer", "description": "Number of results to skip"},
                    {"name": "--fields", "type": "string[]", "description": "Fields to include in output"}
                ]
            },
            {
                "name": "meetings get",
                "description": "Get a meeting by ID",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "integer"},
                    {"name": "topic", "type": "string"},
                    {"name": "start_time", "type": "string"},
                    {"name": "duration", "type": "integer"},
                    {"name": "join_url", "type": "string"},
                    {"name": "start_url", "type": "string"},
                    {"name": "status", "type": "string"},
                    {"name": "password", "type": "string"}
                ],
                "args": [
                    {"name": "id", "type": "integer", "required": true, "description": "Meeting ID"}
                ]
            },
            {
                "name": "meetings create",
                "description": "Create a meeting",
                "mutating": true,
                "output_fields": [
                    {"name": "id", "type": "integer"},
                    {"name": "topic", "type": "string"},
                    {"name": "join_url", "type": "string"},
                    {"name": "start_url", "type": "string"}
                ],
                "args": [
                    {"name": "--topic", "type": "string", "required": true, "description": "Meeting topic"},
                    {"name": "--duration", "type": "integer", "description": "Duration in minutes"},
                    {"name": "--start", "type": "string", "description": "Start time (ISO 8601)"},
                    {"name": "--password", "type": "string", "description": "Meeting password"}
                ]
            },
            {
                "name": "meetings update",
                "description": "Update a meeting",
                "mutating": true,
                "args": [
                    {"name": "id", "type": "integer", "required": true, "description": "Meeting ID"},
                    {"name": "--topic", "type": "string", "description": "New topic"},
                    {"name": "--duration", "type": "integer", "description": "New duration in minutes"},
                    {"name": "--start", "type": "string", "description": "New start time (ISO 8601)"}
                ]
            },
            {
                "name": "meetings delete",
                "description": "Delete a meeting",
                "mutating": true,
                "args": [
                    {"name": "id", "type": "integer", "required": true, "description": "Meeting ID"},
                    {"name": "--yes", "type": "boolean", "default": false, "description": "Skip confirmation prompt"}
                ]
            },
            {
                "name": "meetings end",
                "description": "End a live meeting",
                "mutating": true,
                "args": [
                    {"name": "id", "type": "integer", "required": true, "description": "Meeting ID"}
                ]
            },
            {
                "name": "meetings participants",
                "description": "List participants from a past meeting",
                "mutating": false,
                "output_fields": [
                    {"name": "name", "type": "string"},
                    {"name": "user_email", "type": "string"},
                    {"name": "join_time", "type": "string"},
                    {"name": "leave_time", "type": "string"},
                    {"name": "duration", "type": "integer"}
                ],
                "args": [
                    {"name": "meeting_id", "type": "string", "required": true, "description": "Meeting ID or UUID"}
                ]
            },
            {
                "name": "meetings invite",
                "description": "Get meeting invitation text",
                "mutating": false,
                "output_fields": [
                    {"name": "invitation", "type": "string"}
                ],
                "args": [
                    {"name": "id", "type": "integer", "required": true, "description": "Meeting ID"}
                ]
            },
            {
                "name": "recordings list",
                "description": "List cloud recordings for a user",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "integer"},
                    {"name": "topic", "type": "string"},
                    {"name": "start_time", "type": "string"},
                    {"name": "duration", "type": "integer"},
                    {"name": "total_size", "type": "integer"},
                    {"name": "recording_files", "type": "array"}
                ],
                "args": [
                    {"name": "--user", "type": "string", "default": "me", "description": "User ID or 'me'"},
                    {"name": "--from", "type": "string", "description": "Start date (YYYY-MM-DD)"},
                    {"name": "--to", "type": "string", "description": "End date (YYYY-MM-DD)"},
                    {"name": "--limit", "type": "integer", "description": "Maximum number of results"},
                    {"name": "--offset", "type": "integer", "description": "Number of results to skip"},
                    {"name": "--fields", "type": "string[]", "description": "Fields to include in output"}
                ]
            },
            {
                "name": "recordings get",
                "description": "Get recording details for a meeting",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "integer"},
                    {"name": "topic", "type": "string"},
                    {"name": "start_time", "type": "string"},
                    {"name": "duration", "type": "integer"},
                    {"name": "total_size", "type": "integer"},
                    {"name": "recording_files", "type": "array"}
                ],
                "args": [
                    {"name": "meeting_id", "type": "string", "required": true, "description": "Meeting ID or UUID"}
                ]
            },
            {
                "name": "recordings download",
                "description": "Download recording files for a meeting",
                "mutating": false,
                "args": [
                    {"name": "meeting_id", "type": "string", "required": true, "description": "Meeting ID or UUID"},
                    {"name": "--out", "type": "path", "default": ".", "description": "Output directory"}
                ]
            },
            {
                "name": "recordings delete",
                "description": "Delete all cloud recordings for a meeting",
                "mutating": true,
                "args": [
                    {"name": "meeting_id", "type": "string", "required": true, "description": "Meeting ID or UUID"},
                    {"name": "--permanent", "type": "boolean", "default": false, "description": "Permanently delete instead of moving to trash"},
                    {"name": "--yes", "type": "boolean", "default": false, "description": "Skip confirmation prompt"}
                ]
            },
            {
                "name": "recordings start",
                "description": "Start cloud recording for a live meeting",
                "mutating": true,
                "args": [
                    {"name": "meeting_id", "type": "integer", "required": true, "description": "Numeric meeting ID of the live meeting"}
                ]
            },
            {
                "name": "recordings stop",
                "description": "Stop cloud recording for a live meeting",
                "mutating": true,
                "args": [
                    {"name": "meeting_id", "type": "integer", "required": true, "description": "Numeric meeting ID of the live meeting"}
                ]
            },
            {
                "name": "recordings pause",
                "description": "Pause cloud recording for a live meeting",
                "mutating": true,
                "args": [
                    {"name": "meeting_id", "type": "integer", "required": true, "description": "Numeric meeting ID of the live meeting"}
                ]
            },
            {
                "name": "recordings resume",
                "description": "Resume cloud recording for a live meeting",
                "mutating": true,
                "args": [
                    {"name": "meeting_id", "type": "integer", "required": true, "description": "Numeric meeting ID of the live meeting"}
                ]
            },
            {
                "name": "recordings transcript",
                "description": "Download transcript files (VTT/chat) for a meeting",
                "mutating": false,
                "args": [
                    {"name": "meeting_id", "type": "string", "required": true, "description": "Meeting ID or UUID"},
                    {"name": "--out", "type": "path", "default": ".", "description": "Output directory"}
                ]
            },
            {
                "name": "users list",
                "description": "List users in the account",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "string"},
                    {"name": "email", "type": "string"},
                    {"name": "display_name", "type": "string"},
                    {"name": "first_name", "type": "string"},
                    {"name": "last_name", "type": "string"},
                    {"name": "status", "type": "string"},
                    {"name": "type", "type": "integer"}
                ],
                "args": [
                    {"name": "--status", "type": "string", "description": "Filter: active|inactive|pending"},
                    {"name": "--limit", "type": "integer", "description": "Maximum number of results"},
                    {"name": "--offset", "type": "integer", "description": "Number of results to skip"},
                    {"name": "--fields", "type": "string[]", "description": "Fields to include in output"}
                ]
            },
            {
                "name": "users get",
                "description": "Get a user by ID or email",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "string"},
                    {"name": "email", "type": "string"},
                    {"name": "display_name", "type": "string"},
                    {"name": "first_name", "type": "string"},
                    {"name": "last_name", "type": "string"},
                    {"name": "status", "type": "string"},
                    {"name": "type", "type": "integer"}
                ],
                "args": [
                    {"name": "id_or_email", "type": "string", "required": true, "description": "User ID or email"}
                ]
            },
            {
                "name": "users me",
                "description": "Get the current user",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "string"},
                    {"name": "email", "type": "string"},
                    {"name": "display_name", "type": "string"},
                    {"name": "first_name", "type": "string"},
                    {"name": "last_name", "type": "string"},
                    {"name": "status", "type": "string"},
                    {"name": "type", "type": "integer"}
                ]
            },
            {
                "name": "users create",
                "description": "Create a new user",
                "mutating": true,
                "output_fields": [
                    {"name": "id", "type": "string"},
                    {"name": "email", "type": "string"},
                    {"name": "first_name", "type": "string"},
                    {"name": "last_name", "type": "string"},
                    {"name": "status", "type": "string"}
                ],
                "args": [
                    {"name": "--email", "type": "string", "required": true, "description": "User email"},
                    {"name": "--first-name", "type": "string", "description": "First name"},
                    {"name": "--last-name", "type": "string", "description": "Last name"},
                    {"name": "--type", "type": "integer", "default": 1, "description": "User type: 1=Basic, 2=Licensed, 3=On-prem"}
                ]
            },
            {
                "name": "users deactivate",
                "description": "Deactivate a user",
                "mutating": true,
                "args": [
                    {"name": "id_or_email", "type": "string", "required": true, "description": "User ID or email"}
                ]
            },
            {
                "name": "users activate",
                "description": "Activate (reactivate) a user",
                "mutating": true,
                "args": [
                    {"name": "id_or_email", "type": "string", "required": true, "description": "User ID or email"}
                ]
            },
            {
                "name": "webinars list",
                "description": "List webinars for a user",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "integer"},
                    {"name": "topic", "type": "string"},
                    {"name": "start_time", "type": "string"},
                    {"name": "duration", "type": "integer"},
                    {"name": "join_url", "type": "string"},
                    {"name": "status", "type": "string"}
                ],
                "args": [
                    {"name": "--user", "type": "string", "default": "me", "description": "User ID or 'me'"},
                    {"name": "--limit", "type": "integer", "description": "Maximum number of results"},
                    {"name": "--offset", "type": "integer", "description": "Number of results to skip"},
                    {"name": "--fields", "type": "string[]", "description": "Fields to include in output"}
                ]
            },
            {
                "name": "webinars get",
                "description": "Get a webinar by ID",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "integer"},
                    {"name": "topic", "type": "string"},
                    {"name": "start_time", "type": "string"},
                    {"name": "duration", "type": "integer"},
                    {"name": "join_url", "type": "string"},
                    {"name": "status", "type": "string"},
                    {"name": "agenda", "type": "string"}
                ],
                "args": [
                    {"name": "id", "type": "integer", "required": true, "description": "Webinar ID"}
                ]
            },
            {
                "name": "reports meetings",
                "description": "Meeting summary report for a user",
                "mutating": false,
                "output_fields": [
                    {"name": "id", "type": "integer"},
                    {"name": "uuid", "type": "string"},
                    {"name": "topic", "type": "string"},
                    {"name": "start_time", "type": "string"},
                    {"name": "duration", "type": "integer"},
                    {"name": "participants_count", "type": "integer"},
                    {"name": "total_minutes", "type": "integer"}
                ],
                "args": [
                    {"name": "--user", "type": "string", "default": "me", "description": "User ID or 'me'"},
                    {"name": "--from", "type": "string", "required": true, "description": "Start date (YYYY-MM-DD)"},
                    {"name": "--to", "type": "string", "description": "End date (YYYY-MM-DD, default: today)"},
                    {"name": "--limit", "type": "integer", "description": "Maximum number of results"},
                    {"name": "--offset", "type": "integer", "description": "Number of results to skip"},
                    {"name": "--fields", "type": "string[]", "description": "Fields to include in output"}
                ]
            },
            {
                "name": "reports participants",
                "description": "Participant report for a past meeting",
                "mutating": false,
                "output_fields": [
                    {"name": "name", "type": "string"},
                    {"name": "user_email", "type": "string"},
                    {"name": "join_time", "type": "string"},
                    {"name": "leave_time", "type": "string"},
                    {"name": "duration", "type": "integer"}
                ],
                "args": [
                    {"name": "meeting_id", "type": "string", "required": true, "description": "Meeting ID or UUID"}
                ]
            },
            {
                "name": "config show",
                "description": "Show current configuration",
                "mutating": false,
                "output_fields": [
                    {"name": "profiles", "type": "array"},
                    {"name": "active_profile", "type": "string"},
                    {"name": "env_overrides", "type": "object"}
                ]
            },
            {
                "name": "config delete",
                "description": "Delete a profile from the config file",
                "mutating": true,
                "args": [
                    {"name": "profile", "type": "string", "required": true, "description": "Profile name to delete"},
                    {"name": "--force", "type": "boolean", "default": false, "description": "Skip confirmation prompt"}
                ]
            },
            {
                "name": "init",
                "description": "Set up credentials interactively",
                "mutating": true,
                "args": [
                    {"name": "--profile", "type": "string", "description": "Profile name to create or update"}
                ]
            },
            {
                "name": "schema",
                "description": "Print machine-readable clispec v0.2 schema",
                "mutating": false
            },
            {
                "name": "completions",
                "description": "Generate shell completions",
                "mutating": false,
                "args": [
                    {"name": "shell", "type": "string", "required": true, "description": "Shell to generate completions for"}
                ]
            }
        ],
        "errors": [
            {"kind": "auth_error", "exit_code": 2, "retryable": false, "description": "Authentication failed or forbidden"},
            {"kind": "not_found", "exit_code": 3, "retryable": false, "description": "Resource not found"},
            {"kind": "invalid_input", "exit_code": 2, "retryable": false, "description": "Invalid user input or missing config"},
            {"kind": "rate_limit", "exit_code": 1, "retryable": true, "description": "Rate limited by Zoom API"},
            {"kind": "api_error", "exit_code": 1, "retryable": false, "description": "Non-2xx response from Zoom API"},
            {"kind": "http_error", "exit_code": 1, "retryable": true, "description": "Network or TLS error"},
            {"kind": "error", "exit_code": 1, "retryable": false, "description": "Unexpected error"},
            {"kind": "confirmation_required", "exit_code": 2, "retryable": false, "description": "Destructive operation requires explicit confirmation"},
            {"kind": "conflict", "exit_code": 1, "retryable": false, "description": "Resource already exists or state conflict"}
        ]
    })
}

pub fn schema() {
    println!(
        "{}",
        serde_json::to_string_pretty(&schema_doc()).expect("serialize")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonschema;

    #[test]
    fn schema_output_is_valid_json_with_required_fields() {
        let doc = schema_doc();
        assert_eq!(doc["clispec"], "0.2");
        assert_eq!(doc["name"], "zoom");
        assert!(!doc["version"].as_str().unwrap_or("").is_empty());
        assert!(doc["commands"].as_array().unwrap().len() > 0);
        let errors = doc["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .all(|e| e["kind"].is_string() && e["exit_code"].is_number())
        );
    }

    #[test]
    fn schema_output_validates_against_clispec_v0_2_json_schema() {
        let schema_json = include_str!("../../tests/fixtures/clispec-v0.2.json");
        let meta_schema: serde_json::Value =
            serde_json::from_str(schema_json).expect("fixture must be valid JSON");
        let validator =
            jsonschema::validator_for(&meta_schema).expect("clispec JSON Schema must compile");
        let doc = schema_doc();
        let errors: Vec<String> = validator.iter_errors(&doc).map(|e| e.to_string()).collect();
        assert!(
            errors.is_empty(),
            "schema output does not validate against clispec v0.2:\n{}",
            errors.join("\n")
        );
    }

    #[test]
    fn schema_errors_include_conflict_kind() {
        let doc = schema_doc();
        let errors = doc["errors"].as_array().unwrap();
        let has_conflict = errors.iter().any(|e| e["kind"] == "conflict");
        assert!(has_conflict, "errors must include 'conflict' kind");
    }

    #[test]
    fn schema_all_commands_have_mutating_field() {
        let doc = schema_doc();
        if let Some(cmds) = doc["commands"].as_array() {
            for cmd in cmds {
                assert!(
                    cmd["mutating"].is_boolean(),
                    "command '{}' must have a boolean mutating field",
                    cmd["name"].as_str().unwrap_or("?")
                );
            }
        }
    }
}
