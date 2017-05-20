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
use gfx_core::{Factory, Resources};
use piston_window::*;
use sprite::{Scene, Sprite};
use uuid::Uuid;

pub mod texture;

use sokoban::*;

const EMPTY: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // black
const IMAGE_SIZE: f64 = 360.0;

pub struct Game<R: Resources> {
    /// Path to the assets directory
    assets: PathBuf,

    /// The current level set
    collection: Collection,

    window_size: [u32; 2],
    scene: Scene<Texture<R>>,
    crate_ids: Vec<Uuid>,
    worker_id: Uuid,

    /// Is the shift key currently pressed?
    shift_pressed: bool,

    /// Is the control key currently pressed?
    control_pressed: bool,

    /// Current cursor position
    cursor_pos: [f64; 2],

    /// Size of each cell
    tile_size: i32,

    /// Horizontal margin
    offset_left: i32,

    /// Vertical margin
    offset_top: i32,
}

impl<R: Resources> Game<R> {
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

            window_size: [640, 480],
            scene: Scene::new(),
            crate_ids: vec![],
            worker_id: Uuid::new_v4(),

            shift_pressed: false,
            control_pressed: false,
            cursor_pos: [0.0, 0.0],

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

    /// Update the tile size and offsets such that the level fills most of the window.
    pub fn update_size<F>(&mut self, factory: &mut F)
        where F: Factory<R>
    {
        let width = self.window_size[0] as i32;
        let height = self.window_size[1] as i32;
        let columns = self.current_level().columns() as i32;
        let rows = self.current_level().rows() as i32;

        self.tile_size = min(width / columns, height / rows);
        self.offset_left = (width - columns * self.tile_size) / 2;
        self.offset_top = (height - rows * self.tile_size) / 2;

        self.generate_level_scene(factory);
    }

    /// Handle press event.
    fn press_to_command(&mut self, args: Button) -> Command {
        use Command::*;
        match args {
            Button::Keyboard(key) => {
                match key {
                    // Move
                    Key::Left | Key::Right | Key::Up | Key::Down => {
                        let dir = key_to_direction(key);
                        return if self.control_pressed == self.shift_pressed {
                                   Move(dir)
                               } else {
                                   MoveAsFarAsPossible(dir, MayPushCrate(self.shift_pressed))
                               };
                    }

                    // Undo and redo
                    Key::Z if !self.control_pressed => {}
                    Key::U if self.control_pressed => {}
                    Key::U | Key::Z if self.shift_pressed => return Redo,
                    Key::U | Key::Z => return Undo,

                    // Modifier keys
                    Key::LCtrl | Key::RCtrl => {
                        self.control_pressed = true;
                    }
                    Key::LShift | Key::RShift => {
                        self.shift_pressed = true;
                    }

                    // Open the main menu
                    Key::Escape => {}
                    _ => {
                        error!("Unkown key: {:?}", key);
                    }
                }
            }

            Button::Mouse(mouse_button) => {
                let x = ((self.cursor_pos[0] as i32 - self.offset_left) / self.tile_size) as isize;
                let y = ((self.cursor_pos[1] as i32 - self.offset_top) / self.tile_size) as isize;
                if x >= 0 && y >= 0 {
                    return MoveToPosition(sokoban::Position { x, y },
                                          MayPushCrate(mouse_button == MouseButton::Right));
                }
            }

            x => {
                error!("Unkown event: {:?}", x);
            }
        }

        Nothing
    }

    /// Create a `Scene` containing the level’s background.
    fn generate_level_scene<F>(&mut self, factory: &mut F)
        where F: Factory<R>
    {
        // Load the textures
        let empty_tex = Rc::new(texture::load(factory, "empty", &self.assets));
        let wall_tex = Rc::new(texture::load(factory, "wall", &self.assets));
        let floor_tex = Rc::new(texture::load(factory, "floor", &self.assets));
        let goal_tex = Rc::new(texture::load(factory, "goal", &self.assets));
        let worker_tex = Rc::new(texture::load(factory, "worker", &self.assets));
        let crate_tex = Rc::new(texture::load(factory, "crate", &self.assets));

        let mut scene = Scene::new();

        let columns = self.current_level().columns();

        // Create sprites for the level’s background.
        for (i, cell) in self.current_level().background.iter().enumerate() {
            let tex = match *cell {
                Background::Empty => empty_tex.clone(),
                Background::Floor => floor_tex.clone(),
                Background::Goal => goal_tex.clone(),
                Background::Wall => wall_tex.clone(),
            };
            let mut sprite = Sprite::from_texture(tex);
            let x = IMAGE_SIZE * ((i % columns) as f64 + 0.5);
            let y = IMAGE_SIZE * ((i / columns) as f64 + 0.5);
            sprite.set_position(x, y);
            scene.add_child(sprite);
        }

        // Create sprites for all crates in their initial position.
        let mut crate_ids = vec![];
        {
            let mut tmp: Vec<_> = self.current_level().crates.iter().collect();
            tmp.sort_by_key(|x| x.1);
            for (&sokoban::Position { x, y }, _) in tmp {
                let mut sprite = Sprite::from_texture(crate_tex.clone());
                let x = IMAGE_SIZE * (x as f64 + 0.5);
                let y = IMAGE_SIZE * (y as f64 + 0.5);
                sprite.set_position(x, y);
                crate_ids.push(scene.add_child(sprite));
            }
        }

        // Create the worker sprite.
        let mut sprite = Sprite::from_texture(worker_tex.clone());
        let sokoban::Position { x, y } = self.current_level().worker_position;
        let x = IMAGE_SIZE * (x as f64 + 0.5);
        let y = IMAGE_SIZE * (y as f64 + 0.5);
        sprite.set_position(x, y);
        sprite.set_rotation(direction_to_angle(self.current_level().worker_direction()));
        let worker_id = scene.add_child(sprite);

        self.scene = scene;
        self.crate_ids = crate_ids;
        self.worker_id = worker_id;
    }

