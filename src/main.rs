// GUI
#[macro_use]
extern crate glium;
extern crate glium_text_rusttype as glium_text;
extern crate image;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

// Colored output
extern crate ansi_term;

// Argument handling
extern crate clap;

extern crate natord;

extern crate sokoban_backend as backend;

use std::cmp::min;
use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;

use clap::{App, Arg};
use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::{VirtualKeyCode, MouseButton};
use glium::texture::Texture2d;
use glium_text::{FontTexture, TextDisplay, TextSystem};

mod texture;
mod sprite;

use backend::*;
use texture::*;
use sprite::*;


const TITLE: &'static str = "Sokoban";

/// All we ever do is draw rectangles created from two triangles each, so we don’t need any other
/// `PrimitiveType`.
const NO_INDICES: glium::index::NoIndices =
    glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

const WHITE: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 1.0);


pub struct Gui {
    // Game state
    /// The main back end data structure.
    game: Game,

    /// Has the current level been solved, i.e. should the end-of-level overlay be rendered?
    level_solved: bool,

    /// Is the current level the last of this collection.
    end_of_collection: bool,

    // Inputs
    /// Is the shift key currently pressed?
    shift_pressed: bool,

    /// Is the control key currently pressed?
    control_pressed: bool,

    /// The current mouse position
    cursor_pos: [f64; 2],

    // Graphics
    display: GlutinFacade,
    font_data: FontData,

    /// The size of the window in pixels as `[width, height]`.
    window_size: [u32; 2],

    /// Tile textures, i.e. wall, worker, crate, etc.
    textures: Textures,

    /// Pre-rendered static part of the current level, i.e. walls, floors and goals.
    background: Option<Texture2d>,

    worker: Sprite,
    crates: Vec<Sprite>,
}


impl Gui {
    /// Initialize the `Gui` struct by setting default values, and loading a collection and
    /// textures.
    pub fn new(collection_name: &str) -> Self {
        use glium::DisplayBuild;
        let game = Game::new(collection_name).expect("Failed to load level set");

        let display = glium::glutin::WindowBuilder::new()
            .with_dimensions(800, 600)
            .with_title(TITLE.to_string() + " - " + game.name())
            .build_glium()
            .unwrap_or_else(|e| panic!("Failed to build window: {}", e));

        let textures = Textures::new(&display);
        let font_data = FontData::new(&display,
                                      ASSETS.join("FiraSans-Regular.ttf"),
                                      ASSETS.join("FiraMono-Regular.ttf"));

        let worker = Sprite::new(game.worker_position(), texture::TileKind::Worker);
        // FIXME code duplicated from Gui::update_sprites()

        info!("Loading level #{} of collection {}",
              game.rank(),
              game.name());

        let mut gui = Gui {
            game,
            level_solved: false,
            end_of_collection: false,

            display,
            font_data,
            window_size: [800, 600],
            textures,
            background: None,

            shift_pressed: false,
            control_pressed: false,
            cursor_pos: [0.0, 0.0],

            worker,
            crates: vec![],
        };
        gui.update_sprites();
        gui
    }

    fn current_level(&self) -> &Level {
        self.game.current_level()
    }

    /// Handle key press events.
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
            LControl | RControl => self.control_pressed = true,
            LShift | RShift => self.shift_pressed = true,

            P => return PreviousLevel,
            N => return NextLevel,

            S if self.control_pressed => return Save,

