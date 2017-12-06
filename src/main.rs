extern crate sfml;
extern crate libc;
extern crate reqwest;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate c_str_macro;

use sfml::graphics::*;
use sfml::window::*;

mod world;
mod error;
mod parser;

use world::*;

fn main() {
    test_chunk_loading();
}


fn test_chunk_loading() -> error::SimResult<()> {
    let w = World::new(LatLon::new(52.450817, -1.930513));
    let a = w.request_chunk(0, 0)?;
    let b = w.request_chunk(0, 1)?;
    let b = w.request_chunk(2, 2)?;
    Ok(())
}

fn start_renderer() {
    let mut window = RenderWindow::new(
        (300, 300),
        "Hiya",
        Style::NONE,
        &Default::default()
    );

    window.set_framerate_limit(60);

    let background_colour = Color::rgb(40, 40, 50);
    loop {
        while let Some(e) = window.poll_event() {
            match e {
                Event::KeyPressed { code, .. } => {
                    if code == Key::Escape {
                        return;
                    }
                }
                _ => {}
            }
        }

        window.clear(&background_colour);

        window.display();
    }
}
