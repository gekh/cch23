use axum::{routing::get, Router};
use log::info;
use reqwest::StatusCode;

pub fn get_routes() -> Router {
    Router::new().route("/-1/error", get(error))
}

async fn error() -> Result<String, StatusCode> {
    info!("-1 started");
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}
