use axum::{routing::post, Json, Router};
use log::info;
use regex::Regex;
use reqwest::StatusCode;

pub fn get_routes() -> Router {
    Router::new().route("/6", post(elf_count))
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

    Ok(Json(ElfCount {
        elf,
        elf_on_a_shelf,
        no_elf,
    }))
}
