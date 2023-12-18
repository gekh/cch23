use axum::{
    extract::{Multipart, Path},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose, Engine};
use cookie::Cookie;
use image::{io::Reader as ImageReader, GenericImageView, Pixel};
use itertools::Itertools;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Cursor;
use tower_http::services::ServeDir;

async fn ok() -> Result<String, StatusCode> {
    Ok(String::from(""))
}

async fn error() -> Result<String, StatusCode> {
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn exclusive_cube(Path(params): Path<Vec<(String, String)>>) -> Result<String, StatusCode> {
    let nums = params[0]
        .1
        .split('/')
        .map(|s| s.parse::<i32>().unwrap())
        .collect::<Vec<i32>>();

    let out = nums[1..nums.len()].iter().fold(nums[0], |acc, x| acc ^ x);
    let out = out.pow(3);

    Ok(out.to_string())
}

#[derive(serde::Deserialize, Debug)]
struct Reindeer {
    strength: u32,
}

async fn strength(Json(reindeers): Json<Vec<Reindeer>>) -> Result<String, StatusCode> {
    let mut sum = 0;

    for reindeer in reindeers {
        sum += reindeer.strength
    }

    Ok(sum.to_string())
}

#[derive(serde::Deserialize, Debug, Clone)]
struct Champion {
    name: String,
    strength: u32,
    speed: f32,
    height: u32,
    antler_width: u32,
    snow_magic_power: u32,
    favorite_food: String,
    #[serde(rename = "cAnD13s_3ATeN-yesT3rdAy")]
    candies: u32,
}

async fn contest(Json(champions): Json<Vec<Champion>>) -> Result<String, StatusCode> {
    let mut fastest = champions[0].clone();
    let mut tallest = champions[0].clone();
    let mut magician = champions[0].clone();
    let mut consumer = champions[0].clone();

    for champ in champions {
        if champ.speed > fastest.speed {
            fastest = champ.clone();
        }
        if champ.height > tallest.height {
            tallest = champ.clone();
        }
        if champ.snow_magic_power > magician.snow_magic_power {
            magician = champ.clone();
        }
        if champ.candies > consumer.candies {
            consumer = champ.clone();
        }
    }

    Ok(format!(
        "{{
    \"fastest\": \"Speeding past the finish line with a strength of {} is {}\",
    \"tallest\": \"{} is standing tall with his {} cm wide antlers\",
    \"magician\": \"{} could blast you away with a snow magic power of {}\",
    \"consumer\": \"{} ate lots of candies, but also some {}\"
}}",
        fastest.strength,
        fastest.name,
        tallest.name,
        tallest.antler_width,
        magician.name,
        magician.snow_magic_power,
        consumer.name,
        consumer.favorite_food
    ))
}

#[derive(serde::Serialize, Debug)]
struct ElfCount {
    elf: usize,
    #[serde(rename = "elf on a shelf")]
    elf_on_a_shelf: usize,
    #[serde(rename = "shelf with no elf on it")]
    no_elf: usize,
}

async fn elf_count(body: String) -> Result<Json<ElfCount>, StatusCode> {
    let elf = body.matches("elf").count();

    let mut elf_on_a_shelf = 0;
    let re = Regex::new(r"(elf( on a shelf)+)").unwrap();
    for cap in re.captures_iter(body.as_str()) {
        elf_on_a_shelf += cap[1].matches(" on a shelf").count();
    }

    let no_elf = body.matches("shelf").count() - elf_on_a_shelf;

    Ok(ElfCount {
        elf,
        elf_on_a_shelf,
        no_elf,
    }
    .into())
}

async fn decode(headers: HeaderMap) -> Result<String, StatusCode> {
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

#[derive(serde::Deserialize, Debug)]
struct PokeApi {
    weight: f64,
}

async fn weight(Path(pokedex_number): Path<String>) -> Result<String, StatusCode> {
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
    println!("input = {:?}", input);
    input.weight /= 10.;

    Ok(input.weight.to_string())
}

async fn drop(Path(pokedex_number): Path<String>) -> Result<String, StatusCode> {
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

async fn red_pixels(mut multipart: Multipart) -> Result<String, StatusCode> {
    let mut out = 0;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?
            .decode()
            .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

        for (_, _, rgba) in img.pixels() {
            let (&r, &g, &b, _) = rgba.channels().into_iter().collect_tuple().unwrap();

            if r.saturating_sub(g).saturating_sub(b) > 0 {
                out += 1;
            }
        }
    }

    Ok(out.to_string())
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(ok))
        .route("/-1/error", get(error))
        .route("/1/*key", get(exclusive_cube))
        .route("/4/strength", post(strength))
        .route("/4/contest", post(contest))
        .route("/6", post(elf_count))
        .route("/7/decode", get(decode))
        .route("/7/bake", get(bake))
        .route("/8/weight/:pokedex_number", get(weight))
        .route("/8/drop/:pokedex_number", get(drop))
        .nest_service("/11/assets", ServeDir::new("assets"))
        .route("/11/red_pixels", post(red_pixels));

    Ok(router.into())
}