    /// Draw an overlay with some statistics.
    fn draw_end_of_level_screen<C, G>(&self,
                                      c: &Context,
                                      g: &mut G,
                                      glyphs: &mut C,
                                      end_of_collection: bool)
        where C: CharacterCache,
              G: Graphics<Texture = <C as CharacterCache>::Texture>
    {
        let rectangle = Rectangle::new([0.0, 0.0, 0.0, 0.7]);
        let dims = rectangle::centered([0.0,
                                        0.0,
                                        self.window_size[0] as f64,
                                        self.window_size[1] as f64]);
        rectangle.draw(dims, &c.draw_state, c.transform, g);

        let lvl = self.current_level();
        let rank = lvl.rank;
        let moves = lvl.number_of_moves();
        let pushes = lvl.number_of_pushes();

        let heading = text::Text::new_color(color::WHITE, 32 as types::FontSize);
        heading.draw("Congratulations!",
                     glyphs,
                     &c.draw_state,
                     c.transform
                         .trans(-150.0, -32.0)
                         .trans(self.window_size[0] as f64 / 2.0,
                                self.window_size[1] as f64 / 2.0),
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
                     .trans(self.window_size[0] as f64 / 2.0,
                            self.window_size[1] as f64 / 2.0),
                 g);

        txt.draw(if end_of_collection {
                     "This is the end of the collection."
                 } else {
                     "Press any key to continue"
                 },
                 glyphs,
                 &c.draw_state,
                 c.transform
                     .trans(-120.0, 120.0)
                     .trans(self.window_size[0] as f64 / 2.0,
                            self.window_size[1] as f64 / 2.0),
                 g);
    }

    /// Move the sprite with the given `id` to position `pos`.
    fn move_sprite_to(&mut self, id: Uuid, pos: sokoban::Position) {
        let image_size = 360.0;
        let sokoban::Position { x, y } = pos;
        let (x, y) = (image_size * (x as f64 + 0.5), image_size * (y as f64 + 0.5));

        self.scene
            .child_mut(id)
            .map(|sprite| sprite.set_position(x, y));
    }

    /// Rotate the sprite by the given angle in degrees.
    fn rotate_sprite_to(&mut self, id: Uuid, dir: Direction) {
        self.scene
            .child_mut(id)
            .map(|sprite| sprite.set_rotation(direction_to_angle(dir)));
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



fn main() {
    let title = "Sokoban";
    let mut window: PistonWindow =
        WindowSettings::new(title, [640, 480])
            .exit_on_esc(true)
            .build()
            .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));
    window.set_lazy(true);

    // Initialize colog after window to suppress some log output.
    colog::init();

    let mut game = Game::new("microban");

    let mut level_solved = false;
    let mut end_of_collection = false;
    game.update_size(&mut window.factory);

    let font = &game.assets.clone().join("FiraSans-Regular.ttf");
    let mut glyphs = Glyphs::new(font, window.factory.clone()).unwrap();

    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g| {
            // Black background
            clear(EMPTY, g);

            // Draw the level
            let left = game.offset_left as f64;
            let top = game.offset_top as f64;
            let scale = game.tile_size as f64 / IMAGE_SIZE;
            game.scene
                .draw(c.transform.trans(left, top).scale(scale, scale), g);

            // Overlay message about solving the level.
            if level_solved {
                game.draw_end_of_level_screen(&c, g, &mut glyphs, end_of_collection);
            }
        });

        // Keep track of where the cursor is pointing
        if let Some(new_pos) = e.mouse_cursor_args() {
            game.cursor_pos = new_pos;
        }

        // Handle key press
        let command = match e.press_args() {
            Some(Button::Keyboard(_key)) if level_solved => Command::NextLevel,
            Some(args) if !level_solved => game.press_to_command(args),
            _ => Command::Nothing,
        };

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
                        end_of_collection = lvl.rank == game.collection.number_of_levels();
                    }
                }
                Response::NewLevel(_rank) => {
                    level_solved = false;
                    game.update_size(&mut window.factory);
                }
                Response::MoveWorkerTo(pos, dir) => {
                    let id = game.worker_id;
                    game.move_sprite_to(id, pos);
                    game.rotate_sprite_to(id, dir);
                }
                Response::MoveCrateTo(i, pos) => {
                    let id = game.crate_ids[i];
                    game.move_sprite_to(id, pos);
                }
            }
        }

        // If the window size has been changed, update the tile size and recenter the level.
        if let Some(size) = e.resize_args() {
            game.window_size = size;
            game.update_size(&mut window.factory);
        }
    }
}
