#![allow(dead_code)]
extern crate sfml;
extern crate libc;
extern crate chunk_req;
extern crate std_semaphore;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate error_chain;

use sfml::graphics::*;
use sfml::window::*;
use sfml::system::*;
use std::env;
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender, Receiver};

mod world;
mod error;
mod parser;
mod latlon;
mod building;

use world::*;
use error::*;

const MOVE_SPEED: f64 = 5.0;
const ZOOM_SPEED: f64 = 0.05;

fn main() {
    let origin = {
        let var = env::var("LATLON");
        let mut split = var.as_ref().expect("$LATLON missing in env").split(',');
        let (lat, lon): (f64, f64) = match (split.next(), split.next()) {
            (Some(slat), Some(slon)) => (slat.parse().expect("Bad latitude"), slon.parse().expect("Bad longitude")),
            _ => panic!("<lat>,<lon> expected"),
        };
        LatLon::new(lat, lon)
    };

    let mut world = World::new(String::from("test"), origin);

    render_png(&mut world, "/tmp/render.png", (500, 500));
    // Renderer::new(500, 500, &mut world).start().unwrap();
}

fn render_png(world: &mut World, out_path: &'static str, dims: (u32, u32)) {

    world.request_chunk_sync(0, 0).unwrap();
    world.request_chunk_sync(0, -1).unwrap();

    let mut texture = {
        let size = Vector2f::new(dims.0 as f32, dims.1 as f32);
        let mut t = RenderTexture::new(dims.0, dims.1, false).unwrap();
        let mut r = RectangleShape::with_size(size);
        r.set_fill_color(&Color::BLACK);
        t.draw(&r);

        let mut v = t.view().to_owned();
        v.move_((size.x / -2.0, size.y / -2.0));
        t.set_view(&v);

        t
    };

    render_world(&mut texture, world, None);
    let mut copy = texture.texture().copy_to_image().unwrap();
    println!("Saving to {:?}", out_path);
    copy.flip_vertically();
    copy.save_to_file(out_path);

}


#[derive(Debug)]
enum LoadState{
    Loading,
    Unloading,
    Unloaded,
    Failed,
}

#[derive(Debug)]
enum StateChange {
    Counter(f64),
    Constant
}

#[derive(Debug)]
struct ChunkState(LoadState, StateChange);

struct Renderer<'a> {
    window: RenderWindow,
    world: &'a mut World,

    render_cache: Vec<Vertex>,
    chunk_size: Vector2i,

    load_new_chunks: bool,
    chunk_states: HashMap<(i32, i32), ChunkState>,
    load_channel: (Sender<world::PartialChunk>, Receiver<world::PartialChunk>),
}

impl<'a> Renderer<'a> {
    fn new(width: u32, height: u32, world: &'a mut World) -> Self {
        let mut window = RenderWindow::new(
            (width, height),
            "Hiya",
            Style::NONE,
            &Default::default()
        );
        window.set_framerate_limit(60);

        let chunk_size = {
            let (tl, br) = latlon::get_chunk_bounds(&world.origin, (0, 0));
            let tl_pix = parser::convert_latlon(tl.lat, tl.lon);
            let br_pix = parser::convert_latlon(br.lat, br.lon);
            Vector2i::new(
                br_pix.x - tl_pix.x,
                br_pix.y - tl_pix.y,
                )
        };

        Self {
            window,
            world,
            render_cache: Vec::new(),
            chunk_size,
            load_new_chunks: true,
            chunk_states: HashMap::new(),
            load_channel: mpsc::channel(),
        }
    }

    fn request_chunk_async(&mut self, x: i32, y: i32) {
        let sender = self.load_channel.0.clone();
        self.world.request_chunk_async(x, y, sender);
    }

