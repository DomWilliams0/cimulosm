use error::*;

pub struct LatLon {
    lat: f64,
    lon: f64
}

pub struct World {
    centre: LatLon
}

pub struct Chunk {

}

impl LatLon {
    pub fn new(lat: f64, lon: f64) -> LatLon {
        LatLon {
            lat,
            lon
        }
    }
}

impl World {
    pub fn new(centre: LatLon) -> World {
        World {
            centre
        }
    }

    pub fn request_chunk(&self, x: i32, y: i32) -> SimResult<Chunk> {

        unimplemented!()
    }
}