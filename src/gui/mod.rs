mod font;
mod sprite;
mod texture;

use std::cmp::min;
use std::collections::VecDeque;

use clap::{App, Arg};
use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::{VirtualKeyCode, MouseButton};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::Texture2d;

use backend::*;
use gui::font::{Font, FontData};
use gui::sprite::*;
use gui::texture::*;

/// All we ever do is draw rectangles created from two triangles each, so we don’t need any other
/// `PrimitiveType`.
const NO_INDICES: NoIndices = NoIndices(PrimitiveType::TrianglesList);


pub struct Gui {
    // Game state
    /// The main back end data structure.
    game: Game,

    /// Has the current level been solved, i.e. should the end-of-level overlay be rendered?
    level_solved: bool,

    /// Is the current level the last of this collection.
    end_of_collection: bool,

    command_queue: VecDeque<Command>,

    // Inputs
    /// Is the shift key currently pressed?
    shift_pressed: bool,

    /// Is the control key currently pressed?
    control_pressed: bool,

    /// Is a macro currently being recorded and if so, which?
    recording_macro: bool,

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


/// Constructor and getters
impl Gui {
    /// Initialize the `Gui` struct by setting default values, and loading a collection and
    /// textures.
    pub fn new(collection_name: &str) -> Self {
        use glium::DisplayBuild;
        let game = Game::new(collection_name).expect("Failed to load level set");

        let display = ::glium::glutin::WindowBuilder::new()
            .with_dimensions(800, 600)
            .with_title(TITLE.to_string() + " - " + game.name())
            .build_glium()
            .unwrap_or_else(|e| panic!("Failed to build window: {}", e));

        display
            .get_window()
            .map(|x| x.set_cursor(::glium::glutin::MouseCursor::Default));

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
            command_queue: VecDeque::new(),

            display,
            font_data,
            window_size: [800, 600],
            textures,
            background: None,

            shift_pressed: false,
            control_pressed: false,
            recording_macro: false,
            cursor_pos: [0.0, 0.0],

            worker,
            crates: vec![],
        };
        gui.update_sprites();
        gui
    }

    /// Borrow the current level.
    fn current_level(&self) -> &Level {
        self.game.current_level()
    }

    /// Compute the tile size.
    fn tile_size(&self) -> f64 {
        let columns = self.game.columns() as u32;
        let rows = self.game.rows() as u32;
        min(self.window_size[0] / columns, self.window_size[1] / rows) as f64
    }
}

// Helper functions
/// Map Fn key to their index in [F1, F2, ..., F12].
fn key_to_num(key: VirtualKeyCode) -> u8 {
    use self::VirtualKeyCode::*;
    match key {
        F1 => 0,
        F2 => 1,
        F3 => 2,
        F4 => 3,
        F5 => 4,
        F6 => 5,
        F7 => 6,
        F8 => 7,
        F9 => 8,
        F10 => 9,
        F11 => 10,
        F12 => 11,
        _ => unreachable!(),
    }
}

/// Map arrow keys to the corresponding directions, panic on other keys.
fn key_to_direction(key: VirtualKeyCode) -> Direction {
    match key {
        VirtualKeyCode::Left => Direction::Left,
        VirtualKeyCode::Right => Direction::Right,
        VirtualKeyCode::Up => Direction::Up,
        VirtualKeyCode::Down => Direction::Down,
        _ => unreachable!(),
    }
}