    fn start(mut self) -> SimResult<()> {
        let mut cam = CameraChange::new(self.window.size(), 0.4);
        self.centre_on_chunk(0, 0);

        self.request_chunk_async(0, 0);

        let font = Font::from_file("res/ScreenMedium.ttf").expect("Could not load font");
        let mut text = Text::new("", &font, 8);

        let background_colour = Color::rgb(40, 40, 50);
        loop {
            while let Some(e) = self.window.poll_event() {
                match e {
                    Event::KeyPressed { code: Key::Escape, .. } => return Ok(()),
                    Event::KeyPressed { code: Key::Space, .. } => {
                        self.load_new_chunks = !self.load_new_chunks;
                        println!("Loading new chunks: {}", self.load_new_chunks);
                    },
                    Event::KeyPressed { code, .. } => cam.handle_key(code, true),
                    Event::KeyReleased { code, .. } => cam.handle_key(code, false),
                    Event::Resized { width, height } => cam.resize(width, height),
                    _ => {}
                }
            }

            // tick chunk states
            self.chunk_states.retain(|&_, state| {
                if let ChunkState(_, StateChange::Counter(ref mut i)) = *state {
                    *i -= 0.01;
                    *i > 0.0
                } else {
                    true
                }
            });

            // update chunk states with new
            {
                let chunk_changes = cam.apply(&mut self.window, self.chunk_size);
                if self.load_new_chunks && !chunk_changes.is_empty() {
                    for c in chunk_changes.iter() {
                        let state = if c.load {
                            self.request_chunk_async(c.x, c.y);
                            ChunkState(LoadState::Loading, StateChange::Constant)
                        } else {
                            ChunkState(LoadState::Unloading, StateChange::Counter(1.0))
                        };

                        // TODO temporarily disable unloading
                        if let ChunkState(LoadState::Loading, _) = state {
                            self.chunk_states.insert((c.x, c.y), state);
                        }
                    }
                }
            }

            // finish loading for loaded chunks
            while let Ok(chunk) = self.load_channel.1.try_recv() {
                let PartialChunk(res, coord) = chunk;
                self.chunk_states.remove(&coord);

                match res {
                    Err(Error(ErrorKind::ChunkAlreadyLoaded(_), _)) => {}
                    Err(e) => {
                        println!("Failed to load a chunk: {}", e.description());
                        self.chunk_states.insert(coord, ChunkState(LoadState::Failed, StateChange::Constant));
                    },
                    Ok(_) => self.world.finish_chunk_request(PartialChunk(res, coord)),
                }
            }


            self.window.clear(&background_colour);
            self.render_world(&mut text, &cam);
            self.window.display();
        }
    }

    fn centre_on_chunk(&mut self, x: i32, y: i32) {
        let mut view = self.window.view().to_owned();

        let (cx, cy) = (self.chunk_size.x as f32, self.chunk_size.y as f32);
        view.set_center(
            (cx * x as f32 + cx / 2.0,
             cy * y as f32 + cy / 2.0));
        self.window.set_view(&view);
    }

    fn render_world(&mut self, text: &mut Text, cam: &CameraChange) {
        fn get_state_colour(state: &LoadState, progress: f64) -> Color {
            let mut c = match *state {
                LoadState::Loading => Color::GREEN,
                LoadState::Unloading => Color::BLUE,
                LoadState::Unloaded => Color::BLACK,
                LoadState::Failed => Color::RED,
            };
            c.a = (progress * 255.0) as u8;
            c
        }

        render_world(&mut self.window, self.world, Some(&mut self.render_cache));

        // chunk outlines
        let mut rect = {
            let mut r = RectangleShape::with_size(
                Vector2f::new(self.chunk_size.x as f32, self.chunk_size.y as f32)
                );

            r.set_outline_thickness(1.0);
            r.set_outline_color(&Color::WHITE);
            r.set_fill_color(&Color::TRANSPARENT);
            r
        };

        for x in cam.min_chunk.0..cam.max_chunk.0 + 1 {
            for y in cam.min_chunk.1..cam.max_chunk.1 + 1 {
                let c = if let Some(&ChunkState(ref state, ref change)) = self.chunk_states.get(&(x, y)) {
                    let i = if let StateChange::Counter(i) = *change { i } else { 1.0 };
                    get_state_colour(state, i)
                } else {
                    Color::TRANSPARENT
                };
                rect.set_fill_color(&c);

                rect.set_position((
                        (x * self.chunk_size.x) as f32,
                        (y * self.chunk_size.y) as f32)
                                 );
                self.window.draw(&rect);

                text.set_string(&format!("{}, {}", x, y));
                text.set_position(rect.position());
                self.window.draw(text);
            }
        }
    }
}

