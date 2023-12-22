use axum::{extract::Path, routing::get, Router};
use log::info;
use reqwest::StatusCode;

pub fn get_routes() -> Router {
    Router::new()
        .route("/8/weight/:pokedex_number", get(weight))
        .route("/8/drop/:pokedex_number", get(drop))
}

#[derive(serde::Deserialize, Debug)]
struct PokeApi {
    weight: f64,
}

async fn weight(Path(pokedex_number): Path<String>) -> Result<String, StatusCode> {
    info!("8 weight started");
    let body = reqwest::get(format!(
        "https://pokeapi.co/api/v2/pokemon/{}",
        pokedex_number
    ))
    .await
    .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?
    .text()
    .await
    .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut input: PokeApi = serde_json::from_str(body.as_str()).unwrap();
    input.weight /= 10.;

    Ok(input.weight.to_string())
}

async fn drop(Path(pokedex_number): Path<String>) -> Result<String, StatusCode> {
    info!("8 drop started");
    let body = reqwest::get(format!(
        "https://pokeapi.co/api/v2/pokemon/{}",
        pokedex_number
    ))
    .await
    .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?
    .text()
    .await
    .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

    let input: PokeApi = serde_json::from_str(body.as_str()).unwrap();
    let m = input.weight / 10.;
    let a = 9.825;
    let x = 10.;
    let out = m * a * (2. * x / a).sqrt();

    Ok(out.to_string())
}
