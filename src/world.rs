use error::*;
use std::io::{Read, Write};
use std::{fs, env};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread;

use chunk_req;
use parser;
use latlon;

//const CHUNK_LAT: f64 = 0.0088; // y
//
//const CHUNK_LON: f64 = 0.0144; // x
//
//const CHUNK_PAD: f64 = 0.2;
//const CHUNK_PAD_LAT: f64 = CHUNK_LAT * CHUNK_PAD;
//const CHUNK_PAD_LON: f64 = CHUNK_LON * CHUNK_PAD;

#[derive(Debug, Clone, Copy)]
pub struct Pixel {
    pub x: i32,
    pub y: i32
}

pub type Id = i64;

#[derive(Debug, Clone)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

pub trait PixelVecHolder {
    fn pixels(&mut self) -> &mut Vec<Pixel>;
}

#[derive(Debug)]
pub struct Road {
    pub road_type: parser::RoadType,
    pub segments: Vec<Pixel>,
    pub name: String
}

#[derive(Debug)]
pub struct LandUse {
    pub land_use_type: parser::LandUseType,
    pub points: Vec<Pixel>,
}

pub struct World {
    pub origin: LatLon,

    // road id -> count
    road_refs: HashMap<Id, u8>,

    // TODO use quadtree?
    pub loaded_roads: Vec<Road>,

    loaded_chunks: HashMap<(i32, i32), Chunk>,
    loading_chunks: HashSet<(i32, i32)>,
}

#[derive(Debug)]
pub struct Chunk {
    road_refs: Vec<Id>,
}

pub struct PartialChunk(pub SimResult<parser::PartialWorld>, pub (i32, i32));

impl PixelVecHolder for Road {
    fn pixels(&mut self) -> &mut Vec<Pixel> {
        &mut self.segments
    }
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
    pub fn new(origin: LatLon) -> World {
        World {
            origin,
            road_refs: HashMap::new(),
            loaded_roads: Vec::new(),
            loaded_chunks: HashMap::new(),
            loading_chunks: HashSet::new(),
        }
    }

    pub fn request_chunk_async(
        &mut self,
        x: i32,
        y: i32,
        result_channel: mpsc::Sender<PartialChunk>,
    ) {

        let coord = (x, y);
        let bounds = latlon::get_chunk_bounds(&self.origin, coord);
        let loaded_already = self.loaded_chunks.contains_key(&coord) ||
            self.loading_chunks.contains(&coord);
        if !loaded_already {
            self.loading_chunks.insert(coord);
        }

        thread::spawn(move || {
            let res = if loaded_already {
                Err(ErrorKind::ChunkAlreadyLoaded(coord).into())
            } else {
                fetch_xml(&bounds).and_then(parser::parse_osm)
            };
            result_channel.send(PartialChunk(res, coord));
        });

    }

    pub fn finish_chunk_request(&mut self, partial_chunk: PartialChunk) {
        let PartialChunk(mut partial_world, coord) = partial_chunk;
        self.loading_chunks.remove(&coord);

        if let Ok(mut partial_world) = partial_world {
            partial_world.make_coords_relative_to(&self.origin);

            // create chunk
            let chunk = Chunk { road_refs: partial_world.roads.keys().cloned().collect() };

            // increment reference counts for all roads
            for &id in &chunk.road_refs {
                let count = {
                    self.road_refs.entry(id).or_insert(0)
                };

                // first time load
                if *count == 0 {
                    let road = partial_world.roads.remove(&id).unwrap();
                    self.loaded_roads.push(road);
                } else {
                    println!("Incrementing road {} ref count to {}", id, *count + 1);
                }

                *count += 1;
            }

            self.loaded_chunks.insert(coord, chunk);

        }
    }

    pub fn request_chunk_sync(&mut self, x: i32, y: i32) -> SimResult<()> {
        let (send, recv) = mpsc::channel();
        self.request_chunk_async(x, y, send);
        let res = recv.recv()?;
        self.finish_chunk_request(res);
        Ok(())
    }

    pub fn convert_latlon_to_pixel(&self, latlon: &LatLon) -> Pixel {
        let origin = parser::convert_latlon(self.origin.lat, self.origin.lon);
        let point = parser::convert_latlon(latlon.lat, latlon.lon);
        Pixel {
            x: point.x - origin.x,
            y: point.y - origin.y
        }
    }
}

fn fetch_xml(bounds: &(LatLon, LatLon)) -> SimResult<String> {
    let cache = {
        let mut p = env::temp_dir();
        p.push(format!(
            "chunk_cache_{}_{}_{}_{}.osm",
            (bounds.1).lat,
            (bounds.0).lat,
            (bounds.1).lon,
            (bounds.0).lon
        ));
        p
    };

    if cache.is_file() {
        println!("Loading cached chunk from {:?}", cache);
        let mut contents = String::new();
        fs::File::open(cache)?.read_to_string(&mut contents)?;
        Ok(contents)
    } else {
        println!(
            "Sending request for {}, {} -> {}, {}",
            (bounds.0).lat,
            (bounds.0).lon,
            (bounds.1).lat,
            (bounds.1).lon
        );
        let xml = chunk_req::request_osm((bounds.0.lat, bounds.0.lon), (bounds.1.lat, bounds.1.lon))?;
        println!("{} bytes read", xml.len());
        fs::File::create(cache)?.write_all(xml.as_bytes())?;

        Ok(xml)
    }
}
