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

pub mod backend;
pub mod texture;

use backend::*;
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

    pub fn current_level(&self) -> &CurrentLevel {
        &self.collection.current_level
    }

    pub fn current_level_mut(&mut self) -> &mut CurrentLevel {
        &mut self.collection.current_level
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new("original")
    }
}

/// Map arrow keys to the corresponding directions, panic on other keys.
fn key_to_direction(key: Key) -> Direction {
    use self::Direction::*;
    match key {
        Key::Left => Left,
        Key::Right => Right,
        Key::Up => Up,
        Key::Down => Down,
        _ => panic!("Invalid direction key"),
    }
}

/// Draw the single tile with index `index`.
fn draw_entity(ctx: Context,
               g2d: &mut G2d,
               entity: &Texture<gfx_device_gl::Resources>,
               index: usize,
               app: &App) {
    let image_scale = app.tile_size / 360.0;
    let x = app.tile_size * (index % app.current_level().width()) as f64 + app.offset_left;
    let y = app.tile_size * (index / app.current_level().width()) as f64 + app.offset_top;
    image(entity,
          ctx.transform
              .trans(x, y)
              .scale(image_scale, image_scale),
          g2d);
}


/// Render the current level
fn render_level(c: Context,
                g: &mut G2d,
                app: &App,
                backgrounds: &HashMap<Background, Texture<gfx_device_gl::Resources>>,
                foregrounds: &HashMap<Foreground, Texture<gfx_device_gl::Resources>>) {

    // Set background
    clear(EMPTY, g);

    // Draw the background
    app.current_level()
        .level
        .background
        .iter()
        .enumerate()
        .filter(|&(_, cell)| cell != &Background::Empty)
        .map(|(i, cell)| draw_entity(c, g, &backgrounds[cell], i, app))
        .last();

    // and the foreground
    app.current_level()
        .level
        .foreground
        .iter()
        .enumerate()
        .filter(|&(_, cell)| cell != &Foreground::None)
        .map(|(i, cell)| draw_entity(c, g, &foregrounds[cell], i, app))
        .last();
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
        window.draw_2d(&e,
                       |c, g| render_level(c, g, &app, &backgrounds, &foregrounds));

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
                        if control_pressed != shift_pressed {
                            lvl.move_until(dir, shift_pressed)
                        } else {
                            let _ = lvl.try_move(dir);
                        }
                    }
                    Key::U => lvl.undo(),
                    Key::Z if control_pressed => {
                        if !shift_pressed {
                            lvl.undo();
                        } else {
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
                let x = ((cursor_pos[0] - app.offset_left) / app.tile_size).floor() as isize;
                let y = ((cursor_pos[1] - app.offset_top) / app.tile_size).floor() as isize;
                if x >= 0 && y >= 0 {
                    app.current_level_mut()
                        .move_to(backend::Position { x, y },
                                 mouse_button == MouseButton::Right);
                }
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

        // TODO find a nicer way to to this
        // FIXME frequently the size is wrong
        e.resize(|w, h| {
            let mut tile_size = app.tile_size;
            let mut horizontal_margins;
            let mut vertical_margins;
            {
                let lvl = &app.current_level().level;
                let width = lvl.width();
                let height = lvl.height();
                horizontal_margins = w as i32 - width as i32 * app.tile_size as i32;
                vertical_margins = h as i32 - height as i32 * app.tile_size as i32;

                if horizontal_margins < 0 || vertical_margins < 0 ||
                   horizontal_margins as usize > width && vertical_margins as usize > height {
                    tile_size = min(w / width as u32, h / height as u32) as f64;
                    horizontal_margins = w as i32 - width as i32 * app.tile_size as i32;
                    vertical_margins = h as i32 - height as i32 * app.tile_size as i32;
                }

            }
            app.tile_size = tile_size;
            app.offset_left = horizontal_margins as f64 / 2.0;
            app.offset_top = vertical_margins as f64 / 2.0;
        });
    }
}
