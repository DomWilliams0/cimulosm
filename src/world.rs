use error::*;
use std::io::{Read, Write};
use reqwest;
use parser;
use std::{fs, env};

const CHUNK_LAT: f64 = 0.0088;
// y
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

        let parsed_bbox = request_bbox(min_lat, max_lat, min_lon, max_lon);
        println!("bbox is {:?}", parsed_bbox);

        Ok(Chunk {})
    }
}

fn fetch_xml(min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> SimResult<String> {
    let cache = {
        let mut p = env::temp_dir();
        p.push(format!("chunk-cache--{}-{}-{}-{}.osm", min_lat, max_lat, min_lon, max_lon));
        p
    };

    if cache.is_file() {
        println!("Loading cached chunk");
        let mut contents = String::new();
        fs::File::open(cache)?.read_to_string(&mut contents)?;
        Ok(contents)
    } else {
        println!("Requesting chunk");
        let mut resp = reqwest::get(
            format!("http://overpass-api.de/api/map?bbox={},{},{},{}",
                    min_lon, min_lat, max_lon, max_lat).as_str())?;

        let status = resp.status();
        if !status.is_success() {
            return Err(ErrorKind::OsmRequest(resp.status().as_u16() as i32).into());
        }

        let mut xml = String::new();
        resp.read_to_string(&mut xml)?;

        fs::File::create(cache)?.write_all(xml.as_bytes())?;

        Ok(xml)
    }
}

fn request_bbox(min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64)
                -> SimResult<parser::World> {
    parser::parse_osm(fetch_xml(min_lat, max_lat, min_lon, max_lon)?)
}