            // Open the main menu
            Escape => return ResetLevel,
            _ => error!("Unkown key: {:?}", key),
        }
        Nothing
    }

    /// Compute the tile size.
    fn tile_size(&self) -> f64 {
        let columns = self.game.columns() as u32;
        let rows = self.game.rows() as u32;
        min(self.window_size[0] / columns, self.window_size[1] / rows) as f64
    }

    /// Handle a mouse click.
    fn click_to_command(&mut self, mouse_button: MouseButton) -> Command {
        let columns = self.game.columns() as isize;
        let rows = self.game.rows() as isize;
        let tile_size = self.tile_size();

        let (offset_x, offset_y) = if self.aspect_ratio() < 1.0 {
            ((self.window_size[0] as f64 - columns as f64 * tile_size) / 2.0, 0.0)
        } else {
            (0.0, (self.window_size[1] as f64 - rows as f64 * tile_size) / 2.0)
        };

        let x = ((self.cursor_pos[0] - offset_x) / tile_size).trunc() as isize;
        let y = ((self.cursor_pos[1] - offset_y - 0.5) / tile_size).trunc() as isize;
        if x >= 0 && y >= 0 && x < columns && y < rows {
            Command::MoveToPosition(backend::Position { x, y },
                                    MayPushCrate(mouse_button == MouseButton::Right))
        } else {
            Command::Nothing
        }
    }


    /// Render the static tiles of the current level onto a texture.
    fn generate_background(&mut self) {
        let target;

        {
            let columns = self.game.columns() as u32;
            let rows = self.game.rows() as u32;
            let tile_size = self.tile_size();
            let width = tile_size as u32 * columns;
            let height = tile_size as u32 * rows;

            let lvl = self.current_level();

            // Find transitions between wall and non-wall tiles
            let mut horizontal_wall_floor = vec![];
            let mut horizontal_wall_empty = vec![];
            let mut vertical_wall_floor = vec![];
            let mut vertical_wall_empty = vec![];

            let mut previous_cell = Background::Empty;
            for (i, &cell) in lvl.background.iter().enumerate() {
                use Background::*;
                let pos = lvl.position(i);
                if pos.x == 0 {
                    previous_cell = Background::Empty;
                }

                match (previous_cell, cell) {
                    (Empty, Wall) | (Wall, Empty) => vertical_wall_empty.push(pos),
                    (Wall, Wall) => (),
                    (_, Wall) | (Wall, _) => vertical_wall_floor.push(pos),
                    _ => (),
                }

                previous_cell = cell;

                if cell != Wall {
                    continue;
                }
                if pos.x + 1 == columns as isize {
                    vertical_wall_empty.push(pos.right());
                }

                let above = pos.above();
                let below = pos.below();

                if lvl.is_interior(above) {
                    horizontal_wall_floor.push(pos);
                } else if lvl.is_outside(above) {
                    horizontal_wall_empty.push(pos);
                }

                if lvl.is_interior(below) {
                    horizontal_wall_floor.push(below);
                } else if lvl.is_outside(below) {
                    horizontal_wall_empty.push(below);
                }
            }

            // Create texture
            target = glium::texture::Texture2d::empty(&self.display, width * 2, height * 2)
                .unwrap();
            target.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);

            let program = glium::Program::from_source(&self.display,
                                                      texture::VERTEX_SHADER,
                                                      texture::FRAGMENT_SHADER,
                                                      None)
                    .unwrap();

            let params = glium::DrawParameters {
                backface_culling: glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise,
                blend: glium::Blend::alpha_blending(),
                ..Default::default()
            };

            // Render each of the (square) tiles
            for &value in &[Background::Floor, Background::Goal, Background::Wall] {
                let mut vertices = vec![];
                for (i, &cell) in lvl.background.iter().enumerate() {
                    if cell != value {
                        continue;
                    }
                    let pos = lvl.position(i);
                    vertices.extend(texture::create_quad_vertices(pos, columns, rows, 1.0));
                }
                let vertex_buffer = glium::VertexBuffer::new(&self.display, &vertices).unwrap();

                let texture = match value {
                    Background::Empty => continue,
                    Background::Floor => &self.textures.floor,
                    Background::Goal => &self.textures.goal,
                    Background::Wall => &self.textures.wall,
                };
                let uniforms = uniform!{tex: texture};

                target
                    .as_surface()
                    .draw(&vertex_buffer, &NO_INDICES, &program, &uniforms, &params)
                    .unwrap();
            }

            // Render the transitions
            let mut vertices = vec![];
            let tex = &self.textures;
            for &(ref positions, orientation, texture) in
                &[(horizontal_wall_empty, Direction::Up, &tex.transition_wall_empty_horizontal),
                  (horizontal_wall_floor, Direction::Up, &tex.transition_wall_floor_horizontal),
                  (vertical_wall_empty, Direction::Left, &tex.transition_wall_empty_vertical),
                  (vertical_wall_floor, Direction::Left, &tex.transition_wall_floor_vertical)] {

                for &pos in positions {
                    vertices.extend(texture::create_transition(pos, columns, rows, orientation));
                }
                let vertex_buffer = glium::VertexBuffer::new(&self.display, &vertices).unwrap();
                let uniforms = uniform!{tex: texture};
                target
                    .as_surface()
                    .draw(&vertex_buffer, &NO_INDICES, &program, &uniforms, &params)
                    .unwrap();

                vertices.clear();
            }
        }

        self.background = Some(target);
    }

    /// Create sprites for movable entities of the current level.
    fn update_sprites(&mut self) {
        self.worker = Sprite::new(self.game.worker_position(), texture::TileKind::Worker);
        self.worker.set_direction(self.game.worker_direction());
        self.crates = self.game
            .crate_positions()
            .iter()
            .map(|&pos| Sprite::new(pos, texture::TileKind::Crate))
            .collect();
        // TODO simplify hashmap -> iter -> vec -> iter -> vec -> iter -> vec

        self.background = None;
    }

    /// Compute the window’s aspect ratio.
    fn aspect_ratio(&self) -> f32 {
        let width = self.window_size[0];
        let height = self.window_size[1];
        height as f32 / width as f32
    }

    /// Given a vector of vertices describing a list of quads, draw them onto `target`.
    fn draw_quads(&self,
                  target: &mut glium::Frame,
                  vertices: Vec<Vertex>,
                  tex: &Texture2d,
                  params: &glium::DrawParameters,
                  program: &glium::Program)
                  -> Result<(), glium::DrawError> {
        let vertex_buffer = glium::VertexBuffer::new(&self.display, &vertices).unwrap();
        let uniforms = uniform!{tex: tex};
        target.draw(&vertex_buffer, &NO_INDICES, program, &uniforms, params)
    }

    /// Draw an overlay with some statistics.
    fn draw_end_of_level_overlay(&self,
                                 target: &mut glium::Frame,
                                 params: &glium::DrawParameters) {
        // Darken background
        const DARKEN_SHADER: &str = r#"
            #version 140

            in vec2 v_tex_coords;
            out vec4 color;

            void main() {
                color = vec4(0.0, 0.0, 0.0, 0.7);
            }
            "#;

        let font_data = &self.font_data;

        let program =
            glium::Program::from_source(&self.display, texture::VERTEX_SHADER, DARKEN_SHADER, None)
                .unwrap();

        self.draw_quads(target,
                   texture::create_full_screen_quad(),
                   // The texture is ignored by the given fragment shader, so we can take any here
                   &self.textures.worker, // FIXME find a cleaner solution
                   params,
                   &program)
                .unwrap();

        let aspect_ratio = self.aspect_ratio();

        // Print text
        font_data.draw(target,
                       "Congratulations!",
                       Font::Heading,
                       0.1,
                       [-0.5, 0.2],
                       aspect_ratio);

        let stats_text = format!("You have finished the level {} using {} moves, \
                                      {} of which moved a crate.",
                                 self.game.rank(),
                                 self.game.number_of_moves(),
                                 self.game.number_of_pushes());

        font_data.draw(target,
                       &stats_text,
                       Font::Text,
                       0.05,
                       [-0.5, -0.2],
                       aspect_ratio);
    }

    /// Render the current level.
    fn render_level(&mut self) {
        let params = glium::DrawParameters {
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise,
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        // Do we have to update the cache?
        if self.background.is_none() {
            self.generate_background();
        }
        let bg = self.background.as_ref().unwrap();

        let lvl = self.current_level();
        let columns = lvl.columns() as u32;
        let rows = lvl.rows() as u32;

        // Draw background
        let vertices = texture::create_background_quad(self.aspect_ratio(),
                                                       self.game.columns(),
                                                       self.game.rows());
        let vertex_buffer = glium::VertexBuffer::new(&self.display, &vertices).unwrap();
        let program = glium::Program::from_source(&self.display,
                                                  texture::VERTEX_SHADER,
                                                  texture::FRAGMENT_SHADER,
                                                  None)
                .unwrap();

        let uniforms = uniform!{tex: bg};

        let mut target = self.display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        target
            .draw(&vertex_buffer, &NO_INDICES, &program, &uniforms, &params)
            .unwrap();

        // Draw foreground
        let aspect_ratio = {
            let (width, height) = target.get_dimensions();
            width as f32 / height as f32 * rows as f32 / columns as f32
        };

        // Draw the crates
        let mut vertices = vec![];
        for sprite in &self.crates {
            vertices.extend(sprite.quad(columns, rows, aspect_ratio));
        }
        self.draw_quads(&mut target,
                        vertices,
                        &self.textures.crate_,
                        &params,
                        &program)
            .unwrap();

        // Draw the worker
        self.draw_quads(&mut target,
                        self.worker.quad(columns, rows, aspect_ratio),
                        &self.textures.worker,
                        &params,
                        &program)
            .unwrap();

        // Display text overlay
        if self.level_solved {
            self.draw_end_of_level_overlay(&mut target, &params);
        } else {
            let aspect_ratio = self.aspect_ratio();
            // TODO show collection name
            // Show some statistics
            let text = format!("Level: {}, Steps: {}, Pushes: {}",
                               self.game.rank(),
                               self.game.number_of_moves(),
                               self.game.number_of_pushes());

            self.font_data
                .draw(&mut target,
                      &text,
                      Font::Mono,
                      0.04,
                      [0.5, -0.9],
                      aspect_ratio);
        }

        target.finish().unwrap();
    }

    /// Handle the queue of responses from the back end, updating the gui status and logging
    /// messages.
    pub fn handle_responses(&mut self, queue: &mut VecDeque<Response>) {
        while let Some(response) = queue.pop_front() {
            use Response::*;
            match response {
                LevelFinished(resp) => {
                    if !self.level_solved {
                        use save::UpdateResponse::*;
                        self.level_solved = true;
                        match resp {
                            FirstTimeSolved => {
                                info!("You have successfully solved this level for the first time! \
                                   Congratulations!")
                            }
                            Update { moves, pushes } => {
                                if moves && pushes {
                                    info!("Your solution is the best so far, both in terms of moves and pushes!");
                                } else if moves {
                                    info!("Your solution is the best so far in terms of moves!");
                                } else if pushes {
                                    info!("Your solution is the best so far in terms of pushes!");
                                } else {
                                    info!("Solved the level without creating a new high score.")
                                }
                            }
                        }
                    }
                }
                ResetLevel | NewLevel(_) => {
                    if let Response::NewLevel(rank) = response {
                        info!("Loading level #{}", rank);
                    }
                    self.end_of_collection = false;
                    self.level_solved = false;
                    self.update_sprites();
                }
                MoveWorkerTo(pos, dir) => {
                    self.worker.move_to(pos);
                    self.worker.set_direction(dir);
                    // Only move worker by one tile, so we can do nice animations.  If a crate is
                    // moved, MoveCrateTo is always *before* the corresponding MoveWorkerTo, so
                    // breaking here is enough.
                    break;
                }
                MoveCrateTo(id, pos) => self.crates[id].move_to(pos),

                // Errors
                CannotMove(WithCrate(true), Obstacle::Wall) => info!("A crate hit a wall"),
                CannotMove(WithCrate(false), Obstacle::Wall) => info!("The worker hit a wall"),
                CannotMove(WithCrate(true), Obstacle::Crate) => info!("Two crates collided"),
                CannotMove(WithCrate(false), Obstacle::Crate) => info!("The worker ran into a crate"),
                NoPathfindingWhilePushing => error!("Path finding when moving crates is not implemented"),
                NothingToUndo => info!("Cannot undo move"),
                NothingToRedo => info!("Cannot redo move"),
                NoPreviousLevel => warn!("Cannot go backwards past level 1"),
                EndOfCollection => self.end_of_collection = true,
            }
        }
    }

    fn main_loop(&mut self) {
        let mut queue = VecDeque::new();
        let mut events: Vec<_>;

        loop {
            self.render_level();
            events = self.display.poll_events().collect();

            for ev in events {
                use glium::glutin::Event;
                use glium::glutin::ElementState::*;

                // Draw the current level
                let mut cmd = Command::Nothing;

                match ev {
                    Event::Closed |
                    Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::Q)) => return,

                    Event::KeyboardInput(Pressed, _, _) |
                    Event::MouseInput(..) if self.level_solved => {
                        cmd = Command::NextLevel;
                    }
                    Event::KeyboardInput(state, _, Some(key)) => {
                        use glium::glutin::VirtualKeyCode::*;
                        match key {
                            LControl | RControl => self.control_pressed = state == Pressed,
                            LShift | RShift => self.shift_pressed = state == Pressed,
                            _ if state == Pressed => cmd = self.press_to_command(key),
                            _ => (),
                        }
                    }

                    Event::MouseMoved(x, y) => self.cursor_pos = [x as f64, y as f64],
                    Event::MouseInput(Released, btn) => cmd = self.click_to_command(btn),

                    Event::Resized(w, h) => {
                        self.window_size = [w, h];
                        self.background = None;
                    }

                    /*
                       Event::KeyboardInput(_, _, None) | Event::MouseInput(Pressed, _) |
                       Event::MouseEntered | Event::MouseLeft | Event::MouseWheel(..) |
                       Event::TouchpadPressure(..) | Event::Awakened | Event::Refresh |
                       Event::Suspended(_) | Event::Touch(_) | Event::Moved(..) |
                       Event::ReceivedCharacter(_) | Event::Focused(_) | Event::DroppedFile(_) => (),
                       */
                    _ => (),
                }

                queue.extend(self.game.execute(cmd));
            }

            self.handle_responses(&mut queue);
        }
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

