use crate::api::{ApiError, ZoomClient};
use crate::output::OutputConfig;

pub async fn list(_client: &mut ZoomClient, _out: &OutputConfig, _user: &str, _meeting_type: Option<&str>) -> Result<(), ApiError> { todo!() }
pub async fn get(_client: &mut ZoomClient, _out: &OutputConfig, _id: u64) -> Result<(), ApiError> { todo!() }
pub async fn create(_client: &mut ZoomClient, _out: &OutputConfig, _topic: String, _duration: Option<u32>, _start: Option<String>, _password: Option<String>) -> Result<(), ApiError> { todo!() }
pub async fn update(_client: &mut ZoomClient, _out: &OutputConfig, _id: u64, _topic: Option<String>, _duration: Option<u32>, _start: Option<String>) -> Result<(), ApiError> { todo!() }
pub async fn delete(_client: &mut ZoomClient, _out: &OutputConfig, _id: u64) -> Result<(), ApiError> { todo!() }
