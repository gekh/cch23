use axum::{extract::Path, routing::get, Router};
use dms_coordinates::DMS;
use log::info;
use reqwest::StatusCode;
use reverse_geocoder::ReverseGeocoder;

use crate::countries::countries;

pub fn get_routes() -> Router {
    Router::new()
        .route("/21/coords/:s2", get(s2_coords))
        .route("/21/country/:s2", get(s2_country))
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
