mod font;
mod sprite;
mod texture;

use std::cmp::min;
use std::collections::VecDeque;

use glium::backend::glutin::Display;
use glium::glutin::{Event, KeyboardInput, ModifiersState, MouseButton, VirtualKeyCode, WindowEvent};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::Texture2d;
use glium::{Program, Surface};

use backend::*;
use gui::font::{FontData, FontStyle};
use gui::sprite::*;
use gui::texture::*;

/// All we ever do is draw rectangles created from two triangles each, so we don’t need any other
/// `PrimitiveType`.
const NO_INDICES: NoIndices = NoIndices(PrimitiveType::TrianglesList);

/// Cull half of all faces to make rendering faster.
const CULLING: ::glium::BackfaceCullingMode =
    ::glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise;

const IDENTITY: [[f32; 4]; 4] = {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
};

enum State {
    Level,
    FinishAnimation,
    LevelSolved,
}

pub struct Gui {
    // Game state
    /// The main back end data structure.
    game: Game,

    /// Is the current level the last of this collection.
    is_last_level: bool,

    state: State,

    // Graphics
    display: Display,
    events_loop: ::glium::glutin::EventsLoop,
    params: ::glium::DrawParameters<'static>,
    font_data: FontData,
    matrix: [[f32; 4]; 4],

    program: Program,

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
        use glium;
        let game = Game::new(collection_name).expect("Failed to load level set");

        let events_loop = glium::glutin::EventsLoop::new();
        let window = glium::glutin::WindowBuilder::new()
            .with_dimensions(800, 600)
            .with_title(TITLE.to_string() + " - " + game.name());

        let context = glium::glutin::ContextBuilder::new();
        let display = glium::Display::new(window, context, &events_loop).unwrap();
        display
            .gl_window()
            .set_cursor(::glium::glutin::MouseCursor::Default);

        let textures = Textures::new(&display);
        let font_data = FontData::new(
            &display,
            ASSETS.join("FiraSans-Regular.ttf"),
            ASSETS.join("FiraMono-Regular.ttf"),
        );
        let program = Program::from_source(
            &display,
            texture::VERTEX_SHADER,
            texture::FRAGMENT_SHADER,
            None,
        ).unwrap();

        let worker = Sprite::new(game.worker_position(), texture::TileKind::Worker);
        // FIXME code duplicated from Gui::update_sprites()

        info!(
            "Loading level #{} of collection {}",
            game.rank(),
            game.name()
        );
        let params = ::glium::DrawParameters {
            backface_culling: CULLING,
            blend: ::glium::Blend::alpha_blending(),
            ..Default::default()
        };

        let mut gui = Gui {
            game,
            is_last_level: false,
            state: State::Level,

            display,
            events_loop,
            params,
            font_data,
            matrix: IDENTITY,
            program,
            window_size: [800, 600],
            textures,
            background: None,

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
        f64::from(min(
            self.window_size[0] / columns,
            self.window_size[1] / rows,
        ))
    }

    /// Compute the window’s aspect ratio.
    fn window_aspect_ratio(&self) -> f32 {
        let width = self.window_size[0] as f32;
        let height = self.window_size[1] as f32;
        height / width
    }

    /// Ratio between the window’s and the level’s aspect ratio.
    fn aspect_ratio_ratio(&self) -> f32 {
        self.window_aspect_ratio() * self.game.columns() as f32 / self.game.rows() as f32
    }

    /// Has the current level been solved, i.e. should the end-of-level overlay be rendered?
    fn level_solved(&self) -> bool {
        match self.state {
            State::Level => false,
            _ => true,
        }
    }
}

// Helper functions for input handling
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
    /// Handle a mouse click.
    fn click_to_command(&self, mouse_button: MouseButton, input_state: &InputState) -> Command {
        let columns = self.game.columns() as isize;
        let rows = self.game.rows() as isize;
        let tile_size = self.tile_size();

        let (offset_x, offset_y) = if self.aspect_ratio_ratio() < 1.0 {
            (
                (f64::from(self.window_size[0]) - columns as f64 * tile_size) / 2.0,
                0.0,
            )
        } else {
            (
                0.0,
                (f64::from(self.window_size[1]) - rows as f64 * tile_size) / 2.0,
            )
        };

        let x = ((input_state.cursor_position[0] - offset_x) / tile_size).trunc() as isize;
        let y = ((input_state.cursor_position[1] - offset_y - 0.5) / tile_size).trunc() as isize;
        if x > 0 && y > 0 && x < columns - 1 && y < rows - 1 {
            Command::MoveToPosition(
                ::backend::Position { x, y },
                MayPushCrate(mouse_button == MouseButton::Right),
            )
        } else {
            Command::Nothing
        }
    }
}

