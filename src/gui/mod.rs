pub mod inputstate;
mod sprite;
mod texture;

use std::{
    cmp::min,
    collections::VecDeque,
    sync::mpsc::{channel, Receiver},
};

use glium::{
    self,
    backend::glutin::Display,
    glutin::{self, dpi},
    glutin::event::{ModifiersState, MouseButton},
    index::{NoIndices, PrimitiveType},
    texture::Texture2d,
    Program, Surface,
};

use crate::backend;
use crate::backend::*;
use crate::gui::inputstate::*;
use crate::gui::sprite::*;
use crate::gui::texture::*;

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
    pub game: Game,

    pub rank: usize,
    pub rows: usize,
    pub columns: usize,

    pub worker_position: backend::Position,
    pub worker_direction: backend::Direction,

    /// Is the current level the last of this collection.
    pub is_last_level: bool,

    state: State,

    // Graphics
    pub display: Display,
    pub params: glium::DrawParameters<'static>,
    // font_data: Rc<FontData>,
    // text_object_manager: TextObjectManager,
    // stats_text_handle: TextObjectHandle,
    pub matrix: [[f32; 4]; 4],

    pub program: Program,

    /// The size of the window in pixels as `[width, height]`.
    pub window_size: [u32; 2],

    /// Tile textures, i.e. wall, worker, crate, etc.
    pub textures: Textures,

    /// Pre-rendered static part of the current level, i.e. walls, floors and goals.
    pub background_texture: Option<Texture2d>,

    pub worker: Sprite,
    pub crates: Vec<Sprite>,

    pub need_to_redraw: bool,

    pub events: Receiver<backend::Event>,
}

/// Constructor and getters
impl Gui {
    /// Initialize the `Gui` struct by setting default values, and loading a collection and
    /// textures.
    pub fn new(mut game: Game, events_loop: &glutin::event_loop::EventLoop<()>) -> Self {
        let window = glutin::window::WindowBuilder::new()
            .with_inner_size(dpi::LogicalSize::new(800.0, 600.0))
            .with_title(TITLE.to_string() + " - " + game.name());

        let context = glutin::ContextBuilder::new();
        let display = glium::Display::new(window, context, events_loop).unwrap();
        display
            .gl_window()
            .window()
            .set_cursor_icon(glutin::window::CursorIcon::Default);

        let textures = Textures::new(&display);
        // let font_data = Rc::new(FontData::new(
        //     &display,
        //     ASSETS.join("FiraSans-Regular.ttf"),
        //     ASSETS.join("FiraMono-Regular.ttf"),
        // ));
        let program = Program::from_source(
            &display,
            texture::VERTEX_SHADER,
            texture::FRAGMENT_SHADER,
            None,
        )
        .unwrap();
        let params = glium::DrawParameters {
            backface_culling: CULLING,
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        // let (text_object_manager, stats_text_handle) = init_stats_text(&font_data);

        let worker = Sprite::new(game.worker_position(), texture::TileKind::Worker);
        // FIXME code duplicated from Gui::update_sprites()

        let (sender, receiver) = channel();
        game.subscribe_moves(sender);

        info!(
            "Loading level #{} of collection {}",
            game.rank(),
            game.name()
        );

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
            params,
            // font_data,
            // text_object_manager,
            // stats_text_handle,
            matrix: IDENTITY,
            program,
            window_size: [800, 600],
            textures,
            background_texture: None,

            worker,
            crates: vec![],
            need_to_redraw: true,

            events: receiver,
        };

        gui.update_statistics_text();
        gui.update_sprites();

        gui
    }

