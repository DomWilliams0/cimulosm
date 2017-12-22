use world::LatLon;

const CHUNK_LAT: f64 = -0.00650;
const CHUNK_LON: f64 = 0.0105;

//  in: world origin (top left), chunk coords
// out: top left, bottom right of desired chunk
pub fn get_chunk_bounds(origin: &LatLon, chunk_pos: (i32, i32)) -> (LatLon, LatLon) {
    let top_left = get_chunk_latlon(origin, chunk_pos);
    let bottom_right = get_chunk_bottom_right(&top_left);
    (top_left, bottom_right)
}

//  in: world origin (top left), chunk coords
// out: top left latlon of desired chunk
fn get_chunk_latlon(origin: &LatLon, chunk_pos: (i32, i32)) -> LatLon {
    LatLon {
        lat: origin.lat + (CHUNK_LAT * f64::from(chunk_pos.1)),
        lon: origin.lon + (CHUNK_LON * f64::from(chunk_pos.0)),
    }
}

//  in: top left
// out: bottom right
fn get_chunk_bottom_right(top_left: &LatLon) -> LatLon {
    LatLon {
        lat: top_left.lat + CHUNK_LAT,
        lon: top_left.lon + CHUNK_LON
    }
}