enum Font {
    Heading,
    Text,
    Mono,
}

/// Collection of glyph textures.
struct FontData {
    system: TextSystem,
    heading_font: FontTexture,
    text_font: FontTexture,
    mono_font: FontTexture,
}

impl FontData {
    /// Load font from disk and create a glyph texture at two different font sizes.
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(display: &GlutinFacade,
                                               font_path: P,
                                               mono_path: Q)
                                               -> Self {
        let system = TextSystem::new(display);
        let text_font = FontTexture::new(display,
                                         File::open(&font_path).unwrap(),
                                         32,
                                         glium_text::FontTexture::ascii_character_list())
                .unwrap();
        let heading_font = FontTexture::new(display,
                                            File::open(&font_path).unwrap(),
                                            64,
                                            glium_text::FontTexture::ascii_character_list())
                .unwrap();

        let mono_font = FontTexture::new(display,
                                         File::open(&mono_path).unwrap(),
                                         32,
                                         glium_text::FontTexture::ascii_character_list())
                .unwrap();

        FontData {
            system,
            heading_font,
            text_font,
            mono_font,
        }
    }

    pub fn draw(&self,
                target: &mut glium::Frame,
                text: &str,
                font_type: Font,
                scale: f32,
                offset: [f32; 2],
                aspect_ratio: f32) {

        let font = match font_type {
            Font::Heading => &self.heading_font,
            Font::Text => &self.text_font,
            Font::Mono => &self.mono_font,
        };
        let text_display = TextDisplay::new(&self.system, font, text);
        let matrix = [[scale, 0.0, 0.0, 0.0],
                      [0.0, scale / aspect_ratio, 0.0, 0.0],
                      [0.0, 0.0, 1.0, 0.0],
                      [offset[0] * scale * text_display.get_width(),
                       offset[1],
                       0.0,
                       1.0_f32]];

        let _ = glium_text::draw(&text_display, &self.system, target, matrix, WHITE);
    }
}

