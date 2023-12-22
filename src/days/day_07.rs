use axum::{http::HeaderMap, routing::get, Json, Router};
use base64::{engine::general_purpose, Engine};
use cookie::Cookie;
use log::info;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn get_routes() -> Router {
    Router::new()
        .route("/7/decode", get(decode))
        .route("/7/bake", get(bake))
}

async fn decode(headers: HeaderMap) -> Result<String, StatusCode> {
    info!("7 decode started");
    let header_cookies = headers["cookie"].to_str().unwrap();
    let c = Cookie::parse(header_cookies).unwrap();
    let encoded = c.value();
    let decoded = general_purpose::STANDARD.decode(encoded).unwrap();
    let s = std::str::from_utf8(decoded.as_slice()).unwrap();

    Ok(s.to_string())
}

#[derive(serde::Deserialize, Debug)]
struct RecipeInput {
    recipe: HashMap<String, Value>,
    pantry: HashMap<String, Value>,
}

#[derive(serde::Serialize, Debug)]
struct RecipeOutput {
    cookies: u64,
    pantry: HashMap<String, Value>,
}

async fn bake(headers: HeaderMap) -> Result<Json<RecipeOutput>, StatusCode> {
    info!("7 bake started");
    let header_cookies = headers["cookie"].to_str().unwrap();
    let c = Cookie::parse(header_cookies).unwrap();
    let encoded = c.value();
    let decoded = general_purpose::STANDARD.decode(encoded).unwrap();
    let json = std::str::from_utf8(decoded.as_slice()).unwrap();
    let input: RecipeInput = serde_json::from_str(json).unwrap();
    let mut cookies = u64::MAX;

    for (k, v) in input.recipe.iter() {
        let v = v.as_u64().unwrap();

        if v > 0 {
            if let Some(pantry_value) = input.pantry.get(k) {
                let pantry_value = pantry_value.as_u64().unwrap();
                cookies = cookies.min(pantry_value / v);
            } else {
                cookies = 0;
            }
        }
    }

    let mut out = RecipeOutput {
        cookies,
        pantry: input.pantry,
    };

    for (k, v) in input.recipe.iter() {
        let v = v.as_u64().unwrap();

        if v > 0 {
            if let Some(pantry_value) = out.pantry.get_mut(k) {
                let pantry_value_u = pantry_value.as_u64().unwrap();
                *pantry_value = json!(pantry_value_u - cookies * v);
            }
        }
    }

    Ok(out.into())
}
