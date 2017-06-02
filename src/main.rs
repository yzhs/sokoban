// GUI
#[macro_use]
extern crate glium;
extern crate glium_text;
extern crate image;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

extern crate sokoban_backend as backend;

use std::cmp::min;
use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;

use glium::Surface;
use glium::backend::Facade;
use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::{VirtualKeyCode, MouseButton};
use glium::texture::Texture2d;
use glium_text::{FontTexture, TextDisplay, TextSystem};

mod texture;

use backend::*;
use texture::*;

const NO_INDICES: glium::index::NoIndices =
    glium::index::NoIndices(glium::index::PrimitiveType::TriangleStrip);

pub struct Gui {
    game: Game,

    level_solved: bool,
    end_of_collection: bool,

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
}

impl Gui {
    pub fn new(collection_name: &str, textures: Textures) -> Self {
        let game = Game::new(collection_name);
        if game.is_err() {
            panic!("Failed to load level set: {:?}", game.unwrap_err());
        }
        let game = game.unwrap();
        let worker_position = game.worker_position();
        let worker_direction = game.worker_direction();

        Gui {
            game,
            level_solved: false,
            end_of_collection: false,

            window_size: [640, 480],
            textures,
            worker_position,
            worker_direction,

            shift_pressed: false,
            control_pressed: false,
            cursor_pos: [0.0, 0.0],

            tile_size: 50,
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

    fn render_level(&self, display: &GlutinFacade, bg: &Texture2d, font_data: &FontData) {
        let params = glium::DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

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
            .draw(&vertex_buffer, &NO_INDICES, &program, &uniforms, &params)
            .unwrap();

        // Draw foreground
        let lvl = self.current_level();
        let columns = lvl.columns() as u32;
        let rows = lvl.rows() as u32;

        let uniforms = uniform!{
            tex: &self.textures.crate_,
        };

        // Draw the crates
        for &pos in lvl.crates.keys() {
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
            tex: &self.textures.worker[direction_to_index(self.worker_direction)],
        };

        target
            .draw(&vertex_buffer, &NO_INDICES, &program, &uniforms, &params)
            .unwrap();


        // Draw an overlay with some statistics.
        if self.level_solved {
            // Darken background
            const DARKEN_SHADER: &str = r#"
            #version 140

            in vec2 v_tex_coords;
            out vec4 color;

            void main() {
                color = vec4(0.0, 0.0, 0.0, 0.6);
            }
            "#;

            let vertices = texture::create_full_screen_quad();
            let vertex_buffer = glium::VertexBuffer::new(display, &vertices).unwrap();

            let program =
                glium::Program::from_source(display, texture::VERTEX_SHADER, DARKEN_SHADER, None)
                    .unwrap();

            let uniforms = uniform!{};
            target
                .draw(&vertex_buffer, &NO_INDICES, &program, &uniforms, &params)
                .unwrap();

            // Text
            let text = font_data.heading("Congratulations!");
            let w = self.window_size[0] as f32;
            let h = self.window_size[1] as f32;
            let text_width = text.get_width();

            let matrix = [[1.0 / text_width, 0.0, 0.0, 0.0],
                          [0.0, 1.0 * w / h / text_width, 0.0, 0.0],
                          [0.0, 0.0, 1.0, 0.0],
                          [-0.5, 0.3, 0.0, 1.0_f32]];

            glium_text::draw(&text,
                             &font_data.system,
                             &mut target,
                             matrix,
                             (1.0, 1.0, 1.0, 1.0));

            let stats_text = format!("You have finished the level {} using {} moves, \
                                      {} of which moved a crate.",
                                     self.game.rank(),
                                     self.game.number_of_moves(),
                                     self.game.number_of_pushes());
            let text = font_data.text(&stats_text);
            let text_width = text.get_width();

            let matrix = [[1.0 / text_width, 0.0, 0.0, 0.0],
                          [0.0, 1.0 * w / h / text_width, 0.0, 0.0],
                          [0.0, 0.0, 1.0, 0.0],
                          [-0.5, -0.2, 0.0, 1.0_f32]];


            glium_text::draw(&text,
                             &font_data.system,
                             &mut target,
                             matrix,
                             (1.0, 1.0, 1.0, 1.0));
        }

        target.finish().unwrap();
    }
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
fn direction_to_index(dir: Direction) -> usize {
    match dir {
        Direction::Left => 0,
        Direction::Right => 1,
        Direction::Up => 2,
        Direction::Down => 3,
    }
}

struct FontData {
    system: TextSystem,
    font16: FontTexture,
    font32: FontTexture,
}

impl FontData {
    pub fn new<P: AsRef<Path>>(display: &GlutinFacade, font_path: P) -> Self {
        let system = TextSystem::new(display);
        let font16 = FontTexture::new(display, File::open(&font_path).unwrap(), 16).unwrap();
        let font32 = FontTexture::new(display, File::open(&font_path).unwrap(), 32).unwrap();

        FontData {
            system,
            font16,
            font32,
        }
    }

    pub fn text(&self, content: &str) -> TextDisplay<&FontTexture> {
        TextDisplay::new(&self.system, &self.font16, content)
    }

    pub fn heading(&self, content: &str) -> TextDisplay<&FontTexture> {
        TextDisplay::new(&self.system, &self.font32, content)
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

    let font_data = FontData::new(&display, ASSETS.join("FiraSans-Regular.ttf"));

    let collection = std::env::var("SOKOBAN_COLLECTION").unwrap_or_else(|_| "original".to_string());
    let mut gui = Gui::new(&collection, Textures::new(&display));
    info!("Loading level #{}", gui.game.collection.current_level.rank);

    let mut bg = gui.generate_background(&display);

    let mut queue = VecDeque::new();

    loop {
        gui.render_level(&display, &bg, &font_data);

        for ev in display.poll_events() {
            use glium::glutin::Event;
            use glium::glutin::ElementState::*;
            // Draw the current level
            let mut cmd = Command::Nothing;

            match ev {
                Event::Closed |
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::Q)) => return,

                Event::KeyboardInput(Pressed, _, _) |
                Event::MouseInput(..) if gui.level_solved => {
                    cmd = Command::NextLevel;
                }
                Event::KeyboardInput(state, _, Some(key)) => {
                    use glium::glutin::VirtualKeyCode::*;
                    match key {
                        LControl | RControl => gui.control_pressed = state == Pressed,
                        LShift | RShift => gui.shift_pressed = state == Pressed,
                        _ if state == Pressed => cmd = gui.press_to_command(key),
                        _ => (),
                    }
                }

                Event::MouseMoved(x, y) => gui.cursor_pos = [x as f64, y as f64],
                Event::MouseInput(_, btn) => cmd = gui.click_to_command(btn),

                Event::Resized(w, h) => {
                    gui.window_size = [w, h];
                    bg = gui.generate_background(&display);
                }

                /*
                Event::KeyboardInput(_, _, None) | Event::MouseEntered | Event::MouseLeft |
                Event::MouseWheel(..) | Event::TouchpadPressure(..) | Event::Awakened |
                Event::Refresh | Event::Suspended(_) | Event::Touch(_) | Event::Moved(..) |
                Event::ReceivedCharacter(_) | Event::Focused(_) | Event::DroppedFile(_) => (),
                */
                _ => (),
            }

            queue.extend(gui.game.execute(cmd));
        }


        // Handle responses from the backend.
        while let Some(response) = queue.pop_front() {
            match response {
                Response::LevelFinished => {
                    if !gui.level_solved {
                        gui.level_solved = true;
                        gui.end_of_collection = gui.current_level().rank ==
                                                gui.game.collection.number_of_levels();
                    }
                }
                Response::NewLevel(rank) => {
                    info!("Loading level #{}", rank);
                    gui.level_solved = false;
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
