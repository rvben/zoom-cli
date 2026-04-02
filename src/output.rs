use std::io::IsTerminal;

use crate::api::ApiError;

pub fn use_color() -> bool {
    std::io::stdout().is_terminal()
}

/// Format a URL as a clickable OSC 8 hyperlink in terminals that support it.
pub fn hyperlink(url: &str) -> String {
    if use_color() {
        format!("\x1b]8;;{url}\x1b\\{url}\x1b]8;;\x1b\\")
    } else {
        url.to_string()
    }
}

#[derive(Clone, Copy)]
pub struct OutputConfig {
    pub json: bool,
    pub quiet: bool,
}

impl OutputConfig {
    pub fn new(json_flag: bool, quiet: bool) -> Self {
        let json = json_flag || !std::io::stdout().is_terminal();
        Self { json, quiet }
    }

    /// Print data to stdout (tables, JSON, or single values). Always shown.
    pub fn print_data(&self, data: &str) {
        println!("{data}");
    }

    /// Print an informational message to stderr. Suppressed by --quiet.
    pub fn print_message(&self, msg: &str) {
        if !self.quiet {
            eprintln!("{msg}");
        }
    }

    /// Print the result of a mutation (create/update/delete).
    ///
    /// JSON mode: prints structured JSON to stdout.
    /// Human mode: prints the human message to stdout so callers can capture it.
    pub fn print_result(&self, json_value: &serde_json::Value, human_message: &str) {
        if self.json {
            println!(
                "{}",
                serde_json::to_string_pretty(json_value).expect("failed to serialize JSON")
            );
        } else {
            println!("{human_message}");
        }
    }
}

/// Exit codes for agent-friendly error handling.
pub mod exit_codes {
    use super::ApiError;

    pub const SUCCESS: i32 = 0;
    /// General / unexpected error.
    pub const GENERAL_ERROR: i32 = 1;
    /// Config or auth error (missing credentials, bad profile).
    pub const CONFIG_ERROR: i32 = 2;
    /// Resource not found.
    pub const NOT_FOUND: i32 = 3;

    pub fn for_error(e: &ApiError) -> i32 {
        match e {
            ApiError::Auth(_) | ApiError::InvalidInput(_) => CONFIG_ERROR,
            ApiError::NotFound(_) => NOT_FOUND,
            _ => GENERAL_ERROR,
        }
    }
}

/// Format an ISO 8601 UTC timestamp for human display.
///
/// `"2026-03-29T07:34:19Z"` → `"2026-03-29 07:34"`
/// Strings that don't match the pattern (e.g. `"-"`) are returned unchanged.
pub fn format_timestamp(ts: &str) -> String {
    let inner = ts.strip_suffix('Z').unwrap_or(ts);
    if let Some((date, time)) = inner.split_once('T') {
        let hm = time.get(..5).unwrap_or(time);
        return format!("{date} {hm}");
    }
    ts.to_string()
}

/// Render a simple two-column key/value block for single-resource output.
pub fn kv_block(pairs: &[(&str, String)]) -> String {
    let max_key = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    pairs
        .iter()
        .map(|(k, v)| format!("{:width$}  {}", k, v, width = max_key))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a simple table with a header row and data rows.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let col_count = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:width$}", h, width = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");

    let sep: String = widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("  ");

    let data_lines: Vec<String> = rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .take(col_count)
                .map(|(i, cell)| format!("{:width$}", cell, width = widths[i]))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect();

    let mut out = vec![header_line, sep];
    out.extend(data_lines);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_block_aligns_keys() {
        let pairs = [("id", "123".into()), ("topic", "Standup".into())];
        let out = kv_block(&pairs);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        let id_pos = lines[0].find("123").unwrap();
        let topic_pos = lines[1].find("Standup").unwrap();
        assert_eq!(id_pos, topic_pos, "values must be column-aligned");
    }

    #[test]
    fn table_renders_header_and_separator() {
        let headers = ["ID", "TOPIC", "DURATION"];
        let rows = vec![
            vec!["111".into(), "Standup".into(), "15".into()],
            vec!["222".into(), "All Hands".into(), "60".into()],
        ];
        let out = table(&headers, &rows);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].contains("ID"));
        assert!(lines[0].contains("TOPIC"));
        assert!(lines[1].contains("---"));
        assert!(lines[2].contains("Standup"));
        assert!(lines[3].contains("All Hands"));
    }

    #[test]
    fn table_pads_to_widest_cell() {
        let headers = ["NAME"];
        let rows = vec![vec!["short".into()], vec!["much longer name".into()]];
        let out = table(&headers, &rows);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[1].len() >= "much longer name".len());
    }

    #[test]
    fn format_timestamp_formats_iso8601() {
        assert_eq!(format_timestamp("2026-03-29T07:34:19Z"), "2026-03-29 07:34");
        assert_eq!(format_timestamp("2020-04-06T17:15:00Z"), "2020-04-06 17:15");
    }

    #[test]
    fn format_timestamp_passes_through_non_timestamps() {
        assert_eq!(format_timestamp("-"), "-");
        assert_eq!(format_timestamp(""), "");
    }

    #[test]
    fn exit_codes_for_error_maps_correctly() {
        assert_eq!(
            exit_codes::for_error(&ApiError::Auth("x".into())),
            exit_codes::CONFIG_ERROR
        );
        assert_eq!(
            exit_codes::for_error(&ApiError::NotFound("x".into())),
            exit_codes::NOT_FOUND
        );
        assert_eq!(
            exit_codes::for_error(&ApiError::RateLimit),
            exit_codes::GENERAL_ERROR
        );
    }
}
