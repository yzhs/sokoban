extern crate piston;
extern crate piston_window;
extern crate graphics;
extern crate gfx_graphics;
extern crate gfx_core;

use std::collections::HashMap;
use std::path::PathBuf;

use piston_window::*;
use gfx_graphics::{Texture, TextureSettings};

use sokoban::*;


fn load_texture<R, F>(factory: &mut F, name: &str) -> Texture<R>
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

pub fn load_backgrounds<R, F>(factory: &mut F) -> HashMap<Background, Texture<R>>
    where R: gfx_core::Resources,
          F: gfx_core::Factory<R>
{
    [(Background::Empty, "empty"),
     (Background::Wall, "wall"),
     (Background::Floor, "floor"),
     (Background::Goal, "goal")]
            .into_iter()
            .map(|&(x, path)| (x, load_texture(factory, path)))
            .collect()
}

pub fn load_foregrounds<R, F>(factory: &mut F) -> HashMap<Foreground, Texture<R>>
    where R: gfx_core::Resources,
          F: gfx_core::Factory<R>
{
    [(Foreground::Worker, "worker"), (Foreground::Crate, "crate")]
        .into_iter()
        .map(|&(x, path)| (x, load_texture(factory, path)))
        .collect()
}
