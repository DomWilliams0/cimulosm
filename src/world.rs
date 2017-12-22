use error::*;
use std::io::{Read, Write};
use std::{fs, env};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread;
use std_semaphore::Semaphore;

use chunk_req;
use parser;
use latlon;

const CONCURRENT_REQ_COUNT: isize = 3;
lazy_static! {
    static ref REQUEST_SEM: Semaphore = Semaphore::new(CONCURRENT_REQ_COUNT);
}


#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32
}

pub type Id = i64;

#[derive(Debug, Clone)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

pub trait PointsHolder {
    fn pixels(&mut self) -> &mut Vec<Point>;
}

#[derive(Debug)]
pub struct Road {
    pub road_type: parser::RoadType,
    pub segments: Vec<Point>,
    pub name: String
}

#[derive(Debug)]
pub struct LandUse {
    pub land_use_type: parser::LandUseType,
    pub points: Vec<Point>,
}

type IdCountMap = HashMap<Id, u16>;

pub struct World {
    pub origin: LatLon,

    // id -> count
    road_refs: IdCountMap,
    land_use_refs: IdCountMap,

    // TODO use quadtree?
    pub loaded_roads: Vec<Road>,
    pub loaded_land_uses: Vec<LandUse>,

    loaded_chunks: HashMap<(i32, i32), Chunk>,
    loading_chunks: HashSet<(i32, i32)>,
}

#[derive(Debug)]
pub struct Chunk {
    road_refs: Vec<Id>,
    land_use_refs: Vec<Id>,
}

pub struct PartialChunk(pub SimResult<parser::PartialWorld>, pub (i32, i32));

impl PointsHolder for Road {
    fn pixels(&mut self) -> &mut Vec<Point> {
        &mut self.segments
    }
}

impl PointsHolder for LandUse {
    fn pixels(&mut self) -> &mut Vec<Point> {
        &mut self.points
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
            land_use_refs: HashMap::new(),
            loaded_roads: Vec::new(),
            loaded_land_uses: Vec::new(),
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
                let _guard = REQUEST_SEM.access();
                fetch_xml(&bounds).and_then(parser::parse_osm)
            };
            result_channel.send(PartialChunk(res, coord));
        });

    }

    pub fn finish_chunk_request(&mut self, partial_chunk: PartialChunk) {

        fn inc_refs<T>(chunk_refs: &[Id], world_refs: &mut IdCountMap, chunk_objs: &mut HashMap<Id, T>, world_objs: &mut Vec<T>, que: &str) {
            for &id in chunk_refs {
                let count = world_refs.entry(id).or_insert(0);

                // first time load
                if *count == 0 {
                    let obj = chunk_objs.remove(&id).unwrap();
                    world_objs.push(obj);
                } else {
                    println!("Incrementing {} {} ref count to {}", que, id, *count + 1);
                }

                *count += 1;
            }
        }

        let PartialChunk(partial_world, coord) = partial_chunk;
        self.loading_chunks.remove(&coord);

        if let Ok(mut partial_world) = partial_world {
            partial_world.make_coords_relative_to(&self.origin);

            // create chunk
            let chunk = Chunk {
                road_refs: partial_world.roads.keys().cloned().collect(),
                land_use_refs: partial_world.land_uses.keys().cloned().collect(),
            };

            inc_refs(&chunk.road_refs, &mut self.road_refs, &mut partial_world.roads, &mut self.loaded_roads, "road");
            inc_refs(&chunk.land_use_refs, &mut self.land_use_refs, &mut partial_world.land_uses, &mut self.loaded_land_uses, "land use");

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

    pub fn convert_latlon_to_pixel(&self, latlon: &LatLon) -> Point {
        let origin = parser::convert_latlon(self.origin.lat, self.origin.lon);
        let point = parser::convert_latlon(latlon.lat, latlon.lon);
        Point {
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
