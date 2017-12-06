use libc::*;
use std::ptr;
use std::ffi;

#[repr(C)]
#[derive(Debug)]
struct Point {
    x: u32,
    y: u32
}

#[repr(C)]
struct OsmVec<T> {
    data: *mut T,
    length: u32,
    capacity: u32
}

#[repr(C)]
struct OsmWorld {
    bounds: [u32; 2],
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

#[repr(C)]
struct OsmRoad {
    road_type: RoadType,
    segments: OsmVec<Point>,
    name: *const c_char
}

#[repr(C)]
struct OsmLandUse {
    land_use_type: LandUseType,
    points: OsmVec<Point>,
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

fn convert_vec<T, U>(orig: OsmVec<T>) -> Vec<U>
    where
        U: From<T>
{
    let mut v = Vec::<U>::with_capacity(orig.length as usize);
    for i in 0..orig.length {
        let d = unsafe { ptr::read(orig.data.offset(i as isize)) };
        v.push(From::from(d));
    }
    v
}

impl From<OsmRoad> for Road {
    fn from(r: OsmRoad) -> Self {
        let name = if r.name.is_null() {
            String::new()
        } else {
            let cname = unsafe {ffi::CStr::from_ptr(r.name)};
            cname.to_str().unwrap().to_owned()
        };

        Self {
            road_type: r.road_type,
            segments: convert_vec(r.segments),
            name
        }
    }
}

impl From<OsmLandUse> for LandUse {
    fn from(lu: OsmLandUse) -> Self {
        Self {
            land_use_type: lu.land_use_type,
            points: convert_vec(lu.points),
        }
    }
}

#[derive(Debug)]
pub struct World {
    width: u32,
    height: u32,
    roads: Vec<Road>,
    land_uses: Vec<LandUse>
}

impl From<OsmWorld> for World {
    fn from(w: OsmWorld) -> Self {
        World {
            width: w.bounds[0],
            height: w.bounds[1],
            roads: convert_vec(w.roads),
            land_uses: convert_vec(w.land_uses)
        }
    }
}

#[derive(Debug)]
pub struct Road {
    road_type: RoadType,
    segments: Vec<Point>,
    name: String
}

#[derive(Debug)]
pub struct LandUse {
    land_use_type: LandUseType,
    points: Vec<Point>,
}


#[link_name = "osm"]
extern {
    fn parse_osm_from_file(path: *const c_char, out: *mut OsmWorld) -> i32;
}

pub fn test_from_file(path: &str) -> Result<World, i32> {
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