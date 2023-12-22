use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use log::info;
use reqwest::StatusCode;
use sqlx::PgPool;

pub fn get_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/13/sql", get(sql))
        .route("/13/reset", post(reset))
        .route("/13/orders", post(orders))
        .route("/13/orders/total", get(orders_total))
        .route("/13/orders/popular", get(orders_popular))
        .with_state(DbState { pool })
}

#[derive(Clone)]
pub struct DbState {
    pub pool: PgPool,
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

pub async fn reset(State(state): State<DbState>) -> Result<String, StatusCode> {
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
pub struct Order {
    id: i64,
    region_id: i64,
    gift_name: String,
    quantity: i64,
}

pub async fn orders(
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
