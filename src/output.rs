use crate::api::ApiError;

pub fn use_color() -> bool { false }

pub fn hyperlink(url: &str) -> String { url.to_string() }

#[derive(Clone, Copy)]
pub struct OutputConfig {
    pub json: bool,
    pub quiet: bool,
}

impl OutputConfig {
    pub fn new(json_flag: bool, _quiet: bool) -> Self {
        Self { json: json_flag, quiet: _quiet }
    }
    pub fn print_data(&self, data: &str) { println!("{data}"); }
    pub fn print_message(&self, msg: &str) { if !self.quiet { eprintln!("{msg}"); } }
    pub fn print_result(&self, _json: &serde_json::Value, human: &str) { println!("{human}"); }
}

pub mod exit_codes {
    use super::ApiError;
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const CONFIG_ERROR: i32 = 2;
    pub const NOT_FOUND: i32 = 3;
    pub fn for_error(e: &ApiError) -> i32 {
        match e {
            ApiError::Auth(_) | ApiError::InvalidInput(_) => CONFIG_ERROR,
            ApiError::NotFound(_) => NOT_FOUND,
            _ => GENERAL_ERROR,
        }
    }
}

pub fn kv_block(_pairs: &[(&str, String)]) -> String { String::new() }
pub fn table(_headers: &[&str], _rows: &[Vec<String>]) -> String { String::new() }
