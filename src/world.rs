use error::*;

const CHUNK_LAT: f64 = 0.0088; // y
const CHUNK_LON: f64 = 0.0144; // x

#[derive(Clone)]
pub struct LatLon {
    lat: f64,
    lon: f64
}

pub struct World {
    centre: LatLon
}

pub struct Chunk {}

impl LatLon {
    pub fn new(lat: f64, lon: f64) -> LatLon {
        LatLon {
            lat,
            lon
        }
    }
}

impl World {
    pub fn new(mut centre: LatLon) -> World {
        centre.lat += CHUNK_LAT / 2.0;
        centre.lon += CHUNK_LON / 2.0;
        World {
            centre
        }
    }

    pub fn request_chunk(&self, x: i32, y: i32) -> SimResult<Chunk> {
        let centre_corner = {
            let mut l = self.centre.clone();
            l.lat -= CHUNK_LAT / 2.0;
            l.lon -= CHUNK_LON / 2.0;
            l
        };

        let (min_lat, max_lat) = (centre_corner.lat + (CHUNK_LAT * f64::from(y)),
                                  centre_corner.lat + (CHUNK_LAT * f64::from(y + 1)));
        let (min_lon, max_lon) = (centre_corner.lon + (CHUNK_LON * f64::from(x)),
                                  centre_corner.lon + (CHUNK_LON * f64::from(x + 1)));

        // TODO request

//        println!("{},{}", min_lat, min_lon);
//        println!("{},{}", min_lat, max_lon);
//        println!("{},{}", max_lat, max_lon);
//        println!("{},{}", max_lat, min_lon);
        Ok(Chunk{})
    }
}