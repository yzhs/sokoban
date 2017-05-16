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

extern crate sokoban;

use std::cmp::min;
use std::collections::HashMap;

use piston_window::*;

pub mod texture;

use sokoban::*;
use texture::*;


const EMPTY: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // black

pub struct App {
    collection: Collection,
    tile_size: i32,
    offset_left: i32,
    offset_top: i32,
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
            tile_size: 50,
            offset_left: 0,
            offset_top: 0,
        }
    }

    pub fn current_level(&self) -> &Level {
        &self.collection.current_level
    }

    pub fn current_level_mut(&mut self) -> &mut Level {
        &mut self.collection.current_level
    }

    pub fn update_size(&mut self, size: &[u32; 2]) {
        let width = size[0] as i32;
        let height = size[1] as i32;
        let columns = self.current_level().width() as i32;
        let rows = self.current_level().height() as i32;
        self.tile_size = min(width / columns, height / rows);
        self.offset_left = (width - columns * self.tile_size) / 2;
        self.offset_top = (height - rows * self.tile_size) / 2;
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

/// Render the current level
fn render_level(ctx: Context,
                g2d: &mut G2d,
                app: &App,
                backgrounds: &HashMap<Background, Texture<gfx_device_gl::Resources>>,
                foregrounds: &HashMap<Foreground, Texture<gfx_device_gl::Resources>>) {

    // Set background
    clear(EMPTY, g2d);
    // TODO background image?

    let width = app.current_level().width();
    let tile_size = app.tile_size as f64;
    let image_scale = tile_size / 360.0;
    let offset_left = app.offset_left as f64;
    let offset_top = app.offset_top as f64;

    // Draw the background
    for (i, cell) in app.current_level().background.iter().enumerate() {
        if cell == &Background::Empty {
            continue;
        }
        let x = tile_size * (i % width) as f64 + offset_left;
        let y = tile_size * (i / width) as f64 + offset_top;
        image(&backgrounds[cell],
              ctx.transform
                  .trans(x, y)
                  .scale(image_scale, image_scale),
              g2d);
    }

    // Draw the crates
    for pos in app.current_level().crates.iter() {
        let x = tile_size * pos.x as f64 + offset_left;
        let y = tile_size * pos.y as f64 + offset_top;
        image(&foregrounds[&Foreground::Crate],
              ctx.transform
                  .trans(x, y)
                  .scale(image_scale, image_scale),
              g2d);
    }

    // Draw the worker
    let pos = app.current_level().worker_position;
    let worker_direction = match app.current_level().worker_direction() {
        Direction::Left => 0.0,
        Direction::Right => 180.0,
        Direction::Up => 90.0,
        Direction::Down => 270.0,
    };
    let x = tile_size * pos.x as f64 + offset_left;
    let y = tile_size * pos.y as f64 + offset_top;
    image(&foregrounds[&Foreground::Worker],
          ctx.transform
              .trans(x + tile_size / 2.0, y + tile_size / 2.0)
              .rot_deg(worker_direction)
              .trans(-tile_size / 2.0, -tile_size / 2.0)
              .scale(image_scale, image_scale),
          g2d);

}

fn main() {
    let mut app: App = Default::default();
    info!("{}", app.current_level());

    let title = "Sokoban";
    let mut window_size = [640, 480];
    let mut window: PistonWindow =
        WindowSettings::new(title, window_size.clone())
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
        use Command::*;
        let command = match e.press_args() {
            None => Nothing,
            Some(Button::Keyboard(key)) => {
                match key {
                    Key::Left | Key::Right | Key::Up | Key::Down => {
                        let dir = key_to_direction(key);
                        if control_pressed != shift_pressed {
                            MoveAsFarAsPossible(dir, MayPushCrate(shift_pressed))
                        } else {
                            Move(dir)
                        }
                    }
                    Key::Z if !control_pressed => Nothing,
                    Key::U if control_pressed => Nothing,
                    Key::U | Key::Z if shift_pressed => Redo,
                    Key::U | Key::Z => Undo,

                    Key::LCtrl | Key::RCtrl => {
                        control_pressed = true;
                        Nothing
                    }
                    Key::LShift | Key::RShift => {
                        shift_pressed = true;
                        Nothing
                    }

                    Key::Escape => Nothing,// Closing app, nothing to do here
                    _ => {
                        error!("Unkown key: {:?}", key);
                        Nothing
                    }
                }
            }
            Some(Button::Mouse(mouse_button)) => {
                let x = ((cursor_pos[0] as i32 - app.offset_left) / app.tile_size) as isize;
                let y = ((cursor_pos[1] as i32 - app.offset_top) / app.tile_size) as isize;
                if x >= 0 && y >= 0 {
                    MoveToPosition(sokoban::Position { x, y },
                                   MayPushCrate(mouse_button == MouseButton::Right))
                } else {
                    Nothing
                }
            }
            Some(x) => {
                error!("Unkown event: {:?}", x);
                Nothing
            }
        };
        app.collection.execute(command);

        if let Some(Button::Keyboard(key)) = e.release_args() {
            match key {
                Key::LCtrl | Key::RCtrl => control_pressed = false,
                Key::LShift | Key::RShift => shift_pressed = false,
                _ => {}
            }
        }

        if app.current_level().is_finished() {
            {
                let lvl = app.current_level();
                info!("Level solved using {} moves, {} of which moved a crate.",
                      lvl.number_of_moves(),
                      lvl.number_of_pushes());
                info!("Solution: {}", lvl.moves_to_string());
            }
            use NextLevelError::*;
            match app.collection.next_level() {
                Ok(()) => app.update_size(&window_size),
                Err(EndOfCollection) => error!("Reached the end of the current collection."),
                Err(LevelNotFinished) => error!("Current level is not finished!"),
            }
        }

        // TODO find a nicer way to to this
        // FIXME frequently the size is wrong
        if let Some(size) = e.resize_args() {
            window_size = size;
            app.update_size(&window_size);
        }
    }
}