fn render_world(target: &mut RenderTarget, world: &World, render_cache: Option<&mut Vec<Vertex>>) {
    fn get_road_colour(road_type: &parser::RoadType) -> Color {
        match *road_type {
            parser::RoadType::Motorway |
            parser::RoadType::Primary |
            parser::RoadType::Secondary => Color::rgb(255, 50, 50), // red
            parser::RoadType::Minor => Color::rgb(50, 50, 255), // blue
            parser::RoadType::Pedestrian => Color::rgb(100, 100, 100), // grey
            parser::RoadType::Residential => Color::rgb(50, 255, 50), // green
            _ => Color::rgb(255, 255, 255), // white
        }
    }
    fn get_land_use_colour(land_use_type: &parser::LandUseType) -> Color {
        let mut c = match *land_use_type {
            parser::LandUseType::Residential => Color::rgb(46, 204, 113), // green
            parser::LandUseType::Commercial => Color::rgb(243, 156, 18), // orange
            parser::LandUseType::Agriculture => Color::rgb(211, 84, 0), // dark orange
            parser::LandUseType::Industrial => Color::rgb(192, 57, 43), // dark red
            parser::LandUseType::Green => Color::rgb(39, 240, 96), // more green
            parser::LandUseType::Water => Color::rgb(41, 128, 185), // blue
            _ => Color::rgb(255, 255, 255), // white
        };

        c.a = 40;
        c
    }

    for lu in &world.loaded_land_uses {
        let colour = get_land_use_colour(&lu.land_use_type);
        let mut shape = ConvexShape::new(lu.points.len() as u32);
        shape.set_fill_color(&Color::TRANSPARENT);
        shape.set_outline_color(&colour);
        shape.set_outline_thickness(2.0);
        for (i, p) in lu.points.iter().enumerate() {
            shape.set_point(i as u32, Vector2f::new(p.x as f32, p.y as f32));
        }
        target.draw(&shape);
    }


    let mut backup_vec: Vec<Vertex> = Vec::new();
    let vertices = render_cache.unwrap_or(&mut backup_vec);
    for r in &world.loaded_roads {
        let colour = get_road_colour(&r.road_type);
        vertices.clear();
        vertices.extend(
            r.segments.iter().map(|s| {
                Vertex::with_pos_color(Vector2f::new(s.x as f32, s.y as f32), colour)
            })
            );
        target.draw_primitives(vertices, PrimitiveType::LineStrip, RenderStates::default());
    }
}

#[derive(Debug)]
struct CameraChange {
    x: f64,
    y: f64,
    w: u32,
    h: u32,

    z: f64,
    dz: f64,

    pub min_chunk: (i32, i32),
    pub max_chunk: (i32, i32),
    chunk_changes: Vec<ChunkChange>,
}

#[derive(Debug, Copy, Clone)]
struct ChunkChange {
    x: i32,
    y: i32,
    load: bool,
}

impl ChunkChange {
    fn new(x: i32, y: i32, load: bool) -> Self {
        ChunkChange { x, y, load }
    }
}

impl CameraChange {
    fn new(window_size: Vector2u, initial_zoom: f64) -> Self {
        CameraChange {
            x: 0.0,
            y: 0.0,
            w: window_size.x,
            h: window_size.y,
            dz: 0.0,
            z: initial_zoom,
            min_chunk: (0, 0),
            max_chunk: (0, 0),
            chunk_changes: Vec::new(),
        }
    }

