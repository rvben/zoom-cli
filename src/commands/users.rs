use crate::api::{ApiError, ZoomClient};
use crate::output::OutputConfig;

pub async fn list(_client: &mut ZoomClient, _out: &OutputConfig, _status: Option<&str>) -> Result<(), ApiError> { todo!() }
pub async fn get(_client: &mut ZoomClient, _out: &OutputConfig, _id_or_email: &str) -> Result<(), ApiError> { todo!() }
pub async fn me(_client: &mut ZoomClient, _out: &OutputConfig) -> Result<(), ApiError> { todo!() }
