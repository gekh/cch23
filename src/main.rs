mod countries;

use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket},
        Multipart, Path, Query, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Utc};
use cookie::Cookie;
use dms_coordinates::DMS;
use futures::{sink::SinkExt, stream::StreamExt};
use git2::{Repository, Tree};
use image::{io::Reader as ImageReader, GenericImageView, Pixel};
use itertools::Itertools;
use log::info;
use num_traits::PrimInt;
use regex::Regex;
use reverse_geocoder::ReverseGeocoder;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::SystemTime,
};
use std::{io::Cursor, sync::Mutex};
use tar::Archive;
use tempfile::tempdir;
use tokio::sync::broadcast;
use tower_http::services::ServeDir;
use unicode_segmentation::UnicodeSegmentation;

use crate::countries::countries;

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

async fn elf_count(body: String) -> Result<Json<ElfCount>, StatusCode> {
    info!("6 started");
    let elf = body.matches("elf").count();

    let mut elf_on_a_shelf = 0;
    let re = Regex::new(r"(elf( on a shelf)+)").unwrap();
    for cap in re.captures_iter(body.as_str()) {
        elf_on_a_shelf += cap[1].matches(" on a shelf").count();
    }

    let no_elf = body.matches("shelf").count() - elf_on_a_shelf;

    Ok(Json(ElfCount {
        elf,
        elf_on_a_shelf,
        no_elf,
    }))
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
    info!("13/18 reset started");

    let sqls = vec![
        "DROP TABLE IF EXISTS regions;",
        "DROP TABLE IF EXISTS orders;",
        "CREATE TABLE regions ( id INT PRIMARY KEY, name VARCHAR(50) );",
        "CREATE TABLE orders (
            id INT PRIMARY KEY,
            region_id INT,
            gift_name VARCHAR(50),
            quantity INT
        );",
    ];

    for sql in sqls {
        sqlx::query(sql)
            .execute(&state.pool)
            .await
            .map_err(|_err| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

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
    info!("13/18 orders started");
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

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct RegionsTotalRow {
    region: String,
    total: i64,
}

async fn regions_total(
    State(state): State<DbState>,
) -> Result<Json<Vec<RegionsTotalRow>>, StatusCode> {
    info!("18 regions total started");
    let sql = "SELECT r.name as region, sum(o.quantity) as total
        FROM orders o
        INNER JOIN regions r ON o.region_id = r.id
        GROUP BY r.name
        ORDER BY r.name;
    ";

    match sqlx::query_as::<_, RegionsTotalRow>(sql)
        .fetch_all(&state.pool)
        .await
    {
        Ok(rows) => Ok(rows.into()),
        Err(err) => {
            dbg!(err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct RegionsTopRow {
    region: String,
    top_gifts: sqlx::types::Json<Vec<String>>,
}

async fn regions_top_list(
    Path(limit): Path<usize>,
    State(state): State<DbState>,
) -> Result<Json<Vec<RegionsTopRow>>, StatusCode> {
    info!("18 regions top list started");
    let sql = r#"
        WITH grouped_orders AS (
            SELECT region_id, gift_name, SUM(quantity) as q
            FROM orders
            GROUP BY region_id, gift_name
            ORDER BY q DESC, gift_name ASC
        )

        SELECT r.name as region,
        CASE WHEN COUNT(gift_name) = 0
            THEN '[]'
            ELSE json_agg(gift_name)
            END as top_gifts
        FROM regions r
        LEFT JOIN grouped_orders go ON r.id = go.region_id
        GROUP BY r.name
        ORDER BY r.name;
    "#;

    match sqlx::query_as::<_, RegionsTopRow>(sql)
        .fetch_all(&state.pool)
        .await
    {
        Ok(mut rows) => {
            rows = rows
                .into_iter()
                .map(|mut r| {
                    r.top_gifts.0 = r.top_gifts.0.into_iter().take(limit).collect();
                    r
                })
                .collect();
            Ok(rows.into())
        }
        Err(err) => {
            dbg!(err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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

#[derive(serde::Deserialize, Debug)]
struct Region {
    id: i64,
    name: String,
}

async fn regions(
    State(state): State<DbState>,
    Json(regions): Json<Vec<Region>>,
) -> Result<String, StatusCode> {
    info!("18 regions started");
    let sql = "INSERT INTO regions (id, name) VALUES ($1, $2);";
    for r in regions {
        sqlx::query(sql)
            .bind(r.id)
            .bind(r.name)
            .execute(&state.pool)
            .await
            .map_err(|err| {
                dbg!(&err);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    Ok("OK".to_string())
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

#[derive(serde::Deserialize, Debug, Clone, Default)]
struct Password {
    input: String,
}

async fn nice(Json(password): Json<Password>) -> Result<(StatusCode, String), StatusCode> {
    info!("15 nice started");

    let re_vowels = Regex::new(r"[aeiouy]").unwrap();
    let re_doubles = Regex::new(r"(ab|cd|pq|xy)").unwrap();
    if re_vowels.find_iter(password.input.as_str()).count() < 3
        || !has_two_consecutive_chars(password.input.as_str())
        || re_doubles.find(password.input.as_str()) != None
    {
        return Ok((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\"}".to_string(),
        ));
    }

    Ok((StatusCode::OK, "{\"result\":\"nice\"}".to_string()))
}

async fn game(Json(password): Json<Password>) -> Result<String, (StatusCode, String)> {
    info!("15 game started");

    let s = password.input;

    // Rule 1: must be at least 8 characters long
    if s.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\", \"reason\":\"8 chars\"}".to_string(),
        ));
    }

    // Rule 2: must contain uppercase letters, lowercase letters, and digits
    let re_upper = Regex::new(r"[A-Z]").unwrap();
    let re_lower = Regex::new(r"[a-z]").unwrap();
    let re_digits = Regex::new(r"[\d]").unwrap();

    if re_upper.find(s.as_str()) == None
        || re_lower.find(s.as_str()) == None
        || re_digits.find(s.as_str()) == None
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\", \"reason\":\"more types of chars\"}".to_string(),
        ));
    }

    // Rule 3: must contain at least 5 digits
    if re_digits.find_iter(s.as_str()).count() < 5 {
        return Err((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\", \"reason\":\"55555\"}".to_string(),
        ));
    }

    // Rule 4: all integers (sequences of consecutive digits) in the string must add up to 2023
    let re_numbers = Regex::new(r"[\d]+").unwrap();
    let mut sum = 0;
    for number in re_numbers.find_iter(s.as_str()) {
        sum += number.as_str().parse::<u32>().unwrap();
    }

    if sum != 2023 {
        return Err((
            StatusCode::BAD_REQUEST,
            "{\"result\":\"naughty\", \"reason\":\"math is hard\"}".to_string(),
        ));
    }

    // Rule 5: must contain the letters j, o, and y in that order and in no other order
    let mut joy = (false, false, false);
    let re_joy = Regex::new(r"[joy]").unwrap();
    for c in s.chars() {
        if joy == (false, false, false) && c == 'j' {
            joy.0 = true;
        } else if joy == (true, false, false) && c == 'o' {
            joy.1 = true;
        } else if joy == (true, true, false) && c == 'y' {
            joy.2 = true;
        }
    }
    if joy != (true, true, true) || re_joy.find_iter(s.as_str()).count() != 3 {
        return Err((
            StatusCode::NOT_ACCEPTABLE,
            "{\"result\":\"naughty\", \"reason\":\"not joyful enough\"}".to_string(),
        ));
    }

    // Rule 6: must contain a letter that repeats with exactly one other letter between them (like xyx)
    let mut rule6 = false;
    let graphemes = s
        .graphemes(true)
        .map(|g| g.chars().fold(0, |acc, c| acc + c as u32))
        .collect::<Vec<u32>>();
    for i in 2..graphemes.len() {
        let (g1, g2, g3) = (graphemes[i - 2], graphemes[i - 1], graphemes[i]);
        if g1 == g3
            && (g1 >= b'a' as u32 && g1 <= b'z' as u32 || g1 >= b'A' as u32 && g1 <= b'Z' as u32)
            && (g2 >= b'a' as u32 && g2 <= b'z' as u32 || g2 >= b'A' as u32 && g2 <= b'Z' as u32)
        {
            rule6 = true;
        }
    }
    if !rule6 {
        return Err((
            StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
            "{\"result\":\"naughty\", \"reason\":\"illegal: no sandwich\"}".to_string(),
        ));
    }

    // Rule 7: must contain at least one unicode character in the range [U+2980, U+2BFF]
    let left = 0x2980;
    let right = 0x2BFF;
    let mut rule7 = false;
    for g in s.graphemes(true) {
        let gn = g.chars().fold(0, |acc, c| acc + c as u32);
        if gn >= left && gn <= right {
            rule7 = true;
        }
    }
    if !rule7 {
        return Err((
            StatusCode::RANGE_NOT_SATISFIABLE,
            "{\"result\":\"naughty\", \"reason\":\"outranged\"}".to_string(),
        ));
    }

    // Rule 8: must contain at least one emoji
    let left = 0x1F600;
    let right = 0x1F977;
    let mut rule8 = false;
    for g in s.graphemes(true) {
        let gn = g.chars().fold(0, |acc, c| acc + c as u32);
        if gn >= left && gn <= right {
            rule8 = true;
        }
    }
    if !rule8 {
        return Err((
            StatusCode::UPGRADE_REQUIRED,
            "{\"result\":\"naughty\", \"reason\":\"😳\"}".to_string(),
        ));
    }

    // Rule 9: the hexadecimal representation of the sha256 hash of the string must end with an a
    let hash = sha256::digest(s.to_string());
    if hash.chars().last().unwrap() != 'a' {
        return Err((
            StatusCode::IM_A_TEAPOT,
            "{\"result\":\"naughty\", \"reason\":\"not a coffee brewer\"}".to_string(),
        ));
    }

    Ok("{\"result\":\"nice\", \"reason\":\"that's a nice password\"}".to_string())
}

fn has_two_consecutive_chars(s: &str) -> bool {
    let mut chars = s.chars();
    let mut prev = chars.next().unwrap();
    for c in chars {
        if c == prev && c.is_alphabetic() {
            return true;
        }
        prev = c;
    }
    false
}

async fn ws_ping(ws: WebSocketUpgrade) -> Response {
    info!("19 ws ping started");
    ws.on_upgrade(|socket| ws_ping_socket(socket))
}

async fn ws_ping_socket(mut socket: WebSocket) {
    let mut game_started = false;
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            // client disconnected
            return;
        };

        match msg {
            axum::extract::ws::Message::Text(text) => {
                if text == "ping" && game_started {
                    if let Err(err) = socket.send(Message::Text("pong".to_string())).await {
                        print!("Error: {:?}", err);
                    }
                } else if text == "serve" {
                    game_started = true;
                }
            }
            _ => {}
        }
    }
}

struct TweeterState {
    views: Arc<Mutex<u32>>,
    // We require unique usernames. This tracks which usernames have been taken.
    user_set: Mutex<HashSet<String>>,
    // Channels used to send messages to all connected clients.
    rooms: Mutex<HashMap<usize, broadcast::Sender<String>>>,
}

#[derive(serde::Deserialize, Debug)]
struct UserMsgIn {
    message: String,
}

#[derive(serde::Serialize, Debug)]
struct UserMsgOut {
    user: String,
    message: String,
}

async fn tweeter_reset(
    State(state): State<Arc<TweeterState>>,
) -> Result<String, (StatusCode, String)> {
    info!("19 tweeter reset started");
    let mut views = state.views.lock().expect("mutex was poisoned");
    *views = 0;

    Ok("OK".to_string())
}

async fn tweeter_views(
    State(state): State<Arc<TweeterState>>,
) -> Result<String, (StatusCode, String)> {
    info!("19 tweeter views started");
    let views = state.views.lock().expect("mutex was poisoned");
    info!("views: {views}");

    Ok(views.to_string())
}

async fn tweeter_ws_handler(
    Path((room_number, username)): Path<(usize, String)>,
    ws: WebSocketUpgrade,
    State(state): State<Arc<TweeterState>>,
) -> Response {
    info!("19 tweeter ws started");
    ws.on_upgrade(move |socket| tweeter_ws(socket, state, username, room_number))
}

async fn tweeter_ws(
    stream: WebSocket,
    state: Arc<TweeterState>,
    username: String,
    room_number: usize,
) {
    let (mut sender, mut receiver) = stream.split();

    if !check_username(&state, &username) {
        info!("Username {username} is already taken.");
        // Only send our client that username is taken.
        let _ = sender
            .send(Message::Text(String::from("Username already taken.")))
            .await;

        return;
    }

    info!("User {username} joined room {room_number}.");

    let room_tx = find_room(&state, &room_number);
    // We subscribe *before* sending the "joined" message, so that we will also
    // display it to our client.
    let mut rx = room_tx.subscribe();
    let views = state.views.clone();

    // Spawn the first task that will receive broadcast messages and send text
    // messages over the websocket to our client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            *views.lock().unwrap() += 1;
            // In any websocket error, break loop.
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Clone things we want to pass (move) to the receiving task.
    let tx = room_tx.clone();
    let name = username.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let user_msg: UserMsgIn = serde_json::from_str(text.as_str()).unwrap();
            if user_msg.message.len() > 128 {
                continue;
            }

            let out = serde_json::json!(UserMsgOut {
                user: name.clone(),
                message: user_msg.message.clone(),
            })
            .to_string();
            let _ = tx.send(out.clone());
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    info!("User {username} left.");

    state.user_set.lock().unwrap().remove(&username);
}

fn check_username(state: &TweeterState, name: &str) -> bool {
    let mut user_set = state.user_set.lock().unwrap();

    if !user_set.contains(name) {
        user_set.insert(name.to_owned());

        return true;
    }

    false
}

fn find_room(state: &TweeterState, room_number: &usize) -> broadcast::Sender<String> {
    let mut rooms = state.rooms.lock().unwrap();

    if let Some(room_tx) = rooms.get(&room_number) {
        room_tx.clone()
    } else {
        let (tx, _) = broadcast::channel(100_000);
        rooms.insert(room_number.clone(), tx.clone());
        tx
    }
}

async fn archive_files(body: Bytes) -> Result<String, (StatusCode, String)> {
    info!("20 archive files started");
    let mut a = Archive::new(body.as_ref());

    Ok(a.entries().unwrap().count().to_string())
}

async fn archive_files_size(body: Bytes) -> Result<String, (StatusCode, String)> {
    info!("20 archive files size");
    let mut a = Archive::new(body.as_ref());

    Ok(a.entries()
        .unwrap()
        .map(|file| file.unwrap().header().size().unwrap())
        .sum::<u64>()
        .to_string())
}

async fn cookie(body: Bytes) -> Result<String, (StatusCode, String)> {
    info!("20 cookie started");
    let mut a = Archive::new(body.as_ref());
    let temp_dir = tempdir().unwrap();
    a.unpack(temp_dir.path()).unwrap();

    let repo = match Repository::init(temp_dir.path()) {
        Ok(repo) => repo,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to init Git: {}", e),
            ))
        }
    };

    let mut n = 0;

    for _ in 1..1000 {
        let rev = match n {
            0 => "christmas".to_string(),
            n => format!("christmas@{{{}}}", n),
        };
        let head = repo.revparse_ext(rev.as_str()).unwrap().0;
        let commit = head.as_commit().unwrap();
        let id = commit.id();
        let author = commit.author().name().unwrap().to_string();

        let tree = commit.tree().unwrap();
        if christmas_walk(&tree, &repo).is_some() {
            return Ok(format!("{author} {id}"));
        }
        n += 1;
    }

    Err((StatusCode::BAD_REQUEST, "no cookie".to_string()))
}

fn christmas_walk(tree: &Tree, repo: &Repository) -> Option<()> {
    for entry in tree.iter() {
        let x = entry.to_object(&repo).unwrap();
        if let Some(tt) = x.as_tree() {
            if christmas_walk(&tt, repo).is_some() {
                return Some(());
            }
            continue;
        }

        let name = entry.name().unwrap();
        if name != "santa.txt" {
            continue;
        }
        let o = entry.to_object(&repo).unwrap().peel_to_blob().unwrap();
        let content = String::from_utf8(o.content().to_vec()).unwrap();

        if content.find("COOKIE").is_some() {
            return Some(());
        }
    }

    None
}

async fn s2_coords(Path(s2_string): Path<String>) -> Result<String, (StatusCode, String)> {
    info!("21 s2 coords started");
    let s2_coords = u64::from_str_radix(s2_string.as_str(), 2).unwrap();
    let cell = s2::cellid::CellID(s2_coords);
    let ll = s2::latlng::LatLng::from(cell);
    let mut lat = DMS::from_decimal_degrees(ll.lat.deg(), true);
    let mut long = DMS::from_decimal_degrees(ll.lng.deg(), false);
    lat.seconds = (lat.seconds * 1000.).round() / 1000.;
    long.seconds = (long.seconds * 1000.).round() / 1000.;

    Ok(format!("{} {}", lat.to_string(), long.to_string()))
}

async fn s2_country(Path(s2_string): Path<String>) -> Result<String, (StatusCode, String)> {
    info!("21 s2 country started");
    let s2_coords = u64::from_str_radix(s2_string.as_str(), 2).unwrap();
    let cell = s2::cellid::CellID(s2_coords);
    let ll = s2::latlng::LatLng::from(cell);

    let geocoder = ReverseGeocoder::new();
    let search_result = geocoder.search((ll.lat.deg(), ll.lng.deg()));
    let cc = search_result.record.cc.clone();
    let countries = countries();
    let country = countries.get(&cc).unwrap();

    Ok(country.clone())
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://postgres:{secrets.PASSWORD}@localhost:5432/cch23"
    )]
    pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(ok))
        .route("/-1/error", get(error))
        .route("/1/*key", get(exclusive_cube))
        .route("/4/strength", post(strength))
        .route("/4/contest", post(contest))
        .route("/5", post(slicing_the_loop))
        .route("/6", post(elf_count))
        .route("/7/decode", get(decode))
        .route("/7/bake", get(bake))
        .route("/8/weight/:pokedex_number", get(weight))
        .route("/8/drop/:pokedex_number", get(drop))
        .nest_service("/11/assets", ServeDir::new("assets"))
        .route("/11/red_pixels", post(red_pixels))
        .route("/12/save/:string", post(save_string))
        .route("/12/load/:string", get(load_string))
        .with_state(Arc::new(AppState {
            data: Arc::new(Mutex::new(HashMap::new())),
        }))
        .route("/12/ulids", post(ulids))
        .route("/12/ulids/:weekday", post(ulids_weekday))
        .route("/13/sql", get(sql))
        .route("/13/reset", post(reset))
        .route("/13/orders", post(orders))
        .route("/13/orders/total", get(orders_total))
        .route("/13/orders/popular", get(orders_popular))
        .route("/14/unsafe", post(html_unsafe))
        .route("/14/safe", post(html_safe))
        .route("/15/nice", post(nice))
        .route("/15/game", post(game))
        .route("/18/reset", post(reset))
        .route("/18/orders", post(orders))
        .route("/18/regions", post(regions))
        .route("/18/regions/total", get(regions_total))
        .route("/18/regions/top_list/:limit", get(regions_top_list))
        .with_state(DbState { pool })
        .route("/19/ws/ping", get(ws_ping))
        .route("/19/reset", post(tweeter_reset))
        .route("/19/views", get(tweeter_views))
        .route(
            "/19/ws/room/:room_number/user/:username",
            get(tweeter_ws_handler),
        )
        .with_state(Arc::new(TweeterState {
            views: Arc::new(Mutex::new(0)),
            user_set: Mutex::new(HashSet::new()),
            rooms: Mutex::new(HashMap::new()),
        }))
        .route("/20/archive_files", post(archive_files))
        .route("/20/archive_files_size", post(archive_files_size))
        .route("/20/cookie", post(cookie))
        .route("/21/coords/:s2", get(s2_coords))
        .route("/21/country/:s2", get(s2_country));

    Ok(router.into())
}
