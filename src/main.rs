extern crate sfml;
extern crate libc;
extern crate reqwest;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate c_str_macro;

use sfml::graphics::*;
use sfml::window::*;
use std::ops::Div;

mod world;
mod error;
mod parser;

use world::*;

fn main() {
    start_renderer()
}


fn test_chunk_loading() -> error::SimResult<()> {
    let mut w = World::new(LatLon::new(51.8972, -0.8543));

    let first = w.request_chunk(0, 0)?;
    let second = w.request_chunk(1, 0)?;

    Ok(())
}

fn start_renderer() {
    let mut window = RenderWindow::new(
        (500, 500),
        "Hiya",
        Style::NONE,
        &Default::default()
    );

    window.set_framerate_limit(60);

    let mut cam = CameraChange::new(window.size());
    let test = {
        let mut c = CircleShape::new(10.0, 20);
        c.set_fill_color(&Color::RED);
        c
    };

    let background_colour = Color::rgb(40, 40, 50);
    loop {
        while let Some(e) = window.poll_event() {
            match e {
                Event::KeyPressed { code: Key::Escape, .. } => return,
                Event::KeyPressed { code, .. } => cam.handle_key(code, true),
                Event::KeyReleased { code, .. } => cam.handle_key(code, false),
                Event::Resized {width, height} => cam.resize(width, height),
                _ => {}
            }
        }

        cam.apply(&mut window);

        window.clear(&background_colour);
        window.draw(&test);
        window.display();
    }
}

#[derive(Debug)]
struct CameraChange {
    x: f32,
    y: f32,
    w: u32,
    h: u32,
}

const MOVE_SPEED: f32 = 5.0;

impl CameraChange {
    fn new(window_size: sfml::system::Vector2u) -> Self {
        CameraChange {
            x: 0.0,
            y: 0.0,
            w: window_size.x,
            h: window_size.y,
        }
    }
    fn handle_key(&mut self, key: Key, pressed: bool) {
        let mult = if pressed { 1.0 } else { 0.0 };
        match key {
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
        view.move_((self.x, self.y));
        window.set_view(&view);
    }
}
