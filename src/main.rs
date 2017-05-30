// GUI
#[macro_use]
extern crate glium;
extern crate image;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

extern crate sokoban_backend as backend;

use std::cmp::min;
use std::collections::VecDeque;

use glium::texture::Texture2d;
use glium::backend::Facade;
use glium::glutin::{VirtualKeyCode, MouseButton};
use glium::backend::glutin_backend::GlutinFacade;

pub mod texture;

use backend::*;

const IMAGE_SIZE: f64 = 360.0;
const NO_INDICES: glium::index::NoIndices =
    glium::index::NoIndices(glium::index::PrimitiveType::TriangleStrip);

pub struct Gui {
    game: Game,

    window_size: [u32; 2],

    textures: Textures,
    worker_position: backend::Position,
    worker_direction: Direction,

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

impl Gui {
    pub fn new(collection_name: &str, textures: Textures) -> Self {
        let game = Game::new(collection_name);
        if game.is_err() {
            panic!("Failed to load level set: {:?}", game.unwrap_err());
        }
        let game = game.unwrap();
        let worker_position = game.worker_position().clone();
        let worker_direction = game.worker_direction();

        Gui {
            game,

            window_size: [640, 480],
            textures,
            worker_position,
            worker_direction,

            shift_pressed: false,
            control_pressed: false,
            cursor_pos: [0.0, 0.0],

            tile_size: 50,
            offset_left: 0,
            offset_top: 0,
        }
    }

    fn current_level(&self) -> &Level {
        &self.game.collection.current_level
    }

    /// Handle press event.
    fn press_to_command(&mut self, key: VirtualKeyCode) -> Command {
        use Command::*;
        use VirtualKeyCode::*;
        match key {
            // Move
            Left | Right | Up | Down => {
                let dir = key_to_direction(key);
                return if self.control_pressed == self.shift_pressed {
                           Move(dir)
                       } else {
                           MoveAsFarAsPossible(dir, MayPushCrate(self.shift_pressed))
                       };
            }

            // Undo and redo
            Z if !self.control_pressed => {}
            U if self.control_pressed => {}
            U | Z if self.shift_pressed => return Redo,
            U | Z => return Undo,

            // Modifier keys
            LControl | RControl => {
                self.control_pressed = true;
            }
            LShift | RShift => {
                self.shift_pressed = true;
            }

            P => return PreviousLevel,
            N => return NextLevel,

            // Open the main menu
            Escape => return ResetLevel,
            _ => {
                error!("Unkown key: {:?}", key);
            }
        }
        Nothing
    }

    fn click_to_command(&mut self, mouse_button: MouseButton) -> Command {
        let x = (self.cursor_pos[0] / self.tile_size as f64).trunc() as isize;
        let y = (self.cursor_pos[1] / self.tile_size as f64 - 0.5).trunc() as isize;
        if x >= 0 && y >= 0 {
            Command::MoveToPosition(backend::Position { x, y },
                                    MayPushCrate(mouse_button == MouseButton::Right))
        } else {
            Command::Nothing
        }
    }


    /// Create a `Scene` containing the level’s entities.
    fn generate_background(&mut self, display: &Facade) -> Texture2d {
        use glium::Surface;

        self.tile_size = min(self.window_size[0] / self.game.columns() as u32,
                             self.window_size[1] / self.game.rows() as u32) as
                         i32;

        let lvl = self.current_level();
        let result =
            glium::texture::Texture2d::empty(display, self.window_size[0], self.window_size[1])
                .unwrap();
        result.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);
        let columns = lvl.columns() as u32;
        let rows = lvl.rows() as u32;


        for (i, cell) in lvl.background.iter().enumerate() {
            let pos = lvl.position(i);
            let texture = match *cell {
                Background::Empty => continue,
                Background::Floor => &self.textures.floor,
                Background::Goal => &self.textures.goal,
                Background::Wall => &self.textures.wall,
            };
            let vertices = texture::create_quad_vertices(pos, columns, rows);
            let vertex_buffer = glium::VertexBuffer::new(display, &vertices).unwrap();
            let program = glium::Program::from_source(display,
                                                      texture::VERTEX_SHADER,
                                                      texture::FRAGMENT_SHADER,
                                                      None)
                    .unwrap();

            let uniforms = uniform!{
                tex: texture,
            };

            result
                .as_surface()
                .draw(&vertex_buffer,
                      &NO_INDICES,
                      &program,
                      &uniforms,
                      &Default::default())
                .unwrap();
        }

        result
    }

    fn render_level(&self, display: &GlutinFacade, bg: &Texture2d, level_solved: bool) {
        use glium::Surface;

        // Draw background
        let vertices = texture::create_full_screen_quad();
        let vertex_buffer = glium::VertexBuffer::new(display, &vertices).unwrap();
        let program = glium::Program::from_source(display,
                                                  texture::VERTEX_SHADER,
                                                  texture::FRAGMENT_SHADER,
                                                  None)
                .unwrap();

        let mut target = display.draw();

        let uniforms = uniform!{
            tex: bg,
        };

        target
            .draw(&vertex_buffer,
                  &NO_INDICES,
                  &program,
                  &uniforms,
                  &Default::default())
            .unwrap();

        // Draw foreground
        let lvl = self.current_level();
        let columns = lvl.columns() as u32;
        let rows = lvl.rows() as u32;

        let params = glium::DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        let uniforms = uniform!{
            tex: &self.textures.crate_,
        };

        // Draw the crates
        for (&pos, _) in lvl.crates.iter() {
            let vertices = texture::create_quad_vertices(pos, columns, rows);
            let vertex_buffer = glium::VertexBuffer::new(display, &vertices).unwrap();

            target
                .draw(&vertex_buffer, &NO_INDICES, &program, &uniforms, &params)
                .unwrap();
        }

        // Draw the worker
        let vertices = texture::create_quad_vertices(self.worker_position, columns, rows);
        let vertex_buffer = glium::VertexBuffer::new(display, &vertices).unwrap();

        let uniforms = uniform!{
            tex: &self.textures.worker,
        };

        // TODO rotate worker
        target
            .draw(&vertex_buffer, &NO_INDICES, &program, &uniforms, &params)
            .unwrap();

        target.finish().unwrap();
    }