/// Handle user input
impl Gui {
    /// Handle key press events.
    fn press_to_command(&mut self, key: VirtualKeyCode) -> Command {
        use self::Command::*;
        use self::VirtualKeyCode::*;
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

            // Record or execute macro
            F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12 => {
                let n = key_to_num(key);
                return if self.recording_macro && self.control_pressed {
                           // Finish recording
                           self.recording_macro = false;
                           StoreMacro
                       } else if self.control_pressed {
                    // Start recording
                    self.recording_macro = true;
                    RecordMacro(n)
                } else {
                    // Execute
                    ExecuteMacro(n)
                };
            }

            // Modifier keys
            LControl | RControl => self.control_pressed = true,
            LShift | RShift => self.shift_pressed = true,

            P => return PreviousLevel,
            N => return NextLevel,

            S if self.control_pressed => return Save,

            // Open the main menu
            Escape => return ResetLevel,
            _ => error!("Unknown key: {:?}", key),
        }
        Nothing
    }

    /// Handle a mouse click.
    fn click_to_command(&self, mouse_button: MouseButton) -> Command {
        let columns = self.game.columns() as isize;
        let rows = self.game.rows() as isize;
        let tile_size = self.tile_size();

        let (offset_x, offset_y) = if self.aspect_ratio() < rows as f32 / columns as f32 {
            ((self.window_size[0] as f64 - columns as f64 * tile_size) / 2.0, 0.0)
        } else {
            (0.0, (self.window_size[1] as f64 - rows as f64 * tile_size) / 2.0)
        };

        let x = ((self.cursor_pos[0] - offset_x) / tile_size).trunc() as isize;
        let y = ((self.cursor_pos[1] - offset_y - 0.5) / tile_size).trunc() as isize;
        if x >= 0 && y >= 0 && x < columns && y < rows {
            Command::MoveToPosition(::backend::Position { x, y },
                                    MayPushCrate(mouse_button == MouseButton::Right))
        } else {
            Command::Nothing
        }
    }
}

const CULLING: ::glium::BackfaceCullingMode =
    ::glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise;

/// Rendering
impl Gui {
    /// Compute the window’s aspect ratio.
    fn aspect_ratio(&self) -> f32 {
        let width = self.window_size[0];
        let height = self.window_size[1];
        height as f32 / width as f32
    }

    /// Render the static tiles of the current level onto a texture.
    fn generate_background(&mut self) {
        use glium::Program;
        use glium::texture::Texture2d;
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
                use self::Background::*;
                let pos = lvl.position(i);
                if pos.x == 0 {
                    previous_cell = Empty;
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
            target = Texture2d::empty(&self.display, width * 2, height * 2).unwrap();
            target.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);

            let program = Program::from_source(&self.display,
                                               texture::VERTEX_SHADER,
                                               texture::FRAGMENT_SHADER,
                                               None)
                    .unwrap();
            let params = ::glium::DrawParameters {
                backface_culling: CULLING,
                blend: ::glium::Blend::alpha_blending(),
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
                    vertices.extend(texture::quad(pos, columns, rows, 1.0));
                }
                let vertex_buffer = ::glium::VertexBuffer::new(&self.display, &vertices).unwrap();

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
                    vertices.extend(texture::transition(pos, columns, rows, orientation));
                }
                let vertex_buffer = ::glium::VertexBuffer::new(&self.display, &vertices).unwrap();
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

    /// Given a vector of vertices describing a list of quads, draw them onto `target`.
    fn draw_quads(&self,
                  target: &mut ::glium::Frame,
                  vertices: Vec<Vertex>,
                  tex: &Texture2d,
                  params: &::glium::DrawParameters,
                  program: &::glium::Program)
                  -> Result<(), ::glium::DrawError> {
        let vertex_buffer = ::glium::VertexBuffer::new(&self.display, &vertices).unwrap();
        let uniforms = uniform!{tex: tex};
        target.draw(&vertex_buffer, &NO_INDICES, program, &uniforms, params)
    }

    /// Draw an overlay with some statistics.
    fn draw_end_of_level_overlay(&self,
                                 target: &mut ::glium::Frame,
                                 params: &::glium::DrawParameters) {
        use glium::Program;
        use self::texture::{VERTEX_SHADER, DARKEN_SHADER};

        let program = Program::from_source(&self.display, VERTEX_SHADER, DARKEN_SHADER, None)
            .unwrap();

        self.draw_quads(target,
                   texture::full_screen(),
                   // The texture is ignored by the given fragment shader, so we can take any here
                   &self.textures.worker, // FIXME find a cleaner solution
                   params,
                   &program)
                .unwrap();

        let aspect_ratio = self.aspect_ratio();

        // Print text
        let font_data = &self.font_data;
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

        let txt = if self.game.end_of_collection() {
            "This was the last level in this colletion. Press Q to quit."
        } else {
            "Press any key to go to the next level."
        };

        font_data.draw(target, txt, Font::Text, 0.05, [-0.5, -0.4], aspect_ratio);
    }

