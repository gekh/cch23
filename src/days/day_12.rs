use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use log::info;
use num_traits::PrimInt;
use reqwest::StatusCode;
use std::{collections::HashMap, sync::Arc};
use std::{sync::Mutex, time::SystemTime};

pub fn get_routes() -> Router {
    Router::new()
        .route("/12/save/:string", post(save_string))
        .route("/12/load/:string", get(load_string))
        .with_state(Arc::new(AppState {
            data: Arc::new(Mutex::new(HashMap::new())),
        }))
        .route("/12/ulids", post(ulids))
        .route("/12/ulids/:weekday", post(ulids_weekday))
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
