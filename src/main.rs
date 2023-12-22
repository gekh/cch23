mod countries;
mod days;

use axum::{http::StatusCode, routing::get, Router};
use sqlx::PgPool;

async fn ok() -> Result<String, StatusCode> {
    Ok(String::from("okay"))
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
        .merge(days::day_00::get_routes())
        .merge(days::day_01::get_routes())
        .merge(days::day_04::get_routes())
        .merge(days::day_05::get_routes())
        .merge(days::day_06::get_routes())
        .merge(days::day_07::get_routes())
        .merge(days::day_08::get_routes())
        .merge(days::day_11::get_routes())
        .merge(days::day_12::get_routes())
        .merge(days::day_13::get_routes(pool.clone()))
        .merge(days::day_14::get_routes())
        .merge(days::day_15::get_routes())
        .merge(days::day_18::get_routes(pool.clone()))
        .merge(days::day_19::get_routes())
        .merge(days::day_20::get_routes())
        .merge(days::day_21::get_routes());
    Ok(router.into())
}
