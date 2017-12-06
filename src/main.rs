extern crate sfml;

use sfml::graphics::*;
use sfml::window::*;

fn main() {
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
