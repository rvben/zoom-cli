use crate::api::{ApiError, ZoomClient};
use crate::output::OutputConfig;

pub async fn list(_client: &mut ZoomClient, _out: &OutputConfig, _user: &str, _from: Option<&str>, _to: Option<&str>) -> Result<(), ApiError> { todo!() }
pub async fn get(_client: &mut ZoomClient, _out: &OutputConfig, _meeting_id: &str) -> Result<(), ApiError> { todo!() }
pub async fn download(_client: &mut ZoomClient, _out: &OutputConfig, _meeting_id: &str, _out_dir: &str) -> Result<(), ApiError> { todo!() }
