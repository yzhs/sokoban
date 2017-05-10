#![feature(try_from)]

// GUI
extern crate piston;
extern crate piston_window;
extern crate graphics;
extern crate gfx_graphics;
extern crate gfx_core;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

use piston_window::*;

pub mod cell;
pub mod collection;
pub mod direction;
pub mod level;
pub mod move_;
pub mod util;

pub mod texture;

use cell::*;
use collection::*;
use level::*;

use texture::*;


const EMPTY: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // black
const TILE_SIZE: f64 = 50.0;
const IMAGE_SCALE: f64 = TILE_SIZE / 360.0;

pub struct App {
    collection: Collection,
}

impl App {
    pub fn new() -> App {
        let collection = Collection::load("original");
        if collection.is_err() {
            panic!("Failed to load level set: {:?}", collection.unwrap_err());
        }
        let collection = collection.unwrap();

        App {
            collection: collection,
        }
    }

    pub fn current_level(&mut self) -> &mut Level {
        &mut self.collection.current_level
    }
}

fn key_to_direction(key: Key) -> direction::Direction {
    use direction::Direction::*;
    match key {
        Key::Left => Left,
        Key::Right => Right,
        Key::Up => Up,
        Key::Down => Down,
        _ => panic!("Invalid direction key"),
    }
}


fn main() {
    colog::init();
    let mut app = App::new();
    info!("{}", app.current_level());

    let title = "Sokoban";
    let mut window: PistonWindow =
        WindowSettings::new(title, [640, 480])
            .exit_on_esc(true)
            .build()
            .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

    window.set_lazy(true);

    let backgrounds = load_backgrounds(&mut window.factory);
    let foregrounds = load_foregrounds(&mut window.factory);

    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g| {
            // Set background
            clear(EMPTY, g);

            // Render the current level
            let level = &app.current_level();
            let background = &level.background;

            // Draw the background
            for (i, bg) in background.iter().enumerate() {
                if bg == &Background::Empty {
                    continue;
                }

                let x = TILE_SIZE * (i % level.width) as f64;
                let y = TILE_SIZE * (i / level.width) as f64;
                image(&backgrounds[bg],
                      c.transform.trans(x, y).scale(IMAGE_SCALE, IMAGE_SCALE),
                      g);
            }

            // and the foreground
            let foreground = &level.foreground;
            for (i, fg) in foreground.iter().enumerate() {
                if fg == &Foreground::None {
                    continue;
                }

                let x = TILE_SIZE * (i % level.width) as f64;
                let y = TILE_SIZE * (i / level.width) as f64;
                image(&foregrounds[fg],
                      c.transform.trans(x, y).scale(IMAGE_SCALE, IMAGE_SCALE),
                      g);
            }
        });

        match e.press_args() {
            None => {}
            Some(Button::Keyboard(key)) => match key {
                    Key::Left | Key::Right | Key::Up | Key::Down => {
                        let _ = app.current_level().try_move(key_to_direction(key));
                    },
                    Key::U => app.current_level().undo(),
                    _ => error!("Unkown key: {:?}", key),
                },
            Some(x) => error!("Unkown event: {:?}", x),
        };

        if app.current_level().is_finished() {
            info!("Level solved!");
            app.collection.next_level();
        }
    }
}