    /// Draw an overlay with some statistics.
    fn draw_end_of_level_screen(&self, display: &Facade, end_of_collection: bool) {
        /*
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
        */
    }
}

/// Multiply a position by a factor as a way of mapping tile coordinates to pixel coordinates.
fn scale_position(pos: backend::Position, factor: f64) -> (f64, f64) {
    ((pos.x as f64 + 0.5) * factor, (pos.y as f64 + 0.5) * factor)
}

/// Map arrow keys to the corresponding directions, panic on other keys.
fn key_to_direction(key: VirtualKeyCode) -> Direction {
    use self::Direction::*;
    match key {
        VirtualKeyCode::Left => Left,
        VirtualKeyCode::Right => Right,
        VirtualKeyCode::Up => Up,
        VirtualKeyCode::Down => Down,
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

pub struct Textures {
    wall: Texture2d,
    wall_left: Texture2d,
    wall_right: Texture2d,
    wall_both: Texture2d,
    floor: Texture2d,
    goal: Texture2d,
    worker: Texture2d,
    crate_: Texture2d,
}

impl Textures {
    /// Load all textures.
    fn new(factory: &Facade) -> Self {
        let wall = texture::load(factory, "wall");
        let wall_left = texture::load(factory, "wall_left");
        let wall_right = texture::load(factory, "wall_right");
        let wall_both = texture::load(factory, "wall_both");
        let floor = texture::load(factory, "floor");
        let goal = texture::load(factory, "goal");
        let worker = texture::load(factory, "worker");
        let crate_ = texture::load(factory, "crate");

        Textures {
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
    use glium::DisplayBuild;
    let title = "Sokoban";
    let display = glium::glutin::WindowBuilder::new()
        .with_dimensions(640, 480)
        .with_title(title)
        .build_glium()
        .unwrap_or_else(|e| panic!("Failed to build window: {}", e));

    // Initialize colog after window to suppress some log output.
    colog::init();


    let collection = std::env::var("SOKOBAN_COLLECTION").unwrap_or_else(|_| "original".to_string());
    let mut gui = Gui::new(&collection, Textures::new(&display));
    info!("Loading level #{}", gui.game.collection.current_level.rank);

    let mut level_solved = false;
    let mut end_of_collection = false;
    let mut bg = gui.generate_background(&display);

    let mut commands = VecDeque::new();
    let mut queue = VecDeque::new();

    //let font = &ASSETS.clone().join("FiraSans-Regular.ttf");
    //let mut glyphs = Glyphs::new(font, &display).unwrap();

    loop {
        gui.render_level(&display, &bg, level_solved);

        for ev in display.poll_events() {
            use glium::glutin::Event;
            use glium::glutin::ElementState::*;
            // Draw the current level

            match ev {
                Event::Closed => return,
                Event::Resized(w, h) => {
                    info!("Resizing window...");
                    gui.window_size = [w, h];
                    bg = gui.generate_background(&display);
                }
                /*
            // Handle key press
            let command = match e.press_args() {
                Some(Button::Keyboard(_key)) if level_solved => Command::NextLevel,
                Some(Button::Keyboard(Key::R)) if gui.control_pressed => {
                    // Reload images
                    info!("Reloading textures...");
                    gui.textures = Textures::new(&mut window.factory);
                    Command::Nothing
                }
                Some(args) if !level_solved => gui.press_to_command(args),
                _ => Command::Nothing,
            };
            */
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::Q)) => return,
                Event::KeyboardInput(..) |
                Event::MouseInput(..) if level_solved => {
                    commands.push_back(Command::NextLevel);
                }
                Event::KeyboardInput(state, _, Some(key)) => {
                    use glium::glutin::VirtualKeyCode::*;
                    match key {
                        LControl | RControl => gui.control_pressed = state == Pressed,
                        LShift | RShift => gui.shift_pressed = state == Pressed,
                        _ if state == Pressed => commands.push_back(gui.press_to_command(key)),
                        _ => (),
                    }
                }

                Event::MouseMoved(x, y) => gui.cursor_pos = [x as f64, y as f64],
                Event::MouseInput(_, btn) => commands.push_back(gui.click_to_command(btn)),

                /*
                Event::KeyboardInput(_, _, None) |
                Event::MouseEntered |
                Event::MouseLeft |
                Event::MouseWheel(..) |
                Event::TouchpadPressure(..) |
                Event::Awakened |
                Event::Refresh |
                Event::Suspended(_) |
                Event::Touch(_) |
                Event::Moved(..) |
                Event::ReceivedCharacter(_) |
                Event::Focused(_) |
                Event::DroppedFile(_) => (),
                */
                _ => (),
            }
        }

        for cmd in commands.drain(..) {
            queue.extend(gui.game.execute(cmd));
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
                    gui.worker_position = gui.game.worker_position();
                    gui.worker_direction = gui.game.worker_direction();
                    bg = gui.generate_background(&display);
                }
                Response::MoveWorkerTo(pos, dir) => {
                    gui.worker_position = pos;
                    gui.worker_direction = dir;
                }
                Response::MoveCrateTo(_i, _pos) => {
                    //let id = gui.crate_ids[i];
                    //gui.move_sprite_to(id, pos);
                }
            }
        }
    }
}
