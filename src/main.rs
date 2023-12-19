use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Utc};
use cookie::Cookie;
use image::{io::Reader as ImageReader, GenericImageView, Pixel};
use itertools::Itertools;
use log::info;
use num_traits::PrimInt;
use regex::Regex;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc, time::SystemTime};
use std::{io::Cursor, sync::Mutex};
use tower_http::services::ServeDir;

async fn ok() -> Result<String, StatusCode> {
    Ok(String::from("okay"))
}

async fn error() -> Result<String, StatusCode> {
    info!("-1 started");
    Err(StatusCode::INTERNAL_SERVER_ERROR)
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

#[derive(serde::Deserialize, Debug)]
struct Reindeer {
    strength: u32,
}

async fn strength(Json(reindeers): Json<Vec<Reindeer>>) -> Result<String, StatusCode> {
    info!("4 strength started");
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
    info!("4 contest started");
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
    info!("6 started");
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

async fn red_pixels(mut multipart: Multipart) -> Result<String, StatusCode> {
    info!("11 started");
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

struct AppState {
    data: Arc<Mutex<HashMap<String, SystemTime>>>,
}

async fn save_string(
    Path(s): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<(), StatusCode> {
    info!("12 save started");
    let mut data = state.data.lock().expect("mutex was poisoned");
    data.insert(s.clone(), SystemTime::now());

    Ok(())
}

async fn load_string(
    Path(s): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<String, StatusCode> {
    info!("12 load started");
    let data = state.data.lock().expect("mutex was poisoned");
    let now = SystemTime::now();
    let t = data.get(&s).unwrap();

    Ok(now.duration_since(*t).unwrap().as_secs().to_string())
}

async fn ulids(Json(ulid_strings): Json<Vec<String>>) -> Result<String, StatusCode> {
    info!("12 ulids started");
    let uuid_strings = ulid_strings
        .into_iter()
        .map(|ulid_string| {
            let ulid_bytes = ulid::Ulid::from_string(&ulid_string).unwrap().to_bytes();
            uuid::Uuid::from_bytes(ulid_bytes).to_string()
        })
        .rev()
        .collect::<Vec<String>>();

    Ok(format!("{:?}", uuid_strings))
}

#[derive(serde::Serialize, Debug, Clone, Default)]
struct UlidStats {
    #[serde(rename = "christmas eve")]
    christmas_eve: u16,
    weekday: u16,
    #[serde(rename = "in the future")]
    in_the_future: u16,
    #[serde(rename = "LSB is 1")]
    lsb_is_1: u16,
}

pub fn get_lsb<N: PrimInt>(n: N) -> N {
    n & N::one()
}

async fn ulids_weekday(
    Path(expected_weekday): Path<u8>,
    Json(ulid_strings): Json<Vec<String>>,
) -> Result<Json<UlidStats>, StatusCode> {
    info!("12 ulids weekday started");
    let mut ulid_stats = UlidStats::default();
    for ulid_string in ulid_strings.into_iter() {
        let ulid = ulid::Ulid::from_string(&ulid_string).unwrap();
        let datetime: DateTime<Utc> = ulid.datetime().into();
        let day_month = datetime.format("%d-%m").to_string();
        let weekday = datetime.format("%u").to_string().parse::<u8>().unwrap() - 1;

        ulid_stats.christmas_eve += (day_month == "24-12") as u16;
        ulid_stats.weekday += (weekday == expected_weekday) as u16;
        ulid_stats.in_the_future += (ulid.datetime() > SystemTime::now()) as u16;
        ulid_stats.lsb_is_1 += (get_lsb(ulid.to_bytes()[15]) == 1) as u16;
    }

    Ok(ulid_stats.into())
}

#[derive(Clone)]
struct DbState {
    pool: PgPool,
}

async fn sql(State(state): State<DbState>) -> Result<String, StatusCode> {
    info!("13 sql started");
    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(2023_12_13_i64)
        .fetch_one(&state.pool)
        .await
        .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(row.0.to_string())
}

async fn reset(State(state): State<DbState>) -> Result<String, StatusCode> {
    info!("13 reset started");

    let sql = "DROP TABLE IF EXISTS orders";
    sqlx::query(sql)
        .execute(&state.pool)
        .await
        .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

    let sql = "CREATE TABLE orders (
            id INT PRIMARY KEY,
            region_id INT,
            gift_name VARCHAR(50),
            quantity INT
          )";
    sqlx::query(sql)
        .execute(&state.pool)
        .await
        .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok("db's reset".to_string())
}

#[derive(serde::Deserialize, Debug)]
struct Order {
    id: i64,
    region_id: i64,
    gift_name: String,
    quantity: i64,
}

async fn orders(
    State(state): State<DbState>,
    Json(orders): Json<Vec<Order>>,
) -> Result<String, StatusCode> {
    info!("13 orders started");
    let sql = "INSERT INTO orders (id, region_id, gift_name, quantity) VALUES ($1, $2, $3, $4)";
    for order in orders {
        sqlx::query(sql)
            .bind(order.id)
            .bind(order.region_id)
            .bind(order.gift_name)
            .bind(order.quantity)
            .execute(&state.pool)
            .await
            .map_err(|err| {
                dbg!(&err);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    Ok("OK".to_string())
}

async fn orders_total(State(state): State<DbState>) -> Result<String, StatusCode> {
    info!("13 orders total started");
    let sql = "SELECT SUM(quantity) FROM orders";
    let row: (i64,) = sqlx::query_as(sql)
        .fetch_one(&state.pool)
        .await
        .map_err(|err| {
            dbg!(&err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(format!("{{\"total\": {}}}", row.0.to_string()))
}

async fn orders_popular(State(state): State<DbState>) -> Result<String, StatusCode> {
    info!("13 orders popular started");
    let sql = "SELECT gift_name FROM orders GROUP BY gift_name ORDER BY SUM(quantity) DESC LIMIT 1";
    match sqlx::query_as::<_, (String,)>(sql)
        .fetch_one(&state.pool)
        .await
    {
        Ok(row) => Ok(format!("{{\"popular\": \"{}\"}}", row.0.to_string())),
        Err(err) => match err {
            sqlx::Error::RowNotFound => Ok("{\"popular\": null}".to_string()),
            err => {
                dbg!(&err);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
    }
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
struct HtmlContent {
    content: String,
}

async fn html_unsafe(Json(body): Json<HtmlContent>) -> Result<String, StatusCode> {
    info!("14 html unsafe started");

    Ok(format!(
        "\
<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    {}
  </body>
</html>",
        body.content
    )
    .to_string())
}

async fn html_safe(Json(body): Json<HtmlContent>) -> Result<String, StatusCode> {
    info!("14 html safe started");

    Ok(format!(
        "\
<html>
  <head>
    <title>CCH23 Day 14</title>
  </head>
  <body>
    {}
  </body>
</html>",
        html_escape::encode_double_quoted_attribute(body.content.as_str())
    )
    .to_string())
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://postgres:{secrets.PASSWORD}@localhost:5432/cch23"
    )]
    pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    let shared_state = Arc::new(AppState {
        data: Arc::new(Mutex::new(HashMap::new())),
    });

    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(|err| shuttle_runtime::Error::Database(err.to_string()))?;

    let db_state = DbState { pool };

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
        .route("/11/red_pixels", post(red_pixels))
        .route("/12/save/:string", post(save_string))
        .route("/12/load/:string", get(load_string))
        .with_state(shared_state)
        .route("/12/ulids", post(ulids))
        .route("/12/ulids/:weekday", post(ulids_weekday))
        .route("/13/sql", get(sql))
        .route("/13/reset", post(reset))
        .route("/13/orders", post(orders))
        .route("/13/orders/total", get(orders_total))
        .route("/13/orders/popular", get(orders_popular))
        .with_state(db_state)
        .route("/14/unsafe", post(html_unsafe))
        .route("/14/safe", post(html_safe));

    Ok(router.into())
}