/// Rendering
impl Gui {
    /// Render the static tiles of the current level onto a texture.
    fn generate_background(&mut self) {
        use glium::texture::Texture2d;
        let target;

        let columns = self.game.columns() as u32;
        let rows = self.game.rows() as u32;

        {
            self.matrix = {
                let a_r = self.aspect_ratio_ratio();
                if a_r < 1.0 {
                    [
                        [a_r, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ]
                } else {
                    let a_r = 1.0 / a_r;
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, a_r, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ]
                }
            };
            let lvl = self.current_level();

            // Create texture
            target = {
                let width = self.window_size[0];
                let height = self.window_size[1];
                Texture2d::empty(&self.display, width, height).unwrap()
            };
            target.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);

            let program = &self.program;

            // Render each of the (square) tiles
            for &value in &[Background::Floor, Background::Goal, Background::Wall] {
                let mut vertices = vec![];
                for (i, &cell) in lvl.background.iter().enumerate() {
                    if cell != value {
                        continue;
                    }
                    let pos = lvl.position(i);
                    vertices.extend(texture::quad(pos, columns, rows));
                }
                let vertex_buffer = ::glium::VertexBuffer::new(&self.display, &vertices).unwrap();

                let texture = match value {
                    Background::Empty => continue,
                    Background::Floor => &self.textures.floor,
                    Background::Goal => &self.textures.goal,
                    Background::Wall => &self.textures.wall,
                };
                let uniforms = uniform!{tex: texture, matrix: self.matrix};

                target
                    .as_surface()
                    .draw(
                        &vertex_buffer,
                        &NO_INDICES,
                        program,
                        &uniforms,
                        &self.params,
                    )
                    .unwrap();
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
    fn draw_quads<S: Surface, V: AsRef<Vec<Vertex>>>(
        &self,
        target: &mut S,
        vertices: V,
        tex: &Texture2d,
        program: &::glium::Program,
    ) -> Result<(), ::glium::DrawError> {
        let vertex_buffer = ::glium::VertexBuffer::new(&self.display, vertices.as_ref()).unwrap();
        let uniforms = uniform!{tex: tex, matrix: self.matrix};
        target.draw(
            &vertex_buffer,
            &NO_INDICES,
            program,
            &uniforms,
            &self.params,
        )
    }

    /// Draw an overlay with some statistics.
    fn draw_end_of_level_overlay<S: Surface>(&self, target: &mut S) {
        use self::texture::{DARKEN_SHADER, VERTEX_SHADER};
        use glium::Program;

        let program =
            Program::from_source(&self.display, VERTEX_SHADER, DARKEN_SHADER, None).unwrap();

        self.draw_quads(
            target,
            texture::full_screen(),
            // The texture is ignored by the given fragment shader, so we can take any here
            &self.textures.worker, // FIXME find a cleaner solution
            &program,
        ).unwrap();

        let aspect_ratio = self.window_aspect_ratio();

        // Print text
        let font_data = &self.font_data;
        font_data.draw(
            target,
            "Congratulations!",
            FontStyle::Heading,
            0.1,
            [-0.5, 0.2],
            aspect_ratio,
        );

        let stats_text = format!(
            "You have finished the level {} using {} moves, \
             {} of which moved a crate.",
            self.game.rank(),
            self.game.number_of_moves(),
            self.game.number_of_pushes()
        );

        font_data.draw(
            target,
            &stats_text,
            FontStyle::Text,
            0.05,
            [-0.5, -0.2],
            aspect_ratio,
        );

        let txt = if self.game.is_last_level() {
            "This was the last level in this colletion. Press Q to quit."
        } else {
            "Press any key to go to the next level."
        };

        font_data.draw(
            target,
            txt,
            FontStyle::Text,
            0.05,
            [-0.5, -0.4],
            aspect_ratio,
        );
    }

    /// Render the current level.
    fn render_level(&mut self) {
        // Do we have to update the cache?
        if self.background.is_none() {
            self.generate_background();
        }

        let columns = self.game.columns() as u32;
        let rows = self.game.rows() as u32;

        // Draw background
        let vertices = texture::full_screen();
        let vertex_buffer = ::glium::VertexBuffer::new(&self.display, &vertices).unwrap();

        let bg = self.background.as_ref().unwrap();
        let uniforms = uniform!{tex: bg, matrix: IDENTITY};
        let program = &self.program;

        let mut target = self.display.draw();

        target.clear_color(0.0, 0.0, 0.0, 1.0); // Prevent artefacts when resizing the window

        target
            .draw(
                &vertex_buffer,
                &NO_INDICES,
                program,
                &uniforms,
                &self.params,
            )
            .unwrap();

        // Draw foreground
        {
            let mut draw = |vs, tex| self.draw_quads(&mut target, vs, tex, program).unwrap();

            // Draw the crates
            let mut vertices = vec![];
            for sprite in &self.crates {
                vertices.extend(sprite.quad(columns, rows));
            }
            draw(vertices, &self.textures.crate_);

            // Draw the worker
            draw(self.worker.quad(columns, rows), &self.textures.worker);
        }

        // Display text overlay
        let aspect_ratio = self.window_aspect_ratio();
        // TODO show collection name
        // Show some statistics
        let text = format!(
            "Level: {}, Steps: {}, Pushes: {}",
            self.game.rank(),
            self.game.number_of_moves(),
            self.game.number_of_pushes()
        );

        self.font_data.draw(
            &mut target,
            &text,
            FontStyle::Mono,
            0.04,
            [0.5, -0.9],
            aspect_ratio,
        );

        target.finish().unwrap();
    }

    fn render_end_of_level(&mut self) {
        let vertices = texture::full_screen();
        let vertex_buffer = ::glium::VertexBuffer::new(&self.display, &vertices).unwrap();

        if self.background.is_none() {
            // Render the end-of-level screen and store it in self.bg
            let columns = self.game.columns() as u32;
            let rows = self.game.rows() as u32;

            self.generate_background();
            let width = self.window_size[0];
            let height = self.window_size[1];
            let texture = Texture2d::empty(&self.display, width, height).unwrap();

            {
                let mut target = texture.as_surface();
                let bg = self.background.as_ref().unwrap();
                let uniforms = uniform!{tex: bg, matrix: IDENTITY};
                let program = &self.program;

                // Prevent artefacts when resizing the window
                target.clear_color(0.0, 0.0, 0.0, 1.0);

                target
                    .draw(
                        &vertex_buffer,
                        &NO_INDICES,
                        program,
                        &uniforms,
                        &self.params,
                    )
                    .unwrap();

                // Draw foreground
                {
                    let mut draw =
                        |vs, tex| self.draw_quads(&mut target, vs, tex, program).unwrap();

                    // Draw the crates
                    let mut vertices = vec![];
                    for sprite in &self.crates {
                        vertices.extend(sprite.quad(columns, rows));
                    }
                    draw(vertices, &self.textures.crate_);

                    // Draw the worker
                    draw(self.worker.quad(columns, rows), &self.textures.worker);
                }

                // Display text overlay
                self.draw_end_of_level_overlay(&mut target);
            }

            self.background = Some(texture);
            self.render_level();
        } else {
            // Fill the screen with the cached image
            let bg = self.background.as_ref().unwrap();
            let uniforms = uniform!{tex: bg, matrix: IDENTITY};
            let mut target = self.display.draw();

            target
                .draw(
                    &vertex_buffer,
                    &NO_INDICES,
                    &self.program,
                    &uniforms,
                    &self.params,
                )
                .unwrap();
            target.finish().unwrap();
        }
    }

    fn render(&mut self) {
        match self.state {
            State::Level => self.render_level(),
            State::FinishAnimation => {
                self.render_level();
                if !self.worker.is_animated() {
                    self.background = None;
                    self.state = State::LevelSolved;
                }
            }
            State::LevelSolved => self.render_end_of_level(),
        }
    }
}

#[derive(Default)]
struct InputState {
    recording_macro: bool,
    cursor_position: [f64; 2],
}

impl InputState {
    /// Handle key press events.
    fn press_to_command(&mut self, key: VirtualKeyCode, modifiers: &ModifiersState) -> Command {
        use self::Command::*;
        use self::VirtualKeyCode::*;
        match key {
            // Move
            Left | Right | Up | Down => {
                let dir = key_to_direction(key);
                return if !modifiers.ctrl && !modifiers.shift {
                    Move(dir)
                } else if modifiers.ctrl && modifiers.shift {
                    Nothing
                } else {
                    MoveAsFarAsPossible(dir, MayPushCrate(modifiers.shift))
                };
            }

            // Undo and redo
            Z if !modifiers.ctrl => {}
            U if modifiers.ctrl => {}
            U | Z if modifiers.shift => return Redo,
            U | Z => return Undo,

            // Record or execute macro
            F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12 => {
                let n = key_to_num(key);
                return if self.recording_macro && modifiers.ctrl {
                    // Finish recording
                    self.recording_macro = false;
                    StoreMacro
                } else if modifiers.ctrl {
                    // Start recording
                    self.recording_macro = true;
                    RecordMacro(n)
                } else {
                    // Execute
                    ExecuteMacro(n)
                };
            }

            P => return PreviousLevel,
            N => return NextLevel,

            S if modifiers.ctrl => return Save,

            // Open the main menu
            Escape => return ResetLevel,
            LAlt | LControl | LMenu | LShift | LWin | RAlt | RControl | RMenu | RShift | RWin => {}
            _ => error!("Unknown key: {:?}", key),
        }
        Nothing
    }
}

impl Gui {
    /// Handle the queue of responses from the back end, updating the gui status and logging
    /// messages.
    fn handle_responses(&mut self, queue: &mut VecDeque<Response>) {
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
                    if !self.level_solved() {
                        use self::save::UpdateResponse::*;
                        self.state = State::FinishAnimation;
                        match resp {
                            FirstTimeSolved => info!(
                                "You have successfully solved this level for the first time! \
                                 Congratulations!"
                            ),
                            Update { moves, pushes } => {
                                if moves && pushes {
                                    info!("Your solution uses the least moves and pushes!");
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
                    self.is_last_level = false;
                    self.state = State::Level;
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
                // CannotMove(WithCrate(true), Obstacle::Wall) => /* A crate hit a wall */
                // CannotMove(WithCrate(false), Obstacle::Wall) => /* The worker hit a wall */
                // CannotMove(WithCrate(true), Obstacle::Crate) => /* Two crates collided */
                // CannotMove(WithCrate(false), Obstacle::Crate) => /* The worker hit a crate */
                // NothingToUndo => /* Cannot undo move */
                // NothingToRedo => /* Cannot redo move */
                // NoPreviousLevel => /* Cannot go backwards past level 1 */
                // NoPathfindingWhilePushing => /* Path finding pusing crates unimplemented */
                EndOfCollection => self.is_last_level = true,
                _ => {}
            }
        }
    }

    pub fn main_loop(mut self) {
        let mut queue = VecDeque::new();
        let mut command_queue = VecDeque::new();
        let mut input_state: InputState = Default::default();

        loop {
            use glium::glutin::ElementState::*;
            self.render();

            let mut events = vec![];
            self.events_loop.poll_events(|ev: Event| match ev {
                Event::Awakened | Event::Suspended(_) | Event::DeviceEvent { .. } => {}
                Event::WindowEvent { event, .. } => events.push(event),
            });

            for event in events {
                let mut cmd = Command::Nothing;

                match event {
                    WindowEvent::Closed
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Q),
                                ..
                            },
                        ..
                    } => return,

                    WindowEvent::KeyboardInput {
                        input: KeyboardInput { state: Pressed, .. },
                        ..
                    }
                    | WindowEvent::MouseInput { .. } if self.level_solved() =>
                    {
                        cmd = Command::NextLevel
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: Pressed,
                                virtual_keycode: Some(key),
                                modifiers,
                                ..
                            },
                        ..
                    } => cmd = input_state.press_to_command(key, &modifiers),

                    WindowEvent::CursorMoved {
                        position: (x, y), ..
                    } => input_state.cursor_position = [x, y],
                    WindowEvent::MouseInput {
                        state: Released,
                        button: btn,
                        ..
                    } => cmd = self.click_to_command(btn, &input_state),

                    WindowEvent::Resized(w, h) => {
                        self.window_size = [w, h];
                        self.background = None;
                    }

                    // WindowEvent::KeyboardInput(_, _, None) | WindowEvent::ReceivedCharacter(_) |
                    // WindowEvent::MouseInput(Pressed, _) | WindowEvent::MouseWheel(..)
                    _ => (),
                }

                command_queue.push_back(cmd);
            }

            while let Some(cmd) = command_queue.pop_front() {
                queue.extend(self.game.execute(&cmd));
            }

            self.handle_responses(&mut queue);
        }
    }
}