fn print_collections_table() {
    use ansi_term::Colour::{Blue, Green, White, Yellow};

    #[cfg(windows)]
    ansi_term::enable_ansi_support();

    println!(" {}               {}",
             Yellow.bold().paint("File name"),
             Yellow.bold().paint("Collection name"));
    println!("{0}{0}{0}{0}{0}", "----------------");

    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(ASSETS.join("levels"))
        .unwrap()
        .map(|x| x.unwrap().path().to_owned())
        .collect();
    paths.sort_by(|x, y| {
                      natord::compare(x.file_stem().unwrap().to_str().unwrap(),
                                      y.file_stem().unwrap().to_str().unwrap())
                  });

    for path in paths.into_iter() {
        if let Some(ext) = path.extension() {
            if ext == std::ffi::OsStr::new("lvl") {
                let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
                let collection = Collection::load(name).unwrap();

                if collection.is_solved() {
                    println!(" {:<24}{}{:>10} {}",
                             name,
                             White.bold().paint(format!("{:<36}", collection.name)),
                             "",
                             Green.paint("done"));
                } else {
                    println!(" {:<24}{}{:>10} {}",
                             name,
                             White.bold().paint(format!("{:<36}", collection.name)),
                             format!("{}/{}",
                                     collection.number_of_solved_levels(),
                                     collection.number_of_levels()),
                             Blue.paint("solved"));
                }
            }
        }
    }
}

fn main() {
    colog::init();

    let matches = App::new(TITLE)
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("collection")
                 .help("The level collection to load during startup")
                 .index(1))
        .arg(Arg::with_name("list")
                 .help("Print a list of available level sets")
                 .short("l")
                 .long("list"))
        .get_matches();

    // Print a list of available collections
    if matches.is_present("list") {
        print_collections_table();
        return;
    }

    let collection = match matches.value_of("collection") {
        None | Some("") => {
            std::env::var("SOKOBAN_COLLECTION").unwrap_or_else(|_| "original".to_string())
        }
        Some(c) => c.to_string(),
    };

    let mut gui = Gui::new(&collection);

    gui.main_loop();
}
