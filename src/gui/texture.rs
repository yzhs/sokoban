use glium;
use glium::backend::Facade;
use glium::texture::Texture2d;
use image;

use backend::{Direction, Position, ASSETS};

pub struct Textures {
    pub crate_: Texture2d,
    pub floor: Texture2d,
    pub goal: Texture2d,
    pub wall: Texture2d,
    pub worker: Texture2d,
}

impl Textures {
    /// Load all textures.
    pub fn new(factory: &Facade) -> Self {
        let crate_ = load(factory, "crate");
        let floor = load(factory, "floor");
        let goal = load(factory, "goal");
        let wall = load(factory, "wall");
        let worker = load(factory, "worker");

        Textures {
            crate_,
            floor,
            goal,
            wall,
            worker,
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
    let image =
        glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);
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

uniform mat4 matrix;

void main() {
    v_tex_coords = tex_coords;
    gl_Position = matrix * vec4(position, 0.0, 1.0);
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

/// Darken the screen
pub const DARKEN_SHADER: &str = r#"
#version 140

in vec2 v_tex_coords;
out vec4 color;

void main() {
    color = vec4(0.0, 0.0, 0.0, 0.7);
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
pub fn lrtp_to_vertices(
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    dir: Direction,
) -> Vec<Vertex> {
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
pub fn quad(pos: Position, columns: u32, rows: u32) -> Vec<Vertex> {
    let left = 2.0 * pos.x as f32 / columns as f32 - 1.0;
    let right = left + 2.0 / columns as f32;
    let bottom = -2.0 * pos.y as f32 / rows as f32 + 1.0;
    let top = bottom - 2.0 / rows as f32;

    lrtp_to_vertices(left, right, top, bottom, Direction::Left)
}

/// Create a rectangle covering the entire viewport.
pub fn full_screen() -> Vec<Vertex> {
    lrtp_to_vertices(-1.0, 1.0, -1.0, 1.0, Direction::Left)
}
