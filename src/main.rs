#![allow(dead_code)]
extern crate sfml;
extern crate libc;
extern crate chunk_req;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate c_str_macro;

use sfml::graphics::*;
use sfml::window::*;
use sfml::system::*;
use std::env;

mod world;
mod error;
mod parser;
mod latlon;

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

    let mut world = World::new(origin);

    let renderer = Renderer::new(500, 500, &mut world);
    renderer.start().unwrap();
}


fn test_chunk_loading() -> error::SimResult<()> {
    let mut w = World::new(LatLon::new(51.8972, -0.8543));

    let _first = w.request_chunk(0, 0)?;
    let _second = w.request_chunk(1, 0)?;

    Ok(())
}

struct Renderer<'a> {
    window: RenderWindow,
    world: &'a mut World,

    render_cache: Vec<Vertex>,
    chunk_size: Vector2i,
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
            chunk_size
        }
    }

    fn start(mut self) -> SimResult<()> {
        let mut cam = CameraChange::new(self.window.size());

        self.world.request_chunk(0, 0)?;

        let background_colour = Color::rgb(40, 40, 50);
        loop {
            while let Some(e) = self.window.poll_event() {
                match e {
                    Event::KeyPressed { code: Key::Escape, .. } => return Ok(()),
                    Event::KeyPressed { code, .. } => cam.handle_key(code, true),
                    Event::KeyReleased { code, .. } => cam.handle_key(code, false),
                    Event::Resized { width, height } => cam.resize(width, height),
                    _ => {}
                }
            }

            cam.apply(&mut self.window);

            self.window.clear(&background_colour);
            self.render_world();
            self.window.display();
        }
    }

    fn render_world(&mut self) {
        fn get_road_colour(road_type: &parser::RoadType) -> Color {
            match *road_type {
                parser::RoadType::Motorway |
                parser::RoadType::Primary |
                parser::RoadType::Secondary => Color::rgb(255, 50, 50), // red
                parser::RoadType::Minor => Color::rgb(50, 50, 255), // blue
                parser::RoadType::Pedestrian => Color::rgb(100, 100, 100), // grey
                parser::RoadType::Residential => Color::rgb(50, 255, 50), // green
                parser::RoadType::Unknown |
                _ => Color::rgb(255, 255, 255), // white
            }
        }


        for r in &self.world.loaded_roads {
            let colour = get_road_colour(&r.road_type);
            self.render_cache.clear();
            self.render_cache.extend(
                r.segments.iter().map(|s| {
                    Vertex::with_pos_color(Vector2f::new(s.x as f32, s.y as f32), colour)
                })
            );
            self.window.draw_primitives(&self.render_cache, PrimitiveType::LineStrip, RenderStates::default());
        }

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
        for x in -3..3 {
            for y in -3..3 {
                rect.set_position((
                    (x * self.chunk_size.x) as f32,
                    (y * self.chunk_size.y) as f32)
                );
                self.window.draw(&rect);
            }
        }
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
}

impl CameraChange {
    fn new(window_size: Vector2u) -> Self {
        CameraChange {
            x: 0.0,
            y: 0.0,
            w: window_size.x,
            h: window_size.y,
            dz: 0.0,
            z: 1.0
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

    fn apply(&mut self, window: &mut RenderWindow) {
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
    }
}
