// GUI
extern crate piston_window;
extern crate graphics;
extern crate gfx_core;
extern crate gfx_device_gl;
extern crate gfx_graphics;
extern crate sprite;
extern crate uuid;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

extern crate sokoban_backend as backend;

use std::cmp::min;
use std::collections::VecDeque;
use std::rc::Rc;

use graphics::rectangle;
use graphics::character::CharacterCache;
use gfx_core::{Factory, Resources};
use piston_window::*;
use sprite::{Scene, Sprite};
use uuid::Uuid;

pub mod texture;

use backend::*;

const EMPTY: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // black
const IMAGE_SIZE: f64 = 360.0;

pub struct Gui<R: Resources> {
    game: Game,

    window_size: [u32; 2],
    scene: Scene<Texture<R>>,
    crate_ids: Vec<Uuid>,
    worker_id: Uuid,
    textures: Textures<R>,

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

impl<R: Resources> Gui<R> {
    pub fn new(collection_name: &str, textures: Textures<R>) -> Self {
        let game = Game::new(collection_name);
        if game.is_err() {
            panic!("Failed to load level set: {:?}", game.unwrap_err());
        }
        let game = game.unwrap();

        Gui {
            game,

            window_size: [640, 480],
            scene: Scene::new(),
            crate_ids: vec![],
            worker_id: Uuid::new_v4(),
            textures,

            shift_pressed: false,
            control_pressed: false,
            cursor_pos: [0.0, 0.0],

            tile_size: 50,
            offset_left: 0,
            offset_top: 0,
        }
    }

    pub fn current_level(&self) -> &Level {
        &self.game.collection.current_level
    }

    pub fn current_level_mut(&mut self) -> &mut Level {
        &mut self.game.collection.current_level
    }

    /// Update the tile size and offsets such that the level fills most of the window.
    pub fn update_size(&mut self) {
        let width = self.window_size[0] as i32;
        let height = self.window_size[1] as i32;
        let columns = self.current_level().columns() as i32;
        let rows = self.current_level().rows() as i32;

        self.tile_size = min(width / columns, height / rows);
        self.offset_left = (width - columns * self.tile_size) / 2;
        self.offset_top = (height - rows * self.tile_size) / 2;

        self.generate_level_scene();
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

                    Key::P | Key::Less => return PreviousLevel,
                    Key::N | Key::Greater => return NextLevel,

                    // Open the main menu
                    Key::Escape => return ResetLevel,
                    _ => {
                        error!("Unkown key: {:?}", key);
                    }
                }
            }

            Button::Mouse(mouse_button) => {
                let x = ((self.cursor_pos[0] as i32 - self.offset_left) / self.tile_size) as isize;
                let y = ((self.cursor_pos[1] as i32 - self.offset_top) / self.tile_size) as isize;
                if x >= 0 && y >= 0 {
                    return MoveToPosition(backend::Position { x, y },
                                          MayPushCrate(mouse_button == MouseButton::Right));
                }
            }