    /// Borrow the current level.
    fn current_level(&self) -> &CurrentLevel {
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
    pub fn level_solved(&self) -> bool {
        match self.state {
            State::Level => false,
            _ => true,
        }
    }
}

impl Gui {
    /// Handle a mouse click.
    pub fn click_to_command(
        &self,
        mouse_button: MouseButton,
        modifiers: ModifiersState,
        input_state: &mut InputState,
    ) -> Command {
        if let Some((x, y)) =
            self.cursor_position_to_cell_if_in_bounds(&input_state.cursor_position)
        {
            let target = backend::Position { x, y };
            if mouse_button == MouseButton::Left && modifiers.alt() {
                if let Some(from) = input_state.clicked_crate {
                    let result =
                        Command::Movement(Movement::MoveCrateToTarget { from, to: target });
                    input_state.clicked_crate = None;
                    result
                } else {
                    input_state.clicked_crate = Some(target);
                    Command::Nothing
                }
            } else {
                let worker = self.worker_position;
                let same_row_or_column = target.x == worker.x || target.y == worker.y;
                let can_move_crate = mouse_button == MouseButton::Right;

                match (same_row_or_column, can_move_crate) {
                    (true, true) => Command::Movement(Movement::PushTowards { position: target }),
                    (true, false) => Command::Movement(Movement::WalkTowards { position: target }),
                    (false, false) => {
                        Command::Movement(Movement::WalkToPosition { position: target })
                    }
                    (false, true) => {
                        warn!("Cannot push crate to a different row and column.");
                        Command::Nothing
                    }
                }
            }
        } else {
            Command::Nothing
        }
    }

