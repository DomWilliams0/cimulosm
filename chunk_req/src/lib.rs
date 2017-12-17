extern crate reqwest;
use std::io::Read;
use std::error::Error;

pub fn request_osm(top_left: (f64, f64), bottom_right: (f64, f64)) -> Result<String, String> {
    actually_request_osm(
        bottom_right.0,
        top_left.0,
        top_left.1,
        bottom_right.1
        )
}

fn actually_request_osm(min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> Result<String, String> {

    let url = format!("http://overpass-api.de/api/map?bbox={},{},{},{}",
                      min_lon, min_lat, max_lon, max_lat);
    let mut resp = reqwest::get(url.as_str())
        .map_err(|e| e.description().to_owned())?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Bad server status code {}", resp.status()));
    }

    let mut xml = String::new();
    resp.read_to_string(&mut xml).map_err(|e| e.description().to_owned())?;
    Ok(xml)
}