    /// Render the current level.
    fn render_level(&mut self) {
        let params = ::glium::DrawParameters {
            backface_culling: CULLING,
            blend: ::glium::Blend::alpha_blending(),
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
        let vertices =
            texture::background(self.aspect_ratio(), self.game.columns(), self.game.rows());
        let vertex_buffer = ::glium::VertexBuffer::new(&self.display, &vertices).unwrap();
        let program = ::glium::Program::from_source(&self.display,
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
        {
            let aspect_ratio = {
                let (width, height) = target.get_dimensions();
                width as f32 / height as f32 * rows as f32 / columns as f32
            };

            let mut draw = |vs, tex| {
                self.draw_quads(&mut target, vs, tex, &params, &program)
                    .unwrap()
            };

            // Draw the crates
            let mut vertices = vec![];
            for sprite in &self.crates {
                vertices.extend(sprite.quad(columns, rows, aspect_ratio));
            }
            draw(vertices, &self.textures.crate_);

            // Draw the worker
            draw(self.worker.quad(columns, rows, aspect_ratio),
                 &self.textures.worker);
        }

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
}

impl Gui {
    /// Handle the queue of responses from the back end, updating the gui status and logging
    /// messages.
    pub fn handle_responses(&mut self, queue: &mut VecDeque<Response>) {
        let mut steps = 0;
        while let Some(response) = queue.pop_front() {
            use self::Response::*;

            if queue.len() > 60 {
                *sprite::ANIMATION_DURATION.lock().unwrap() = 0.02_f32;
            } else if queue.len() > 20 {
                *sprite::ANIMATION_DURATION.lock().unwrap() = 0.05_f32;
            } else {
                *sprite::ANIMATION_DURATION.lock().unwrap() = 0.08_f32;
            }

            match response {
                LevelFinished(resp) => {
                    if !self.level_solved {
                        use self::save::UpdateResponse::*;
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
                    // As this makes very large levels painfully slow, allow multiple steps if the
                    // response queue is long.
                    steps = (steps + 1) % 16;
                    if steps == 0 || queue.len() < 100 {
                        break;
                    }
                }
                MoveCrateTo(id, pos) => self.crates[id].move_to(pos),

                // Errors
                // CannotMove(WithCrate(true), Obstacle::Wall) => info!("A crate hit a wall"),
                // CannotMove(WithCrate(false), Obstacle::Wall) => info!("The worker hit a wall"),
                // CannotMove(WithCrate(true), Obstacle::Crate) => info!("Two crates collided"),
                // CannotMove(WithCrate(false), Obstacle::Crate) => info!("The worker ran into a crate"),
                // NothingToUndo => info!("Cannot undo move"),
                // NothingToRedo => info!("Cannot redo move"),
                // NoPreviousLevel => warn!("Cannot go backwards past level 1"),
                // NoPathfindingWhilePushing => error!("Path finding when moving crates is not implemented"),
                EndOfCollection => self.end_of_collection = true,
                _ => {}
            }
        }
    }

    pub fn main_loop(mut self) {
        let mut queue = VecDeque::new();
        let mut events: Vec<_>;
        let mut cmd;

        loop {
            self.render_level();

            events = self.display.poll_events().collect();
            for ev in events {
                use glium::glutin::Event;
                use glium::glutin::ElementState::*;

                cmd = Command::Nothing;

                match ev {
                    Event::Closed |
                    Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::Q)) => return,

                    Event::KeyboardInput(Pressed, _, _) |
                    Event::MouseInput(..) if self.level_solved => cmd = Command::NextLevel,
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

                    // Event::KeyboardInput(_, _, None) | Event::ReceivedCharacter(_) |
                    // Event::MouseInput(Pressed, _) | Event::MouseWheel(..)
                    _ => (),
                }

                self.command_queue.push_back(cmd);
            }

            while let Some(cmd) = self.command_queue.pop_front() {
                queue.extend(self.game.execute(cmd));
            }

            self.handle_responses(&mut queue);
        }
    }
}