    fn cursor_position_to_cell_if_in_bounds(
        &self,
        cursor_position: &[f64],
    ) -> Option<(isize, isize)> {
        let (offset_x, offset_y) = self.compute_offsets();
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

fn generate_vertices_for(level: &CurrentLevel, cell_type: Background) -> Vec<Vertex> {
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
            let mut surface = target.as_surface();

            // Render each of the (square) tiles
            for &background in &[Background::Floor, Background::Goal, Background::Wall] {
                let vertices = generate_vertices_for(level, background);
                let vb = glium::VertexBuffer::new(&self.display, &vertices).unwrap();

                let texture = self.background_to_texture(background);
                let uniforms = uniform! {tex: texture, matrix: self.matrix};

                surface
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
        let uniforms = uniform! {tex: tex, matrix: self.matrix};
        target.draw(&vb, &NO_INDICES, program, &uniforms, &self.params)
    }

    /// Draw an overlay with some statistics.
    fn draw_end_of_level_overlay<S: Surface>(&self, target: &mut S) {
        let program =
            Program::from_source(&self.display, VERTEX_SHADER, DARKEN_SHADER, None).unwrap();

        self.draw_quads(
            target,
            texture::full_screen(),
            // The texture is ignored by the given fragment shader, so we can take any here
            &self.textures.worker, // FIXME find a cleaner solution
            &program,
        )
        .unwrap();

        let aspect_ratio = self.window_aspect_ratio();

        // Print text
        // let font_data = &self.font_data;
        // font_data.draw(
        //     target,
        //     "Congratulations!",
        //     FontStyle::Heading,
        //     0.08,
        //     [-0.5, 0.2],
        //     aspect_ratio,
        // );

        // let stats_text = format!(
        //     "You have finished the level {} using {} moves, \
        //      {} of which moved a crate.",
        //     self.rank,
        //     self.game.number_of_moves(),
        //     self.game.number_of_pushes()
        // );

        // font_data.draw(
        //     target,
        //     &stats_text,
        //     FontStyle::Text,
        //     0.035,
        //     [-0.5, -0.2],
        //     aspect_ratio,
        // );

        // let txt = self.end_of_level_text();

        // font_data.draw(
        //     target,
        //     txt,
        //     FontStyle::Text,
        //     0.035,
        //     [-0.5, -0.4],
        //     aspect_ratio,
        // );
    }

    fn end_of_level_text(&self) -> &str {
        if self.game.is_last_level() {
            "This was the last level in this colletion. Press Q to quit."
        } else {
            "Press any key to go to the next level."
        }
    }

    /// Fill the screen with the cached background image
    fn draw_background<S: glium::Surface>(&self, target: &mut S) {
        let vertices = texture::full_screen();
        let vb = glium::VertexBuffer::new(&self.display, &vertices).unwrap();

        let bg = self.background_texture.as_ref().unwrap();
        let uniforms = uniform! {tex: bg, matrix: IDENTITY};
        let program = &self.program;

        target.clear_color(0.0, 0.0, 0.0, 1.0); // Prevent artefacts when resizing the window

        target
            .draw(&vb, &NO_INDICES, program, &uniforms, &self.params)
            .unwrap();
    }

    fn draw_foreground<S: glium::Surface>(&self, target: &mut S) {
        let columns = self.columns as u32;
        let rows = self.rows as u32;

        let mut draw = |vs, tex| self.draw_quads(target, vs, tex, &self.program).unwrap();

        // Draw the crates
        let mut vertices = vec![];
        for sprite in &self.crates {
            vertices.extend(sprite.quad(columns, rows));
        }
        draw(vertices, &self.textures.crate_);

        // Draw the worker
        draw(self.worker.quad(columns, rows), &self.textures.worker);
    }

    fn statistics_text(&self) -> String {
        format!(
            "Level: {:>4}, Steps: {:>4}, Pushes: {:>4}",
            self.game.rank(),
            self.game.number_of_moves(),
            self.game.number_of_pushes()
        )
    }

    fn update_statistics_text(&mut self) {
        let text = self.statistics_text();
        // self.text_object_manager
        //     .set_text(self.stats_text_handle, &text);
    }

    fn draw_statistics_overlay<S: glium::Surface>(&mut self, target: &mut S) {
        let aspect_ratio = self.window_aspect_ratio();
        // self.text_object_manager
        //     .draw_text_objects(target, aspect_ratio);
    }

    /// Render the current level.
    fn render_level(&mut self) {
        self.generate_background_if_none();

        let mut target = self.display.draw();

        self.draw_background(&mut target);
        self.draw_foreground(&mut target);
        self.draw_statistics_overlay(&mut target);

        target.finish().unwrap();
    }

    fn generate_background_if_none(&mut self) {
        if self.background_texture.is_none() {
            self.generate_background();
        }
    }

    fn render_end_of_level(&mut self) {
        // TODO extract functions, reduce duplication with render_level()
        if self.background_texture.is_none() {
            self.generate_background();

            let width = self.window_size[0];
            let height = self.window_size[1];
            let texture = Texture2d::empty(&self.display, width, height).unwrap();

            {
                let mut target = texture.as_surface();
                self.draw_background(&mut target);
                self.draw_foreground(&mut target);

                // Display text overlay
                self.draw_end_of_level_overlay(&mut target);
            }

            self.background_texture = Some(texture);
            self.render_level();
        } else {
            let mut target = self.display.draw();
            self.draw_background(&mut target);
            target.finish().unwrap();
        }
    }

    pub fn render(&mut self) {
        match self.state {
            State::Level => {
                self.render_level();
                if !self.worker.is_animated() {
                    self.need_to_redraw = false;
                }
            }
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

fn set_animation_duration(queue_length: usize) {
    let new_duration = if queue_length > 60 {
        0.02_f32
    } else if queue_length > 20 {
        0.05_f32
    } else {
        0.08_f32
    };
    *sprite::ANIMATION_DURATION.lock().unwrap() = new_duration;
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
    pub fn handle_responses(&mut self, queue: &mut VecDeque<crate::backend::Event>) {
        const SKIP_FRAMES: u32 = 16;
        const QUEUE_LENGTH_THRESHOLD: usize = 100;

        let mut steps = 0;

        while let Some(response) = queue.pop_front() {
            set_animation_duration(queue.len());

            let is_move = self.handle_response(response);
            if is_move {
                self.update_statistics_text();
                steps = (steps + 1) % SKIP_FRAMES;
                if steps == 0 || queue.len() < QUEUE_LENGTH_THRESHOLD {
                    break;
                }
                // TODO this sort of works, but can we find a better way to skip animation
                // steps?
            }
        }
    }

    fn handle_response(&mut self, event: crate::backend::Event) -> bool {
        use crate::backend::Event::*;
        match event {
            LevelFinished(resp) if !self.level_solved() => {
                self.state = State::FinishAnimation;
                log_update_response(resp);
                self.need_to_redraw = true;
            }
            LevelFinished(_) => {}
            InitialLevelState {
                rank,
                columns,
                rows,
                background: _background,
                crates: _crates,
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
                self.need_to_redraw = true;
            }
            MoveWorker {
                from: _from,
                to,
                direction,
            } => {
                self.worker.move_to(to);
                self.worker.set_direction(direction);
                self.need_to_redraw = true;
                return true;
            }
            MoveCrate { id, to, .. } => {
                self.crates[id].move_to(to);
                self.need_to_redraw = true;
            }

            EndOfCollection => {
                self.is_last_level = true;
                self.need_to_redraw = true;
            }
            _ => {}
        }

        false
    }
}
