
use glium;
use glium::texture::Texture2d;
use glium::backend::Facade;
use image;

use backend::{ASSETS, Direction, Position};

pub struct Textures {
    pub crate_: Texture2d,
    pub floor: Texture2d,
    pub goal: Texture2d,
    pub wall: Texture2d,
    pub worker: Texture2d,
    pub transition_wall_empty_vertical: Texture2d,
    pub transition_wall_floor_vertical: Texture2d,
    pub transition_wall_empty_horizontal: Texture2d,
    pub transition_wall_floor_horizontal: Texture2d,
}

impl Textures {
    /// Load all textures.
    pub fn new(factory: &Facade) -> Self {
        let crate_ = load(factory, "crate");
        let floor = load(factory, "floor");
        let goal = load(factory, "goal");
        let wall = load(factory, "wall");
        let worker = load(factory, "worker");
        let transition_wall_empty_vertical = load(factory, "transition_wall_empty_vertical");
        let transition_wall_floor_vertical = load(factory, "transition_wall_floor_vertical");
        let transition_wall_empty_horizontal = load(factory, "transition_wall_empty_horizontal");
        let transition_wall_floor_horizontal = load(factory, "transition_wall_floor_horizontal");

        Textures {
            crate_,
            floor,
            goal,
            wall,
            worker,
            transition_wall_empty_vertical,
            transition_wall_floor_vertical,
            transition_wall_empty_horizontal,
            transition_wall_floor_horizontal,
        }
    }
}

/// Load an image from the assets directory and turn it into a `Texture2d`.
pub fn load(display: &Facade, name: &str) -> Texture2d {
    let mut path = ASSETS.join("images");
    path.push(name);
    path.set_extension("png");
    let image = image::open(path).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(),
                                                                   image_dimensions);
    Texture2d::new(display, image).unwrap()
}


#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

/// Pass through coordinates and texture coordinates.
pub const VERTEX_SHADER: &str = r#"
#version 140

in vec2 position;
in vec2 tex_coords;
out vec2 v_tex_coords;

void main() {
    v_tex_coords = tex_coords;
    gl_Position = vec4(position, 0.0, 1.0);
}
"#;


/// Render texture on triangles.
pub const FRAGMENT_SHADER: &str = r#"
#version 140

in vec2 v_tex_coords;
out vec4 color;

uniform sampler2D tex;

void main() {
    color = texture(tex, v_tex_coords);
}
"#;

#[derive(Clone, Copy, Debug)]
pub enum TileKind {
    Crate,
    Worker,
}


/// All tiles face left by default, so the worker has to turned by 90 degrees (clockwise) to face
/// up instead of left, etc.
fn direction_to_index(dir: Direction) -> usize {
    match dir {
        Direction::Left => 0,
        Direction::Down => 1,
        Direction::Right => 2,
        Direction::Up => 3,
    }
}

/// Create a vector of vertices consisting of two triangles which together form a square with the
/// given coordinates, together with texture coordinates to fill that square with a texture.
pub fn lrtp_to_vertices(mut left: f32,
                        mut right: f32,
                        mut top: f32,
                        mut bottom: f32,
                        dir: Direction,
                        aspect_ratio: f32)
                        -> Vec<Vertex> {

    if aspect_ratio < 1.0 {
        top *= aspect_ratio;
        bottom *= aspect_ratio;
    } else {
        left /= aspect_ratio;
        right /= aspect_ratio;
    }

    let tex = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];

    let rot = direction_to_index(dir);

    let a = Vertex {
        position: [left, top],
        tex_coords: tex[rot],
    };
    let b = Vertex {
        position: [left, bottom],
        tex_coords: tex[(rot + 1) % 4],
    };
    let c = Vertex {
        position: [right, bottom],
        tex_coords: tex[(rot + 2) % 4],
    };
    let d = Vertex {
        position: [right, top],
        tex_coords: tex[(rot + 3) % 4],
    };
    vec![a, b, c, c, d, a]
}

/// Create a bunch of vertices for rendering a textured square.
pub fn create_quad_vertices(pos: Position,
                            columns: u32,
                            rows: u32,
                            aspect_ratio: f32)
                            -> Vec<Vertex> {
    let left = 2.0 * pos.x as f32 / columns as f32 - 1.0;
    let right = left + 2.0 / columns as f32;
    let bottom = -2.0 * pos.y as f32 / rows as f32 + 1.0;
    let top = bottom - 2.0 / rows as f32;

    lrtp_to_vertices(left, right, top, bottom, Direction::Left, aspect_ratio)
}


/// Create a rectangle covering the entire viewport.
pub fn create_full_screen_quad() -> Vec<Vertex> {
    lrtp_to_vertices(-1.0, 1.0, -1.0, 1.0, Direction::Left, 1.0)
}

/// Create a centered rectangle with the right size to display the static parts of a level with
/// the correct aspect ratio.
pub fn create_background_quad(window_aspect_ratio: f32,
                              columns: usize,
                              rows: usize)
                              -> Vec<Vertex> {
    let aspect_ratio = columns as f32 / rows as f32 * window_aspect_ratio;
    if aspect_ratio < 1.0 {
        lrtp_to_vertices(-aspect_ratio, aspect_ratio, -1.0, 1.0, Direction::Left, 1.0)
    } else {
        lrtp_to_vertices(-1.0,
                         1.0,
                         -1.0 / aspect_ratio,
                         1.0 / aspect_ratio,
                         Direction::Left,
                         1.0)
    }
}

pub fn create_transition(pos: Position,
                         columns: u32,
                         rows: u32,
                         orientation: Direction)
                         -> Vec<Vertex> {
    let left;
    let right;
    let top;
    let bottom;
    match orientation {
        Direction::Left | Direction::Right => {
            left = (2.0 * pos.x as f32 - 0.25) / columns as f32 - 1.0;
            right = left + 0.5 / columns as f32;
            bottom = -2.0 * pos.y as f32 / rows as f32 + 1.0;
            top = bottom - 2.0 / rows as f32;
        }
        Direction::Up | Direction::Down => {
            left = 2.0 * pos.x as f32 / columns as f32 - 1.0;
            right = left + 2.0 / columns as f32;
            bottom = (-2.0 * pos.y as f32 + 0.25) / rows as f32 + 1.0;
            top = bottom - 0.5 / rows as f32;
        }
    }
    lrtp_to_vertices(left, right, top, bottom, Direction::Left, 1.0)

}
