use libc::*;
use std::{self, ffi, ptr};
use std::collections::HashMap;
use error::*;
use world::{Id, Road, LandUse, Pixel, LatLon, PixelVecHolder};
use sfml::system::Vector2f;

#[repr(C)]
#[derive(Debug, Clone)]
struct OsmLatLon {
    lat: f64,
    lon: f64
}

#[repr(C)]
#[derive(Debug)]
pub struct OsmPoint {
    pub x: i32,
    pub y: i32
}

#[repr(C)]
#[derive(Debug)]
struct OsmVec<T> {
    data: *mut T,
    length: u32,
    capacity: u32
}

#[repr(C)]
struct OsmWorld {
    roads: OsmVec<OsmRoad>,
    land_uses: OsmVec<OsmLandUse>
}

#[repr(C)]
#[derive(Debug)]
pub enum RoadType {
    Unknown,
    Motorway,
    Primary,
    Secondary,
    Minor,
    Residential,
    Pedestrian
}

#[repr(C)]
#[derive(Debug)]
pub enum LandUseType {
    Unknown,
    Residential,
    Commercial,
    Agriculture,
    Industrial,
    Green,
    Water
}

trait OsmIdHolder {
    fn id(&self) -> Id;
}

#[repr(C)]
#[derive(Debug)]
struct OsmRoad {
    id: Id,
    road_type: RoadType,
    segments: OsmVec<OsmLatLon>,
    name: *const c_char
}

#[repr(C)]
#[derive(Debug)]
struct OsmLandUse {
    id: Id,
    land_use_type: LandUseType,
    points: OsmVec<OsmLatLon>,
}

impl OsmIdHolder for OsmRoad {
    fn id(&self) -> Id {
        self.id
    }
}

impl OsmIdHolder for OsmLandUse {
    fn id(&self) -> Id {
        self.id
    }
}

impl Into<Vector2f> for OsmPoint {
    fn into(self) -> Vector2f {
        Vector2f::new(self.x as f32, self.y as f32)
    }
}

impl<T> Default for OsmVec<T> {
    fn default() -> Self {
        OsmVec::<T> {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0
        }
    }
}

impl Default for OsmWorld {
    fn default() -> Self {
        Self {
            roads: Default::default(),
            land_uses: Default::default(),
        }
    }
}

impl From<LatLon> for OsmLatLon {
    fn from(ll: LatLon) -> Self {
        Self {
            lat: ll.lat,
            lon: ll.lon
        }
    }
}

fn convert_vec_to_map<T, U>(orig: &OsmVec<T>) -> HashMap<Id, U>
    where
        T: OsmIdHolder,
        U: From<T>
{
    let mut m = HashMap::<Id, U>::with_capacity(orig.length as usize);
    for i in 0..orig.length {
        let d: T = unsafe { ptr::read(orig.data.offset(i as isize)) };
        m.insert(d.id(), From::from(d));
    }
    m
}

fn convert_latlon_vec(orig: &OsmVec<OsmLatLon>) -> Vec<Pixel>
{
    let mut v = Vec::with_capacity(orig.length as usize);
    for i in 0..orig.length {
        let d = unsafe { ptr::read(orig.data.offset(i as isize)) };
        let p = convert_latlon(d.lat, d.lon);
        v.push(Pixel {x: p.x, y: p.y});
    }
    v
}

impl From<OsmRoad> for Road {
    fn from(r: OsmRoad) -> Self {
        let name = if r.name.is_null() {
            String::new()
        } else {
            let cname = unsafe { ffi::CStr::from_ptr(r.name) };
            cname.to_str().unwrap().to_owned()
        };

        Self {
            road_type: r.road_type,
            segments: convert_latlon_vec(&r.segments),
            name
        }
    }
}

impl From<OsmLandUse> for LandUse {
    fn from(lu: OsmLandUse) -> Self {
        Self {
            land_use_type: lu.land_use_type,
            points: convert_latlon_vec(&lu.points),
        }
    }
}

#[derive(Debug)]
pub struct PartialWorld {
    //pub width: u32,
    //pub height: u32,
    pub roads: HashMap<Id, Road>,
    pub land_uses: HashMap<Id, LandUse>
}

impl From<OsmWorld> for PartialWorld {
    fn from(w: OsmWorld) -> Self {
        PartialWorld {
            roads: convert_vec_to_map(&w.roads),
            land_uses: convert_vec_to_map(&w.land_uses)
        }
    }
}

impl PartialWorld {

    pub fn make_coords_relative_to(&mut self, origin: &LatLon) {
        fn make_relative<T: PixelVecHolder>(x: &mut T, origin: &OsmPoint) {
            for p in x.pixels() {
                (*p).x -= origin.x;
                (*p).y -= origin.y;
            }
        }

        let rel = convert_latlon(origin.lat, origin.lon);

        for r in self.roads.values_mut() {
            make_relative(r, &rel);
        }

        // for lu in self.land_uses.values_mut() {
        //     make_relative(&mut lu);
        // }
    }
}

impl Drop for OsmWorld {
    fn drop(&mut self) {
        unsafe { free_world(self as *mut _) };
    }
}

#[link_name = "osm"]
extern "C" {
    fn parse_osm_from_buffer(buffer: *const c_void, len: size_t, out: *mut OsmWorld) -> i32;
    fn free_world(world: *mut OsmWorld);
}

pub fn parse_osm(xml: String) -> SimResult<PartialWorld> {
    let len = xml.len();
    let cstr = ffi::CString::new(xml)?;
    let mut osm_world = OsmWorld::default();

    match unsafe {
        parse_osm_from_buffer(
            cstr.as_ptr() as *const _,
            len as size_t,
            &mut osm_world as *mut _,
        )
    } {
        0 => Ok(PartialWorld::from(osm_world)),
        e => Err(ErrorKind::OsmParse(e).into()),
    }
}


pub fn convert_latlon(lat: f64, lon: f64) -> OsmPoint {
    const ZOOM: i32 = 23;
    const N: f64 = (1 << ZOOM) as f64;

    let lat_rad = lat.to_radians();

    let x = ((lon + 180.0) / 360.0 * N) as i32;
    let y = ((1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / std::f64::consts::PI) / 2.0 * N) as i32;
    OsmPoint {x, y}

}
/*
fn test_from_file(path: &str) -> Result<World, i32> {
    let c_string = match ffi::CString::new(path) {
        Ok(s) => s,
        Err(_) => return Err(1)
    };

    let mut world = OsmWorld {
        bounds: [0, 0],
        roads: Default::default(),
        land_uses: Default::default(),
    };

    let ret = unsafe {
        parse_osm_from_file(
            c_string.as_ptr() as *const _,
            &mut world as *mut OsmWorld
        )
    };

    match ret {
        0 => Ok(world.into()),
        n => Err(n)
    }
}
*/
