use axum::{routing::post, Json, Router};
use log::info;
use reqwest::StatusCode;

pub fn get_routes() -> Router {
    Router::new()
        .route("/14/unsafe", post(html_unsafe))
        .route("/14/safe", post(html_safe))
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
