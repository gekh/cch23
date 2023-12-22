use axum::{extract::Path, routing::get, Router};
use log::info;
use reqwest::StatusCode;

pub fn get_routes() -> Router {
    Router::new().route("/1/*key", get(exclusive_cube))
}

async fn exclusive_cube(Path(params): Path<Vec<(String, String)>>) -> Result<String, StatusCode> {
    info!("1 started");
    let nums = params[0]
        .1
        .split('/')
        .map(|s| s.parse::<i32>().unwrap())
        .collect::<Vec<i32>>();

    let out = nums[1..nums.len()].iter().fold(nums[0], |acc, x| acc ^ x);
    let out = out.pow(3);

    Ok(out.to_string())
}
