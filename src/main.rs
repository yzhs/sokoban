#![feature(try_from)]

// GUI
extern crate piston;
extern crate piston_window;
extern crate graphics;
extern crate gfx_graphics;
extern crate gfx_core;
extern crate gfx_device_gl;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

use std::collections::HashMap;

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
    pub fn new(collection_name: &str) -> App {
        let collection = Collection::load(collection_name);
        if collection.is_err() {
            panic!("Failed to load level set: {:?}", collection.unwrap_err());
        }
        let collection = collection.unwrap();
        App { collection }
    }

    pub fn current_level(&self) -> &Level {
        &self.collection.current_level
    }

    pub fn current_level_mut(&mut self) -> &mut Level {
        &mut self.collection.current_level
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new("original")
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

fn render_level(c: Context,
                g: &mut G2d,
                level: &Level,
                backgrounds: &HashMap<Background, Texture<gfx_device_gl::Resources>>,
                foregrounds: &HashMap<Foreground, Texture<gfx_device_gl::Resources>>) {
    // Set background
    clear(EMPTY, g);

    // Render the current level
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
}

fn main() {
    let mut app = App::new("microban");
    info!("{}", app.current_level());

    let title = "Sokoban";
    let mut window: PistonWindow =
        WindowSettings::new(title, [640, 480])
            .exit_on_esc(true)
            .build()
            .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

    window.set_lazy(true);

    // Initialize colog after window to suppress some log output.
    colog::init();

    let mut cursor_pos = [0.0, 0.0];

    let backgrounds = load_backgrounds(&mut window.factory);
    let foregrounds = load_foregrounds(&mut window.factory);

    while let Some(e) = window.next() {
        window.draw_2d(&e,
                       |c, g| render_level(c, g, app.current_level(), &backgrounds, &foregrounds));

        // Keep track of where the cursor is pointing
        if let Some(new_pos) = e.mouse_cursor_args() {
            cursor_pos = new_pos;
        }

        // Handle key press events
        match e.press_args() {
            None => {}
            Some(Button::Keyboard(key)) => {
                match key {
                    Key::Left | Key::Right | Key::Up | Key::Down => {
                        let _ = app.current_level_mut().try_move(key_to_direction(key));
                    }
                    Key::U => app.current_level_mut().undo(),
                    Key::Escape => {} // Closing app, nothing to do here
                    _ => error!("Unkown key: {:?}", key),
                }
            }
            Some(Button::Mouse(mouse_button)) => {
                let x = (cursor_pos[0] / TILE_SIZE) as usize;
                let y = (cursor_pos[1] / TILE_SIZE) as usize;
                app.current_level_mut().move_to((x, y), mouse_button == MouseButton::Right);
            }
            Some(x) => error!("Unkown event: {:?}", x),
        };

        if app.current_level().is_finished() {
            info!("Level solved!");
            app.collection.next_level();
        }
    }
}