    fn handle_key(&mut self, key: Key, pressed: bool) {
        let mult = if pressed { 1.0 } else { 0.0 };
        match key {
            Key::Q => self.dz = -ZOOM_SPEED * mult,
            Key::E => self.dz = ZOOM_SPEED * mult,
            Key::W => self.y = -MOVE_SPEED * mult,
            Key::S => self.y = MOVE_SPEED * mult,
            Key::A => self.x = -MOVE_SPEED * mult,
            Key::D => self.x = MOVE_SPEED * mult,
            _ => (),
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.w = width;
        self.h = height;
    }

    fn apply(&mut self, window: &mut RenderWindow, chunk_size: Vector2i) -> &Vec<ChunkChange> {
        let mut view = window.view().to_owned();

        if self.dz != 0.0 {
            // zoom in/out
            self.z += self.dz;
            self.z = f64::min(self.z, 4.5);
            self.z = f64::max(self.z, 0.25);
        }

        view.set_size(Vector2f::new(self.w as f32 * self.z as f32, self.h as f32 * self.z as f32));
        view.move_((self.x as f32, self.y as f32));
        window.set_view(&view);

        // chunks visible
        {
            self.chunk_changes.clear();

            let tl = window.map_pixel_to_coords(&Vector2i::new(0, 0), &view);
            let br = {
                let win_size = window.size();
                let size_i = Vector2i::new(win_size.x as i32, win_size.y as i32);
                window.map_pixel_to_coords(&size_i, &view)
            };

            let min_x = (tl.x /  chunk_size.x as f32).floor() as i32;
            let min_y = (tl.y /  chunk_size.y as f32).floor() as i32;
            let max_y = (br.y /  chunk_size.y as f32).floor() as i32;
            let max_x = (br.x /  chunk_size.x as f32).floor() as i32;

            let x_min_change = min_x - self.min_chunk.0;
            let x_max_change = max_x - self.max_chunk.0;
            let y_min_change = min_y - self.min_chunk.1;
            let y_max_change = max_y - self.max_chunk.1;

            // just one at a time for now
            assert!(x_min_change == 0 || x_min_change.abs() == 1);
            assert!(x_max_change == 0 || x_max_change.abs() == 1);
            assert!(y_max_change == 0 || y_max_change.abs() == 1);
            assert!(y_max_change == 0 || y_max_change.abs() == 1);

            if x_min_change != 0 {
                let (x, load) = if x_min_change < 0 {
                    (min_x, true)
                } else {
                    (min_x - x_min_change, false)
                };

                for y in min_y..max_y + 1 {
                    self.chunk_changes.push(ChunkChange::new(x, y, load));
                }
            }

            if x_max_change != 0 {
                let (x, load) = if x_max_change > 0 {
                    (max_x, true)
                } else {
                    (max_x - x_max_change, false)
                };

                for y in min_y..max_y + 1 {
                    self.chunk_changes.push(ChunkChange::new(x, y, load));
                }
            }

            if y_min_change != 0 {
                let (y, load) = if y_min_change < 0 {
                    (min_y, true)
                } else {
                    (min_y - y_min_change, false)
                };

                for x in min_x..max_x + 1 {
                    self.chunk_changes.push(ChunkChange::new(x, y, load));
                }
            }

            if y_max_change != 0 {
                let (y, load) = if y_max_change > 0 {
                    (max_y, true)
                } else {
                    (max_y - y_max_change, false)
                };

                for x in min_x..max_x + 1 {
                    self.chunk_changes.push(ChunkChange::new(x, y, load));
                }
            }


            self.min_chunk = (min_x, min_y);
            self.max_chunk = (max_x, max_y);
            &self.chunk_changes
        }
    }
}
