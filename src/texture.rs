extern crate gfx_core;

use std::path::{PathBuf, Path};

use piston_window::*;
use gfx_graphics::{Texture, TextureSettings};

pub fn load<P, R, F>(factory: &mut F, name: &str, assets: P) -> Texture<R>
    where P: AsRef<Path>,
          R: gfx_core::Resources,
          F: gfx_core::Factory<R>
{
    let ts = TextureSettings::new();
    let mut path = PathBuf::new();
    path.push(assets.as_ref());
    path.push("images");
    path.push(name);
    path.set_extension("png");
    Texture::from_path(factory, &path, Flip::None, &ts).expect(&format!("Failed to load '{:?}'",
                                                                        path))
}
