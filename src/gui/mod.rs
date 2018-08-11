mod font;
mod sprite;
mod texture;

use std::cmp::min;
use std::collections::VecDeque;

use glium::backend::glutin::Display;
use glium::glutin::{
    Event, KeyboardInput, ModifiersState, MouseButton, VirtualKeyCode, WindowEvent,
};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::Texture2d;
use glium::{self, Program, Surface};

use backend;
use backend::*;
use gui::font::{FontData, FontStyle};
use gui::sprite::*;
use gui::texture::*;

/// All we ever do is draw rectangles created from two triangles each, so we don’t need any other
/// `PrimitiveType`.
const NO_INDICES: NoIndices = NoIndices(PrimitiveType::TrianglesList);

/// Cull half of all faces to make rendering faster.
const CULLING: glium::BackfaceCullingMode =
    glium::draw_parameters::BackfaceCullingMode::CullCounterClockwise;

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

    rank: usize,
    rows: usize,
    columns: usize,

    worker_position: backend::Position,
    worker_direction: backend::Direction,

    /// Is the current level the last of this collection.
    is_last_level: bool,

    state: State,

    // Graphics
    display: Display,
    events_loop: glium::glutin::EventsLoop,
    params: glium::DrawParameters<'static>,
    font_data: FontData,
    matrix: [[f32; 4]; 4],

    program: Program,

    /// The size of the window in pixels as `[width, height]`.
    window_size: [u32; 2],

    /// Tile textures, i.e. wall, worker, crate, etc.
    textures: Textures,

    /// Pre-rendered static part of the current level, i.e. walls, floors and goals.
    background_texture: Option<Texture2d>,

    worker: Sprite,
    crates: Vec<Sprite>,
}

