#![feature(ascii_ctype, try_from)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(quickcheck_macros))]

/// Colored output
extern crate colog;
#[macro_use]
extern crate log;

/// MessagePack
extern crate rmp_serde;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate app_dirs;
extern crate find_folder;
#[macro_use]
extern crate lazy_static;

extern crate ansi_term;
/// Sort strings respecting numeric value, i.e. "9" before "10"
extern crate natord;
/// XML parser
extern crate quick_xml;

#[cfg(test)]
extern crate quickcheck;

mod collection;
mod command;
mod direction;
mod game;
mod level;
mod macros;
mod move_;
mod position;
pub mod save;
mod util;

use std::fs;
use std::path::PathBuf;

pub use collection::*;
pub use command::*;
pub use direction::*;
pub use game::*;
pub use level::*;
pub use macros::*;
pub use move_::*;
pub use position::*;
pub use util::*;

fn file_stem(p: &::std::path::PathBuf) -> &str {
    p.file_stem().unwrap().to_str().unwrap()
}

pub fn print_collections_table() {
    use ansi_term::Colour::{Blue, Green, White, Yellow};

    println!(
        " {}               {}",
        Yellow.bold().paint("File name"),
        Yellow.bold().paint("Collection name")
    );
    println!("{0}{0}{0}{0}{0}", "----------------");

    // Find all level set files
    let mut paths: Vec<PathBuf> = fs::read_dir(ASSETS.join("levels"))
        .unwrap()
        .map(|x| x.unwrap().path().to_owned())
        .collect();
    paths.sort_by(|x, y| ::natord::compare(file_stem(x), file_stem(y)));

    for path in paths {
        if let Some(ext) = path.extension() {
            use std::ffi::OsStr;
            if ext == OsStr::new("lvl") || ext == OsStr::new("slc") {
                let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
                let collection = Collection::parse(name, false).unwrap();

                let padded_short_name = format!("{:<24}", name);
                let padded_full_name = format!("{:<36}", collection.name);

                if collection.is_solved() {
                    println!(
                        " {}{}{:>10} {}",
                        Green.paint(padded_short_name),
                        Green.bold().paint(padded_full_name),
                        "",
                        Green.paint("done")
                    );
                } else {
                    let num_solved = collection.number_of_solved_levels();
                    let solved = if num_solved == 0 {
                        White.paint("solved")
                    } else {
                        Blue.paint("solved")
                    };
                    println!(
                        " {}{}{:>10} {}",
                        padded_short_name,
                        White.bold().paint(padded_full_name),
                        format!("{}/{}", num_solved, collection.number_of_levels()),
                        solved
                    );
                }
            }
        }
    }
}
