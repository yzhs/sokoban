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

use std::cmp::min;
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

pub struct App {
    collection: Collection,
    tile_size: f64,
    offset_left: f64,
    offset_top: f64,
}

impl App {
    pub fn new(collection_name: &str) -> App {
        let collection = Collection::load(collection_name);
        if collection.is_err() {
            panic!("Failed to load level set: {:?}", collection.unwrap_err());
        }
        let collection = collection.unwrap();
        App {
            collection,
            tile_size: 50.0,
            offset_left: 0.0,
            offset_top: 0.0,
        }
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

fn draw_entity(c: Context,
               g: &mut G2d,
               entity: &Texture<gfx_device_gl::Resources>,
               i: usize,
               width: usize,
               tile_size: f64,
               image_scale: f64,
               offset_left: f64,
               offset_top: f64) {
    let x = tile_size * (i % width) as f64 + offset_left;
    let y = tile_size * (i / width) as f64 + offset_top;
    image(entity,
          c.transform.trans(x, y).scale(image_scale, image_scale),
          g);
}

fn render_level(c: Context,
                g: &mut G2d,
                level: &Level,
                tile_size: f64,
                offset_left: f64,
                offset_top: f64,
                backgrounds: &HashMap<Background, Texture<gfx_device_gl::Resources>>,
                foregrounds: &HashMap<Foreground, Texture<gfx_device_gl::Resources>>) {
    let image_scale = tile_size / 360.0;

    // Set background
    clear(EMPTY, g);

    // Render the current level
    let background = &level.background;

    // Draw the background
    for (i, bg) in background.iter().enumerate() {
        if bg == &Background::Empty {
            continue;
        }
        draw_entity(c,
                    g,
                    &backgrounds[bg],
                    i,
                    level.width,
                    tile_size,
                    image_scale,
                    offset_left,
                    offset_top);
    }

    // and the foreground
    let foreground = &level.foreground;
    for (i, fg) in foreground.iter().enumerate() {
        if fg == &Foreground::None {
            continue;
        }
        draw_entity(c,
                    g,
                    &foregrounds[fg],
                    i,
                    level.width,
                    tile_size,
                    image_scale,
                    offset_left,
                    offset_top);
    }
}

fn main() {
    let mut app: App = Default::default();
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

    let mut control_pressed = false;
    let mut shift_pressed = false;

    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g| {
            render_level(c,
                         g,
                         app.current_level(),
                         app.tile_size,
                         app.offset_left,
                         app.offset_top,
                         &backgrounds,
                         &foregrounds)
        });

        // Keep track of where the cursor is pointing
        if let Some(new_pos) = e.mouse_cursor_args() {
            cursor_pos = new_pos;
        }

        // Handle key press events
        match e.press_args() {
            None => {}
            Some(Button::Keyboard(key)) => {
                let mut lvl = app.current_level_mut();
                match key {
                    Key::Left | Key::Right | Key::Up | Key::Down => {
                        let dir = key_to_direction(key);
                        if control_pressed && !shift_pressed || !control_pressed && shift_pressed {
                            lvl.move_until(dir, shift_pressed)
                        } else {
                            let _ = lvl.try_move(dir);
                        }
                    }
                    Key::U => lvl.undo(),
                    Key::Z => {
                        if control_pressed && !shift_pressed {
                            lvl.undo();
                        } else if control_pressed {
                            lvl.redo();
                        }
                    }

                    Key::LCtrl | Key::RCtrl => control_pressed = true,
                    Key::LShift | Key::RShift => shift_pressed = true,

                    Key::Escape => {} // Closing app, nothing to do here
                    _ => error!("Unkown key: {:?}", key),
                }
            }
            Some(Button::Mouse(mouse_button)) => {
                let x = (cursor_pos[0] / app.tile_size) as usize;
                let y = (cursor_pos[1] / app.tile_size) as usize;
                app.current_level_mut()
                    .move_to((x, y), mouse_button == MouseButton::Right);
            }
            Some(x) => error!("Unkown event: {:?}", x),
        };

        if let Some(Button::Keyboard(key)) = e.release_args() {
            match key {
                Key::LCtrl | Key::RCtrl => control_pressed = false,
                Key::LShift | Key::RShift => shift_pressed = false,
                _ => {}
            }
        }

        if app.current_level().is_finished() {
            info!("Level solved!");
            app.collection.next_level();
        }

        e.resize(|w, h| {
            let mut tile_size = app.tile_size;
            let mut horizontal_margins;
            let mut vertical_margins;
            {
                let lvl = app.current_level();
                horizontal_margins = w as i32 - lvl.width as i32 * app.tile_size as i32;
                vertical_margins = h as i32 - lvl.height as i32 * app.tile_size as i32;

                if horizontal_margins < 0 || vertical_margins < 0 ||
                   horizontal_margins as usize > lvl.width &&
                   vertical_margins as usize > lvl.height {
                    tile_size = min(w / lvl.width as u32, h / lvl.height as u32) as f64;
                    horizontal_margins = w as i32 - lvl.width as i32 * app.tile_size as i32;
                    vertical_margins = h as i32 - lvl.height as i32 * app.tile_size as i32;
                }

            }
            app.tile_size = tile_size;
            app.offset_left = horizontal_margins as f64 / 2.0;
            app.offset_top = vertical_margins as f64 / 2.0;
        });
    }
}
