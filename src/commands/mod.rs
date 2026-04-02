pub mod meetings;
pub mod recordings;
pub mod users;

use crate::output::OutputConfig;

pub fn schema(resource: &str, out: &OutputConfig) {
    let content = match resource {
        "meetings" => serde_json::json!({
            "resource": "meetings",
            "commands": {
                "list": {
                    "description": "List meetings for a user",
                    "flags": {
                        "--user": "User ID or 'me' (default: me)",
                        "--type": "Filter: scheduled|live|upcoming"
                    }
                },
                "get": {
                    "description": "Get a meeting by numeric ID",
                    "args": { "id": "Meeting ID (u64)" }
                },
                "create": {
                    "description": "Create a meeting",
                    "flags": {
                        "--topic": "(required) Meeting topic",
                        "--duration": "Duration in minutes",
                        "--start": "Start time (ISO 8601, e.g. 2026-04-03T09:00:00Z)",
                        "--password": "Meeting password"
                    }
                },
                "update": {
                    "description": "Update a meeting",
                    "args": { "id": "Meeting ID" },
                    "flags": {
                        "--topic": "New topic",
                        "--duration": "New duration in minutes",
                        "--start": "New start time (ISO 8601)"
                    }
                },
                "delete": {
                    "description": "Delete a meeting",
                    "args": { "id": "Meeting ID" }
                }
            },
            "fields": {
                "id": "u64 — meeting ID",
                "topic": "string",
                "start_time": "string — ISO 8601",
                "duration": "u32 — minutes",
                "join_url": "string",
                "start_url": "string",
                "status": "string — waiting|started|finished",
                "password": "string",
                "meeting_type": "u8 — 1=instant 2=scheduled"
            }
        }),
        "recordings" => serde_json::json!({
            "resource": "recordings",
            "commands": {
                "list": {
                    "description": "List cloud recordings for a user",
                    "flags": {
                        "--user": "User ID or 'me' (default: me)",
                        "--from": "Start date (YYYY-MM-DD)",
                        "--to": "End date (YYYY-MM-DD)"
                    }
                },
                "get": {
                    "description": "Get recording details for a meeting",
                    "args": { "meeting_id": "Meeting ID or UUID" }
                },
                "download": {
                    "description": "Download recording files to disk",
                    "args": { "meeting_id": "Meeting ID or UUID" },
                    "flags": { "--out": "Output directory (default: .)" }
                }
            },
            "fields": {
                "id": "string — meeting UUID",
                "topic": "string",
                "start_time": "string — ISO 8601",
                "duration": "u32 — minutes",
                "total_size": "u64 — bytes",
                "recording_files": "array of RecordingFile",
                "recording_file.file_type": "string — MP4|M4A|CHAT|TRANSCRIPT|TIMELINE",
                "recording_file.file_size": "u64 — bytes",
                "recording_file.download_url": "string",
                "recording_file.status": "string — completed|processing"
            }
        }),
        "users" => serde_json::json!({
            "resource": "users",
            "commands": {
                "list": {
                    "description": "List users in the account",
                    "flags": {
                        "--status": "Filter: active|inactive|pending"
                    }
                },
                "get": {
                    "description": "Get a user by ID or email address",
                    "args": { "id_or_email": "User ID or email" }
                },
                "me": {
                    "description": "Get the current authenticated user"
                }
            },
            "fields": {
                "id": "string — user ID",
                "email": "string",
                "display_name": "string",
                "first_name": "string",
                "last_name": "string",
                "status": "string — active|inactive|pending",
                "user_type": "u8 — 1=Basic 2=Licensed 3=On-Prem"
            }
        }),
        _ => {
            eprintln!("Unknown resource '{resource}'. Available: meetings, recordings, users");
            std::process::exit(1);
        }
    };

    out.print_data(&serde_json::to_string_pretty(&content).expect("serialize"));
}
