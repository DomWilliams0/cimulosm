use error::*;
use std::io::{Read, Write};
use reqwest;
use parser;
use std::{fs, env};
use std::collections::HashMap;

const CHUNK_LAT: f64 = 0.0088; // y

const CHUNK_LON: f64 = 0.0144; // x

const CHUNK_PAD: f64 = 0.2;
const CHUNK_PAD_LAT: f64 = CHUNK_LAT * CHUNK_PAD;
const CHUNK_PAD_LON: f64 = CHUNK_LON * CHUNK_PAD;

pub type Id = i64;

#[derive(Clone)]
pub struct LatLon {
    lat: f64,
    lon: f64
}

#[derive(Clone, Copy)]
struct ChunkId {
    x: i32,
    y: i32,
}

#[derive(Debug)]
pub struct Road {
    pub road_type: parser::RoadType,
    pub segments: Vec<parser::Point>,
    pub name: String
}

#[derive(Debug)]
pub struct LandUse {
    pub land_use_type: parser::LandUseType,
    pub points: Vec<parser::Point>,
}

pub struct World {
    centre: LatLon,

    // road id -> count
    road_refs: HashMap<Id, u8>,

    // TODO use quadtree?
    loaded_roads: Vec<Road>
}

pub struct Chunk {
    id: ChunkId,
    road_refs: Vec<Id>
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
    pub fn new(mut centre: LatLon) -> World {
        centre.lat += CHUNK_LAT / 2.0;
        centre.lon += CHUNK_LON / 2.0;
        World {
            centre,
            road_refs: HashMap::new(),
            loaded_roads: Vec::new()
        }
    }

    pub fn request_chunk(&mut self, x: i32, y: i32) -> SimResult<Chunk> {
        let centre_corner = {
            let mut l = self.centre.clone();
            l.lat -= CHUNK_LAT / 2.0;
            l.lon -= CHUNK_LON / 2.0;
            l
        };

        let (min_lat, max_lat) = (centre_corner.lat - CHUNK_PAD_LAT + (CHUNK_LAT * f64::from(y)),
                                  centre_corner.lat + CHUNK_PAD_LAT + (CHUNK_LAT * f64::from(y + 1)));
        let (min_lon, max_lon) = (centre_corner.lon - CHUNK_PAD_LON + (CHUNK_LON * f64::from(x)),
                                  centre_corner.lon + CHUNK_PAD_LON + (CHUNK_LON * f64::from(x + 1)));

        let mut w: parser::PartialWorld = request_bbox(min_lat, max_lat, min_lon, max_lon)?;

        // create chunk
        let mut chunk = Chunk {
            id: ChunkId { x, y },
            road_refs: w.roads.keys().cloned().collect()
        };

        // increment reference counts for all roads
        for &id in &chunk.road_refs {
            let count = {
                self.road_refs.entry(id).or_insert(0)
            };

            // first time load
            if *count == 0 {
                println!("Loading road {}", id);
                let road = w.roads.remove(&id).unwrap();
                self.loaded_roads.push(road);
            } else {
                println!("Incrementing road {} ref count to {}", id, *count + 1);
            }

            *count += 1;
        }

        Ok(chunk)
    }
}

fn fetch_xml(min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> SimResult<String> {
    let cache = {
        let mut p = env::temp_dir();
        p.push(format!("chunk_cache_{}_{}_{}_{}.osm", min_lat, max_lat, min_lon, max_lon));
        p
    };

    if cache.is_file() {
        println!("Loading cached chunk from {:?}", cache);
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
                -> SimResult<parser::PartialWorld> {
    parser::parse_osm(fetch_xml(min_lat, max_lat, min_lon, max_lon)?)
}