/// Constructor and getters
impl Gui {
    /// Initialize the `Gui` struct by setting default values, and loading a collection and
    /// textures.
    pub fn new(collection_name: &str) -> Self {
        let game = Game::load(collection_name).expect("Failed to load level set");

        let events_loop = glium::glutin::EventsLoop::new();
        let window = glium::glutin::WindowBuilder::new()
            .with_dimensions(800, 600)
            .with_title(TITLE.to_string() + " - " + game.name());

        let context = glium::glutin::ContextBuilder::new();
        let display = glium::Display::new(window, context, &events_loop).unwrap();
        display
            .gl_window()
            .set_cursor(glium::glutin::MouseCursor::Default);

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
        let params = glium::DrawParameters {
            backface_culling: CULLING,
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        let mut gui = Gui {
            columns: game.columns(),
            rows: game.rows(),
            rank: game.rank(),
            worker_position: game.worker_position(),
            worker_direction: game.worker_direction(),
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
            background_texture: None,

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
        let columns = self.columns as u32;
        let rows = self.rows as u32;
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
        self.window_aspect_ratio() * self.columns as f32 / self.rows as f32
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

impl Gui {
    /// Handle a mouse click.
    fn click_to_command(&self, mouse_button: MouseButton, input_state: &InputState) -> Command {
        if let Some((x, y)) =
            self.cursor_position_to_cell_if_in_bounds(&input_state.cursor_position)
        {
            Command::MoveToPosition {
                position: backend::Position { x, y },
                may_push_crate: mouse_button == MouseButton::Right,
            }
        } else {
            Command::Nothing
        }
    }

    fn cursor_position_to_cell_if_in_bounds(
        &self,
        cursor_position: &[f64],
    ) -> Option<(isize, isize)> {
        let (offset_x, offset_y) = self.compute_offsets();;
        let tile_size = self.tile_size();

        let x = ((cursor_position[0] - offset_x) / tile_size).trunc() as isize;
        let y = ((cursor_position[1] - offset_y - 0.5) / tile_size).trunc() as isize;

        if x > 0 && y > 0 && x < self.columns as isize - 1 && y < self.rows as isize - 1 {
            Some((x, y))
        } else {
            None
        }
    }

    fn compute_offsets(&self) -> (f64, f64) {
        let tile_size = self.tile_size();
        if self.aspect_ratio_ratio() < 1.0 {
            let offset_x = (f64::from(self.window_size[0]) - self.columns as f64 * tile_size) / 2.0;
            (offset_x, 0.0)
        } else {
            let offset_y = (f64::from(self.window_size[1]) - self.rows as f64 * tile_size) / 2.0;
            (0.0, offset_y)
        }
    }
}

fn correct_aspect_ratio_matrix(aspect_ratio: f32) -> [[f32; 4]; 4] {
    let a_r = aspect_ratio;
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
}

fn generate_vertices_for(level: &Level, cell_type: Background) -> Vec<Vertex> {
    let columns = level.columns() as u32;
    let rows = level.rows() as u32;
    let mut vertices = vec![];
    for (i, _) in level
        .background_cells()
        .iter()
        .enumerate()
        .filter(|(_, &cell)| cell == cell_type)
    {
        let pos = level.position(i);
        vertices.extend(texture::quad(pos, columns, rows));
    }
    vertices
}

/// Rendering
impl Gui {
    /// Render the static tiles of the current level onto a texture.
    fn generate_background(&mut self) {
        let target = self.generate_empty_background_texture();

        self.matrix = correct_aspect_ratio_matrix(self.aspect_ratio_ratio());
        let program = &self.program;

        // We need this block so the last borrow of `self` ends before we need to borrow
        // `self.background_texture` mutably at the end.
        {
            let level = self.current_level();

            // Render each of the (square) tiles
            for &background in &[Background::Floor, Background::Goal, Background::Wall] {
                let vertices = generate_vertices_for(&level, background);
                let vb = glium::VertexBuffer::new(&self.display, &vertices).unwrap();

                let texture = self.background_to_texture(background);
                let uniforms = uniform!{tex: texture, matrix: self.matrix};

                target
                    .as_surface()
                    .draw(&vb, &NO_INDICES, program, &uniforms, &self.params)
                    .unwrap();
            }
        }

        self.background_texture = Some(target);
    }

    fn background_to_texture(&self, background: Background) -> &Texture2d {
        match background {
            Background::Empty => unreachable!(),
            Background::Floor => &self.textures.floor,
            Background::Goal => &self.textures.goal,
            Background::Wall => &self.textures.wall,
        }
    }

    fn generate_empty_background_texture(&self) -> Texture2d {
        let width = self.window_size[0];
        let height = self.window_size[1];
        let target = Texture2d::empty(&self.display, width, height).unwrap();
        target.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);
        target
    }

    /// Create sprites for movable entities of the current level.
    fn update_sprites(&mut self) {
        self.worker = Sprite::new(self.worker_position, texture::TileKind::Worker);
        self.worker.set_direction(self.worker_direction);
        self.crates = self
            .game
            .crate_positions()
            .iter()
            .map(|&pos| Sprite::new(pos, texture::TileKind::Crate))
            .collect();
        // TODO simplify hashmap -> iter -> vec -> iter -> vec -> iter -> vec

        self.background_texture = None;
    }

    /// Given a vector of vertices describing a list of quads, draw them onto `target`.
    fn draw_quads<S: Surface, V: AsRef<Vec<Vertex>>>(
        &self,
        target: &mut S,
        vertices: V,
        tex: &Texture2d,
        program: &glium::Program,
    ) -> Result<(), glium::DrawError> {
        let vb = glium::VertexBuffer::new(&self.display, vertices.as_ref()).unwrap();
        let uniforms = uniform!{tex: tex, matrix: self.matrix};
        target.draw(&vb, &NO_INDICES, program, &uniforms, &self.params)
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
            self.rank,
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

        let txt = self.end_of_level_text();;

        font_data.draw(
            target,
            txt,
            FontStyle::Text,
            0.05,
            [-0.5, -0.4],
            aspect_ratio,
        );
    }

    fn end_of_level_text(&self) -> &str {
        if self.game.is_last_level() {
            "This was the last level in this colletion. Press Q to quit."
        } else {
            "Press any key to go to the next level."
        }
    }

    /// Render the current level.
    fn render_level(&mut self) {
        self.generate_background_if_none();

        let columns = self.columns as u32;
        let rows = self.rows as u32;

        // Draw background
        let vertices = texture::full_screen();
        let vb = glium::VertexBuffer::new(&self.display, &vertices).unwrap();

        let bg = self.background_texture.as_ref().unwrap();
        let uniforms = uniform!{tex: bg, matrix: IDENTITY};
        let program = &self.program;

        let mut target = self.display.draw();

        target.clear_color(0.0, 0.0, 0.0, 1.0); // Prevent artefacts when resizing the window

        target
            .draw(&vb, &NO_INDICES, program, &uniforms, &self.params)
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

    fn generate_background_if_none(&mut self) {
        if self.background_texture.is_none() {
            self.generate_background();
        }
    }

    fn render_end_of_level(&mut self) {
        // TODO extract functions, reduce duplication with render_level()
        let vertices = texture::full_screen();
        let vb = glium::VertexBuffer::new(&self.display, &vertices).unwrap();

        if self.background_texture.is_none() {
            // Render the end-of-level screen and store it in self.bg
            let columns = self.columns as u32;
            let rows = self.rows as u32;

            self.generate_background();
            let width = self.window_size[0];
            let height = self.window_size[1];
            let texture = Texture2d::empty(&self.display, width, height).unwrap();

            {
                let mut target = texture.as_surface();
                let bg = self.background_texture.as_ref().unwrap();
                let uniforms = uniform!{tex: bg, matrix: IDENTITY};
                let program = &self.program;

                // Prevent artefacts when resizing the window
                target.clear_color(0.0, 0.0, 0.0, 1.0);

                target
                    .draw(&vb, &NO_INDICES, program, &uniforms, &self.params)
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

            self.background_texture = Some(texture);
            self.render_level();
        } else {
            self.draw_background(&vb);
        }
    }

    /// Fill the screen with the cached background image
    fn draw_background(&self, vb: &glium::VertexBuffer<Vertex>) {
        let bg = self.background_texture.as_ref().unwrap();
        let uniforms = uniform!{tex: bg, matrix: IDENTITY};
        let mut target = self.display.draw();

        target
            .draw(vb, &NO_INDICES, &self.program, &uniforms, &self.params)
            .unwrap();
        target.finish().unwrap();
    }

    fn render(&mut self) {
        match self.state {
            State::Level => self.render_level(),
            State::FinishAnimation => {
                self.render_level();
                if !self.worker.is_animated() {
                    self.background_texture = None;
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
    fn press_to_command(&mut self, key: VirtualKeyCode, modifiers: ModifiersState) -> Command {
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
                    MoveAsFarAsPossible {
                        direction: dir,
                        may_push_crate: modifiers.shift,
                    }
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

            // TODO Open the main menu
            Escape => return ResetLevel,
            LAlt | LControl | LMenu | LShift | LWin | RAlt | RControl | RMenu | RShift | RWin => {}
            _ => error!("Unknown key: {:?}", key),
        }
        Nothing
    }
}

fn set_animation_duration(queue_length: usize) {
    if queue_length > 60 {
        *sprite::ANIMATION_DURATION.lock().unwrap() = 0.02_f32;
    } else if queue_length > 20 {
        *sprite::ANIMATION_DURATION.lock().unwrap() = 0.05_f32;
    } else {
        *sprite::ANIMATION_DURATION.lock().unwrap() = 0.08_f32;
    }
}

fn log_update_response(response: save::UpdateResponse) {
    use self::save::UpdateResponse::*;
    match response {
        FirstTimeSolved => info!(
            "You have successfully solved this level for the first time! \
             Congratulations!"
        ),
        Update {
            moves: true,
            pushes: true,
        } => info!("Your solution uses the least moves and pushes!"),
        Update { moves: true, .. } => info!("Your solution is the best so far in terms of moves!"),
        Update { pushes: true, .. } => {
            info!("Your solution is the best so far in terms of pushes!")
        }
        Update { .. } => info!("Solved the level without creating a new high score."),
    }
}

impl Gui {
    /// Handle the queue of responses from the back end, updating the gui status and logging
    /// messages.
    fn handle_responses(&mut self, queue: &mut VecDeque<Response>) {
        const SKIP_FRAMES: u32 = 16;
        const QUEUE_LENGTH_THRESHOLD: usize = 100;
        let mut steps = 0;
        while let Some(response) = queue.pop_front() {
            set_animation_duration(queue.len());

            let is_move = self.handle_response(&response);
            if is_move {
                // Only move worker by one tile, so we can do nice animations.  If a crate is
                // moved, MoveCrateTo is always *before* the corresponding MoveWorkerTo, so
                // breaking here is enough.
                // As this makes very large levels painfully slow, allow multiple steps if the
                // response queue is long.
                steps = (steps + 1) % SKIP_FRAMES;
                if steps == 0 || queue.len() < QUEUE_LENGTH_THRESHOLD {
                    break;
                }
                // TODO this sort of works, but can we find a better way to skip animation
                // steps?
            }
        }
    }

    fn handle_response(&mut self, response: &Response) -> bool {
        use self::Response::*;
        match *response {
            LevelFinished(resp) if !self.level_solved() => {
                self.state = State::FinishAnimation;
                log_update_response(resp);
            }
            LevelFinished(_) => {}
            NewLevel {
                rank,
                columns,
                rows,
                worker_position,
                worker_direction,
            } => {
                // TODO replace with observer pattern?
                if rank != self.rank {
                    info!("Loading level #{}", rank);
                    self.rank = rank;
                    self.columns = columns;
                    self.rows = rows;
                }

                self.worker_position = worker_position;
                self.worker_direction = worker_direction;
                self.is_last_level = false;

                self.state = State::Level;
                self.update_sprites();
            }
            MoveWorkerTo(pos, dir) => {
                self.worker.move_to(pos);
                self.worker.set_direction(dir);
                return true;
            }
            MoveCrateTo(id, pos) => self.crates[id].move_to(pos),

            EndOfCollection => self.is_last_level = true,
            _ => {}
        }

        false
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
                    | WindowEvent::MouseInput { .. }
                        if self.level_solved() =>
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
                    } => cmd = input_state.press_to_command(key, modifiers),

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
                        self.background_texture = None;
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
