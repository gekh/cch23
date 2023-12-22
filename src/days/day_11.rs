use axum::{extract::Multipart, routing::post, Router};
use image::{io::Reader as ImageReader, GenericImageView, Pixel};
use itertools::Itertools;
use log::info;
use reqwest::StatusCode;
use std::io::Cursor;
use tower_http::services::ServeDir;

pub fn get_routes() -> Router {
    Router::new()
        .nest_service("/11/assets", ServeDir::new("assets"))
        .route("/11/red_pixels", post(red_pixels))
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
