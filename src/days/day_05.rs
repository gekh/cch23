use axum::{extract::Query, routing::post, Json, Router};
use log::info;
use reqwest::StatusCode;

pub fn get_routes() -> Router {
    Router::new().route("/5", post(slicing_the_loop))
}

#[derive(serde::Deserialize, Debug, Default)]
struct Pagination {
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    split: usize,
}

async fn slicing_the_loop(
    pagination: Query<Pagination>,
    Json(names): Json<Vec<String>>,
) -> Result<String, (StatusCode, String)> {
    info!("5 started");
    let start = if let Some(start) = pagination.offset {
        if start > names.len() {
            names.len()
        } else {
            start
        }
    } else {
        0
    };

    let end = if let Some(limit) = pagination.limit {
        if start + limit > names.len() {
            names.len()
        } else {
            start + limit
        }
    } else {
        names.len()
    };

    if pagination.split == 0 {
        let out = serde_json::to_string(&names[start..end].to_vec()).unwrap();
        Ok(out)
    } else {
        let chunks = names[start..end]
            .chunks(pagination.split)
            .map(|chunk| chunk.to_vec())
            .collect::<Vec<_>>();
        let out = serde_json::to_string(&chunks).unwrap();
        Ok(out)
    }
}