            x => {
                error!("Unkown event: {:?}", x);
            }
        }

        Nothing
    }

    /// Move the sprite with the given `id` to position `pos`.
    fn move_sprite_to(&mut self, id: Uuid, pos: backend::Position) {
        let (x, y) = scale_position(pos, IMAGE_SIZE);

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


    /// Create a `Scene` containing the level’s entities.
    fn generate_level_scene(&mut self) {
        let mut scene = Scene::new();
        let mut crate_ids = vec![];
        let worker_id;

        {
            let lvl = self.current_level();

            // Create sprites for the level’s background.
            for (i, cell) in lvl.background.iter().enumerate() {
                let pos = lvl.position(i);
                let tex = match *cell {
                    Background::Empty => continue,
                    Background::Floor => self.textures.floor.clone(),
                    Background::Goal => self.textures.goal.clone(),
                    Background::Wall => {
                        let is_left_end = pos.x == 0 || lvl.background[i - 1] != Background::Wall;
                        let is_right_end = pos.x == lvl.columns() as isize - 1 ||
                                           lvl.background[i + 1] != Background::Wall;
                        if is_left_end && is_right_end {
                            self.textures.wall_both.clone()
                        } else if is_left_end {
                            self.textures.wall_left.clone()
                        } else if is_right_end {
                            self.textures.wall_right.clone()
                        } else {
                            self.textures.wall.clone()
                        }
                    }
                };
                let mut sprite = Sprite::from_texture(tex);
                let (x, y) = scale_position(pos, IMAGE_SIZE);
                sprite.set_position(x, y);
                scene.add_child(sprite);
            }

            // Create sprites for all crates in their initial position.
            let mut tmp: Vec<_> = lvl.crates.iter().collect();
            tmp.sort_by_key(|x| x.1);
            for (&pos, _) in tmp {
                let mut sprite = Sprite::from_texture(self.textures.crate_.clone());
                let (x, y) = scale_position(pos, IMAGE_SIZE);
                sprite.set_position(x, y);
                crate_ids.push(scene.add_child(sprite));
            }

            // Create the worker sprite.
            let mut sprite = Sprite::from_texture(self.textures.worker.clone());
            let (x, y) = scale_position(lvl.worker_position, IMAGE_SIZE);
            sprite.set_position(x, y);
            sprite.set_rotation(direction_to_angle(lvl.worker_direction()));
            worker_id = scene.add_child(sprite);
        }

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
}

/// Multiply a position by a factor as a way of mapping tile coordinates to pixel coordinates.
fn scale_position(pos: backend::Position, factor: f64) -> (f64, f64) {
    ((pos.x as f64 + 0.5) * factor, (pos.y as f64 + 0.5) * factor)
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

pub struct Textures<R: Resources> {
    empty: Rc<Texture<R>>,
    wall: Rc<Texture<R>>,
    wall_left: Rc<Texture<R>>,
    wall_right: Rc<Texture<R>>,
    wall_both: Rc<Texture<R>>,
    floor: Rc<Texture<R>>,
    goal: Rc<Texture<R>>,
    worker: Rc<Texture<R>>,
    crate_: Rc<Texture<R>>,
}

impl<R: Resources> Textures<R> {
    /// Load all textures.
    fn new<F>(factory: &mut F) -> Self
        where F: Factory<R>
    {
        let empty = Rc::new(texture::load(factory, "empty"));
        let wall = Rc::new(texture::load(factory, "wall"));
        let wall_left = Rc::new(texture::load(factory, "wall_left"));
        let wall_right = Rc::new(texture::load(factory, "wall_right"));
        let wall_both = Rc::new(texture::load(factory, "wall_both"));
        let floor = Rc::new(texture::load(factory, "floor"));
        let goal = Rc::new(texture::load(factory, "goal"));
        let worker = Rc::new(texture::load(factory, "worker"));
        let crate_ = Rc::new(texture::load(factory, "crate"));

        Textures {
            empty,
            wall,
            wall_left,
            wall_right,
            wall_both,
            floor,
            goal,
            worker,
            crate_,
        }
    }
}

fn main() {
    let title = "Sokoban";
    let mut window: PistonWindow =
        WindowSettings::new(title, [640, 480])
            .vsync(true)
            .build()
            .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));
    window.set_lazy(true);

    // Initialize colog after window to suppress some log output.
    colog::init();

    let collection = std::env::var("SOKOBAN_COLLECTION").unwrap_or_else(|_| "original".to_string());
    let mut gui = Gui::new(&collection, Textures::new(&mut window.factory));
    info!("Loading level #{}", gui.game.collection.current_level.rank);

    let mut queue = VecDeque::new();
    let mut level_solved = false;
    let mut end_of_collection = false;
    gui.update_size();

    let font = &ASSETS.clone().join("FiraSans-Regular.ttf");
    let mut glyphs = Glyphs::new(font, window.factory.clone()).unwrap();

    loop {
        // Handle input and rendering
        if let Some(e) = window.next() {
            let mut is_draw = false;
            window.draw_2d(&e, |c, g| {
                // Black background
                clear(EMPTY, g);

                // Draw the level
                let left = gui.offset_left as f64;
                let top = gui.offset_top as f64;
                let scale = gui.tile_size as f64 / IMAGE_SIZE;
                gui.scene
                    .draw(c.transform.trans(left, top).scale(scale, scale), g);


                // Overlay message about solving the level.
                if level_solved {
                    gui.draw_end_of_level_screen(&c, g, &mut glyphs, end_of_collection);
                }
                is_draw = true;
            });

            // Keep track of where the cursor is pointing
            if let Some(new_pos) = e.mouse_cursor_args() {
                gui.cursor_pos = new_pos;
            }

            // Handle key press
            let command = match e.press_args() {
                Some(Button::Keyboard(_key)) if level_solved => Command::NextLevel,
                Some(Button::Keyboard(Key::R)) if gui.control_pressed => {
                    // Reload images
                    info!("Reloading textures...");
                    gui.textures = Textures::new(&mut window.factory);
                    Command::Nothing
                }
                Some(Button::Keyboard(Key::Q)) => std::process::exit(0),
                Some(args) if !level_solved => gui.press_to_command(args),
                _ => Command::Nothing,
            };
            queue.extend(gui.game.execute(command));

            // and release events
            if let Some(Button::Keyboard(key)) = e.release_args() {
                match key {
                    Key::LCtrl | Key::RCtrl => gui.control_pressed = false,
                    Key::LShift | Key::RShift => gui.shift_pressed = false,
                    _ => {}
                }
            }

            // If the window size has been changed, update the tile size and recenter the level.
            if let Some(size) = e.resize_args() {
                gui.window_size = size;
                gui.update_size();
            }
        }


        // Handle responses from the backend.
        while let Some(response) = queue.pop_front() {
            match response {
                Response::LevelFinished => {
                    if !level_solved {
                        level_solved = true;
                        end_of_collection = gui.current_level().rank ==
                                            gui.game.collection.number_of_levels();
                    }
                }
                Response::NewLevel(rank) => {
                    info!("Loading level #{}", rank);
                    level_solved = false;
                    gui.update_size();
                }
                Response::MoveWorkerTo(pos, dir) => {
                    let id = gui.worker_id;
                    gui.move_sprite_to(id, pos);
                    gui.rotate_sprite_to(id, dir);
                }
                Response::MoveCrateTo(i, pos) => {
                    let id = gui.crate_ids[i];
                    gui.move_sprite_to(id, pos);
                }
            }
        }
    }
}
