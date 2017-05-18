// GUI
extern crate piston_window;
extern crate gfx_graphics;
extern crate gfx_core;
extern crate sprite;
extern crate uuid;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

extern crate sokoban;

use std::cmp::min;
use std::rc::Rc;

use piston_window::*;
use sprite::{Scene, Sprite};
use uuid::Uuid;

pub mod texture;

use sokoban::*;

const EMPTY: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // black

pub struct App {
    collection: Collection,
    cursor_pos: [f64; 2],
    shift_pressed: bool,
    control_pressed: bool,
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
            shift_pressed: false,
            control_pressed: false,
            offset_left: 0,
            cursor_pos: [0.0, 0.0],
            offset_top: 0,
        }
    }

    pub fn current_level(&self) -> &Level {
        &self.collection.current_level
    }

    pub fn current_level_mut(&mut self) -> &mut Level {
        &mut self.collection.current_level
    }

    /// Update the tile size and offsets such that the level fills most of the window.
    pub fn update_size(&mut self, size: &[u32; 2]) {
        let width = size[0] as i32;
        let height = size[1] as i32;
        let columns = self.current_level().columns() as i32;
        let rows = self.current_level().rows() as i32;
        self.tile_size = min(width / columns, height / rows);
        self.offset_left = (width - columns * self.tile_size) / 2;
        self.offset_top = (height - rows * self.tile_size) / 2;
    }

    /// Handle press event.
    fn press_to_command(&mut self, args: Button) -> Command {
        use Command::*;
        match args {
            Button::Keyboard(key) => {
                match key {
                    Key::Left | Key::Right | Key::Up | Key::Down => {
                        let dir = key_to_direction(key);
                        if self.control_pressed == self.shift_pressed {
                            Move(dir)
                        } else {
                            MoveAsFarAsPossible(dir, MayPushCrate(self.shift_pressed))
                        }
                    }
                    Key::Z if !self.control_pressed => Nothing,
                    Key::U if self.control_pressed => Nothing,
                    Key::U | Key::Z if self.shift_pressed => Redo,
                    Key::U | Key::Z => Undo,

                    Key::LCtrl | Key::RCtrl => {
                        self.control_pressed = true;
                        Nothing
                    }
                    Key::LShift | Key::RShift => {
                        self.shift_pressed = true;
                        Nothing
                    }

                    Key::Escape => Nothing,// Closing app, nothing to do here
                    _ => {
                        error!("Unkown key: {:?}", key);
                        Nothing
                    }
                }
            }
            Button::Mouse(mouse_button) => {
                let x = ((self.cursor_pos[0] as i32 - self.offset_left) / self.tile_size) as isize;
                let y = ((self.cursor_pos[1] as i32 - self.offset_top) / self.tile_size) as isize;
                if x >= 0 && y >= 0 {
                    MoveToPosition(sokoban::Position { x, y },
                                   MayPushCrate(mouse_button == MouseButton::Right))
                } else {
                    Nothing
                }
            }
            x => {
                error!("Unkown event: {:?}", x);
                Nothing
            }
        }
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

/// All tiles face left by default, so the worker has to turned by 90 degrees (clockwise) to face
/// up instead of left, etc.
fn direction_to_angle(dir: Direction) -> f64 {
    match dir {
        Direction::Left => 0.0,
        Direction::Right => 180.0,
        Direction::Up => 90.0,
        Direction::Down => 270.0,
    }
}

/// Create a `Scene` containing the level’s background.
fn background_to_scene<R, F>(factory: &mut F, app: &App) -> (Scene<Texture<R>>, Vec<Uuid>, Uuid)
    where R: gfx_core::Resources,
          F: gfx_core::Factory<R>
{
    // Load the textures
    let empty_tex = Rc::new(texture::load(factory, "empty"));
    let wall_tex = Rc::new(texture::load(factory, "wall"));
    let floor_tex = Rc::new(texture::load(factory, "floor"));
    let goal_tex = Rc::new(texture::load(factory, "goal"));
    let worker_tex = Rc::new(texture::load(factory, "worker"));
    let crate_tex = Rc::new(texture::load(factory, "crate"));

    let lvl = app.current_level();
    let tile_size = app.tile_size as f64;
    let image_scale = tile_size / 360.0;
    let columns = lvl.columns();

    let mut scene = Scene::new();

    // Create sprites for the level’s background.
    for (i, cell) in app.current_level().background.iter().enumerate() {
        let tex = match *cell {
            Background::Empty => empty_tex.clone(),
            Background::Floor => floor_tex.clone(),
            Background::Goal => goal_tex.clone(),
            Background::Wall => wall_tex.clone(),
        };
        let mut sprite = Sprite::from_texture(tex);
        let x = tile_size * ((i % columns) as f64 + 0.5);
        let y = tile_size * ((i / columns) as f64 + 0.5);
        sprite.set_position(x, y);
        sprite.set_scale(image_scale, image_scale);
        scene.add_child(sprite);
    }

    // Create sprites for all crates in their initial position.
    let mut tmp: Vec<_> = app.current_level().crates.iter().collect();
    tmp.sort_by_key(|x| x.1);
    let mut crate_ids = vec![];
    for (&sokoban::Position { x, y }, _) in tmp {
        let mut sprite = Sprite::from_texture(crate_tex.clone());
        let x = tile_size * (x as f64 + 0.5);
        let y = tile_size * (y as f64 + 0.5);
        sprite.set_position(x, y);
        sprite.set_scale(image_scale, image_scale);
        crate_ids.push(scene.add_child(sprite));
    }

    // Create the worker sprite.
    let mut sprite = Sprite::from_texture(worker_tex.clone());
    let sokoban::Position { x, y } = app.current_level().worker_position;
    let x = tile_size * (x as f64 + 0.5);
    let y = tile_size * (y as f64 + 0.5);
    sprite.set_scale(image_scale, image_scale);
    sprite.set_position(x, y);
    sprite.set_rotation(direction_to_angle(app.current_level().worker_direction()));
    let worker_id = scene.add_child(sprite);

    (scene, crate_ids, worker_id)
}

/// Move the sprite with the given `id` to position `pos`.
fn set_position<I: ImageSize>(scene: &mut Scene<I>,
                              id: Uuid,
                              pos: sokoban::Position,
                              tile_size: f64) {
    let sokoban::Position { x, y } = pos;
    let (x, y) = (tile_size as f64 * (x as f64 + 0.5), tile_size as f64 * (y as f64 + 0.5));

    scene
        .child_mut(id)
        .map(|sprite| sprite.set_position(x, y));
}

/// Rotate the sprite by the given angle in degrees.
fn set_rotation<I: ImageSize>(scene: &mut Scene<I>, id: Uuid, angle: f64) {
    scene
        .child_mut(id)
        .map(|sprite| sprite.set_rotation(angle));
}

fn main() {
    let mut app: App = Default::default();
    info!("{}", app.current_level());

    let title = "Sokoban";
    let mut window_size = [640, 480];
    let mut window: PistonWindow =
        WindowSettings::new(title, window_size)
            .exit_on_esc(true)
            .build()
            .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

    window.set_lazy(true);

    // Initialize colog after window to suppress some log output.
    colog::init();


    let (mut scene, mut crate_ids, mut worker_id) = background_to_scene(&mut window.factory, &app);

    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g| {
            // Set background
            // TODO background image?
            clear(EMPTY, g);
            scene.draw(c.transform
                           .trans(app.offset_left as f64, app.offset_top as f64),
                       g);
            // TODO update crate and worker position
        });

        // Keep track of where the cursor is pointing
        if let Some(new_pos) = e.mouse_cursor_args() {
            app.cursor_pos = new_pos;
        }

        // Handle key press
        let mut command = Command::Nothing;
        e.press(|args| command = app.press_to_command(args));

        // and release events
        if let Some(Button::Keyboard(key)) = e.release_args() {
            match key {
                Key::LCtrl | Key::RCtrl => app.control_pressed = false,
                Key::LShift | Key::RShift => app.shift_pressed = false,
                _ => {}
            }
        }

        // Handle the response from the backend.
        for response in app.collection.execute(command) {
            match response {
                Response::NewLevel(rank) => info!("Switched to level #{}", rank),
                Response::MoveWorkerTo(pos, dir) => {
                    set_position(&mut scene, worker_id, pos, app.tile_size as f64);
                    set_rotation(&mut scene, worker_id, direction_to_angle(dir))
                }
                Response::MoveCrateTo(i, pos) => {
                    set_position(&mut scene, crate_ids[i], pos, app.tile_size as f64);
                }
            }
        }

        // If the level has been solved, display a message and go to the next level.
        // TODO display message
        if app.current_level().is_finished() {
            use NextLevelError::*;
            {
                let lvl = app.current_level();
                info!("Level solved using {} moves, {} of which moved a crate.",
                      lvl.number_of_moves(),
                      lvl.number_of_pushes());
                info!("Solution: {}", lvl.moves_to_string());
            }
            match app.collection.next_level() {
                Ok(ref resp) if resp.len() == 1 => {
                    if let Response::NewLevel(_) = resp[0] {
                        app.update_size(&window_size);
                        let tmp = background_to_scene(&mut window.factory, &app);
                        scene = tmp.0;
                        crate_ids = tmp.1;
                        worker_id = tmp.2;
                    } else {
                        error!("Invalid response: {:?}", resp);
                    }
                }
                Ok(resp) => error!("Invalid response: {:?}", resp),
                Err(EndOfCollection) => error!("Reached the end of the current collection."),
                Err(LevelNotFinished) => error!("Current level is not finished!"),
            }
        }

        // TODO find a nicer way to to this
        // FIXME frequently the size is wrong
        if let Some(size) = e.resize_args() {
            window_size = size;
            app.update_size(&window_size);
            let tmp = background_to_scene(&mut window.factory, &app);
            scene = tmp.0;
            crate_ids = tmp.1;
            worker_id = tmp.2;
        }
    }
}
