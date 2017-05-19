// GUI
extern crate piston_window;
extern crate graphics;
extern crate gfx_core;
extern crate gfx_device_gl;
extern crate gfx_graphics;
extern crate sprite;
extern crate uuid;
extern crate find_folder;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

extern crate sokoban;

use std::cmp::min;
use std::path::PathBuf;
use std::rc::Rc;

use graphics::rectangle;
use graphics::character::CharacterCache;
use piston_window::*;
use sprite::{Scene, Sprite};
use uuid::Uuid;

pub mod texture;

use sokoban::*;

const EMPTY: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // black

pub struct Game {
    assets: PathBuf,
    collection: Collection,
    cursor_pos: [f64; 2],
    shift_pressed: bool,
    control_pressed: bool,
    tile_size: i32,
    offset_left: i32,
    offset_top: i32,
}

impl Game {
    pub fn new(collection_name: &str) -> Self {
        let assets = find_folder::Search::ParentsThenKids(3, 3)
            .for_folder("assets")
            .unwrap();
        let collection = Collection::load(&assets, collection_name);
        if collection.is_err() {
            panic!("Failed to load level set: {:?}", collection.unwrap_err());
        }
        let collection = collection.unwrap();
        Game {
            assets,
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

                    Key::Escape => Nothing,// Closing game, nothing to do here
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

impl Default for Game {
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
fn generate_level_scene<R, F>(factory: &mut F, game: &Game) -> (Scene<Texture<R>>, Vec<Uuid>, Uuid)
    where R: gfx_core::Resources,
          F: gfx_core::Factory<R>
{
    // Load the textures
    let empty_tex = Rc::new(texture::load(factory, "empty", &game.assets));
    let wall_tex = Rc::new(texture::load(factory, "wall", &game.assets));
    let floor_tex = Rc::new(texture::load(factory, "floor", &game.assets));
    let goal_tex = Rc::new(texture::load(factory, "goal", &game.assets));
    let worker_tex = Rc::new(texture::load(factory, "worker", &game.assets));
    let crate_tex = Rc::new(texture::load(factory, "crate", &game.assets));

    let lvl = game.current_level();
    let tile_size = game.tile_size as f64;
    let image_scale = tile_size / 360.0;
    let columns = lvl.columns();

    let mut scene = Scene::new();

    // Create sprites for the level’s background.
    for (i, cell) in game.current_level().background.iter().enumerate() {
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
    let mut tmp: Vec<_> = game.current_level().crates.iter().collect();
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
    let sokoban::Position { x, y } = game.current_level().worker_position;
    let x = tile_size * (x as f64 + 0.5);
    let y = tile_size * (y as f64 + 0.5);
    sprite.set_scale(image_scale, image_scale);
    sprite.set_position(x, y);
    sprite.set_rotation(direction_to_angle(game.current_level().worker_direction()));
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

fn draw_end_of_level_screen<C, G>(c: &Context,
                                  g: &mut G,
                                  glyphs: &mut C,
                                  window_size: [u32; 2],
                                  game: &Game)
    where C: CharacterCache,
          G: Graphics<Texture = <C as CharacterCache>::Texture>
{
    let rectangle = Rectangle::new([0.0, 0.0, 0.0, 0.7]);
    let dims = rectangle::centered([0.0, 0.0, window_size[0] as f64, window_size[1] as f64]);
    rectangle.draw(dims, &c.draw_state, c.transform, g);

    let lvl = game.current_level();
    let rank = lvl.rank;
    let moves = lvl.number_of_moves();
    let pushes = lvl.number_of_pushes();

    let heading = text::Text::new_color(color::WHITE, 32 as types::FontSize);
    heading.draw("Congratulations!",
                 glyphs,
                 &c.draw_state,
                 c.transform
                     .trans(-150.0, -32.0)
                     .trans(window_size[0] as f64 / 2.0, window_size[1] as f64 / 2.0),
                 g);

    let txt = text::Text::new_color(color::WHITE, 16 as types::FontSize);
    let msg = format!("You have solved level {rank} with {moves} \
                                  moves, {pushes} of which moved a crate.",
                      rank = rank,
                      moves = moves,
                      pushes = pushes);

    txt.draw(&msg,
             glyphs,
             &c.draw_state,
             c.transform
                 .trans(-270.0, 0.0)
                 .trans(window_size[0] as f64 / 2.0, window_size[1] as f64 / 2.0),
             g);

    txt.draw("Press any key to continue",
             glyphs,
             &c.draw_state,
             c.transform
                 .trans(-120.0, 120.0)
                 .trans(window_size[0] as f64 / 2.0, window_size[1] as f64 / 2.0),
             g);
}

fn main() {
    let mut game: Game = Default::default();
    info!("{}", game.current_level());

    let title = "Sokoban";
    let mut window_size = [640, 480];
    let mut level_solved = false;
    let mut window: PistonWindow =
        WindowSettings::new(title, window_size)
            .exit_on_esc(true)
            .build()
            .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

    window.set_lazy(true);

    // Initialize colog after window to suppress some log output.
    colog::init();

    let font = &game.assets.clone().join("FiraSans-Regular.ttf");
    let mut glyphs = Glyphs::new(font, window.factory.clone()).unwrap();

    let (mut scene, mut crate_ids, mut worker_id) = generate_level_scene(&mut window.factory,
                                                                         &game);

    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g| {
            // Set background
            clear(EMPTY, g);

            // Draw the level.
            scene.draw(c.transform
                           .trans(game.offset_left as f64, game.offset_top as f64),
                       g);

            // Overlay message about solving the level.
            if level_solved {
                draw_end_of_level_screen(&c, g, &mut glyphs, window_size, &game);
            }
        });

        // Keep track of where the cursor is pointing
        if let Some(new_pos) = e.mouse_cursor_args() {
            game.cursor_pos = new_pos;
        }

        // Handle key press
        let mut command = Command::Nothing;
        if level_solved {
            command = match e.press_args() {
                Some(_) => Command::NextLevel,
                None => Command::Nothing,
            }
        } else {
            e.press(|args| command = game.press_to_command(args));
        }

        // and release events
        if let Some(Button::Keyboard(key)) = e.release_args() {
            match key {
                Key::LCtrl | Key::RCtrl => game.control_pressed = false,
                Key::LShift | Key::RShift => game.shift_pressed = false,
                _ => {}
            }
        }

        // Handle the response from the backend.
        for response in game.collection.execute(command) {
            match response {
                Response::LevelFinished => {
                    if !level_solved {
                        let lvl = game.current_level();
                        info!("Level solved using {} moves, {} of which moved a crate.",
                              lvl.number_of_moves(),
                              lvl.number_of_pushes());
                        info!("Solution: {}", lvl.moves_to_string());
                        level_solved = true;
                    }
                }
                Response::NewLevel(rank) => {
                    info!("Switched to level #{}", rank);
                    game.update_size(&window_size);
                    let tmp = generate_level_scene(&mut window.factory, &game);
                    scene = tmp.0;
                    crate_ids = tmp.1;
                    worker_id = tmp.2;
                    level_solved = false;
                }
                Response::MoveWorkerTo(pos, dir) => {
                    set_position(&mut scene, worker_id, pos, game.tile_size as f64);
                    set_rotation(&mut scene, worker_id, direction_to_angle(dir));
                }
                Response::MoveCrateTo(i, pos) => {
                    set_position(&mut scene, crate_ids[i], pos, game.tile_size as f64);
                }
            }
        }

        // If the window size has been changed, update the tile size and recenter the level.
        if let Some(size) = e.resize_args() {
            window_size = size;
            game.update_size(&window_size);
            let tmp = generate_level_scene(&mut window.factory, &game);
            scene = tmp.0;
            crate_ids = tmp.1;
            worker_id = tmp.2;
        }
    }
}
