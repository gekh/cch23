use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use log::info;
use reqwest::StatusCode;
use sqlx::PgPool;

use super::day_13::{orders, reset, DbState};

pub fn get_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/18/reset", post(reset))
        .route("/18/orders", post(orders))
        .route("/18/regions", post(regions))
        .route("/18/regions/total", get(regions_total))
        .route("/18/regions/top_list/:limit", get(regions_top_list))
        .with_state(DbState { pool })
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
