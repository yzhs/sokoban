extern crate gfx_core;

use std::path::PathBuf;

use piston_window::*;
use gfx_graphics::{Texture, TextureSettings};

use sokoban::*;


pub fn load<R, F>(factory: &mut F, name: &str) -> Texture<R>
    where R: gfx_core::Resources,
          F: gfx_core::Factory<R>
{
    let ts = TextureSettings::new();
    let mut path = PathBuf::new();
    path.push(ASSETS_PATH);
    path.push("images");
    path.push(name);
    path.set_extension("png");
    Texture::from_path(factory, &path, Flip::None, &ts).expect(&format!("Failed to load '{:?}'",
                                                                        path))
}
