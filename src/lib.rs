#![feature(try_from)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(quickcheck_macros))]
#![warn(
    clippy::empty_enum,
    clippy::filter_map,
    clippy::if_not_else,
    clippy::invalid_upcast_comparisons,
    clippy::items_after_statements,
    clippy::mut_mut,
    clippy::nonminimal_bool,
    clippy::option_map_unwrap_or,
    clippy::option_map_unwrap_or_else,
    clippy::pub_enum_variant_names,
    clippy::shadow_same,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::unicode_not_nfc,
    clippy::unseparated_literal_suffix,
    clippy::used_underscore_binding,
    clippy::wrong_pub_self_convention
)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

mod collection;
mod command;
mod current_level;
mod direction;
mod event;
mod game;
mod level;
mod macros;
mod move_;
mod position;
pub mod save;
mod undo;
mod util;

use std::fs;
use std::path::PathBuf;

use ansi_term::Colour::{Blue, Green, White, Yellow};

pub use crate::collection::*;
pub use crate::command::*;
pub use crate::current_level::*;
pub use crate::direction::*;
pub use crate::event::*;
pub use crate::game::*;
pub use crate::level::*;
pub use crate::macros::*;
pub use crate::move_::*;
pub use crate::position::*;
use crate::save::CollectionState;
pub use crate::util::*;

fn file_stem(p: &PathBuf) -> &str {
    p.file_stem().unwrap().to_str().unwrap()
}

pub fn convert_savegames() {
    use std::ffi::OsStr;

    for entry in fs::read_dir(DATA_DIR.as_path()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("json")) {
            let collection_name = file_stem(&path);
            let mut state = save::CollectionState::load(collection_name);
            state.save(collection_name).unwrap();
        }
    }
}

struct CollectionStats {
    pub short_name: String,
    pub name: String,
    pub total_levels: usize,
    pub solved_levels: usize,
}

impl CollectionStats {
    fn solved(&self) -> bool {
        self.total_levels == self.solved_levels
    }
    fn started(&self) -> bool {
        self.solved_levels > 0
    }
}

fn gather_stats() -> Vec<CollectionStats> {
    // Find all level set files
    let mut paths: Vec<PathBuf> = fs::read_dir(ASSETS.join("levels"))
        .unwrap()
        .map(|x| x.unwrap().path().to_owned())
        .collect();
    paths.sort_by(|x, y| ::natord::compare(file_stem(x), file_stem(y)));

    let mut result = vec![];

    for path in paths {
        if let Some(ext) = path.extension() {
            use std::ffi::OsStr;
            if ext == OsStr::new("lvl") || ext == OsStr::new("slc") {
                let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
                let collection = Collection::parse_metadata(name).unwrap();
                let state = CollectionState::load(collection.short_name());

                result.push(CollectionStats {
                    short_name: name.to_string(),
                    name: collection.name().to_string(),
                    total_levels: collection.number_of_levels(),
                    solved_levels: state.number_of_solved_levels(),
                });
            }
        }
    }

    result
}

pub fn print_collections_table() {
    let stats = gather_stats();

    println!(
        " {}               {}",
        Yellow.bold().paint("File name"),
        Yellow.bold().paint("Collection name")
    );
    println!("--------------------------------------------------------------------------------");

    for collection in stats {
        let padded_short_name = format!("{:<24}", collection.short_name);
        let padded_full_name = format!("{:<36}", collection.name);

        if collection.solved() {
            println!(
                " {}{}           {}",
                Green.paint(padded_short_name),
                Green.bold().paint(padded_full_name),
                Green.paint("done")
            );
        } else {
            let solved = if collection.started() {
                Blue.paint("solved")
            } else {
                White.paint("solved")
            };
            println!(
                " {}{}{:>10} {}",
                padded_short_name,
                White.bold().paint(padded_full_name),
                format!("{}/{}", collection.solved_levels, collection.total_levels),
                solved
            );
        }
    }
}

pub fn print_stats() {
    let stats = gather_stats();

    let num_collections = stats.len();
    let num_levels: usize = stats.iter().map(|x| x.total_levels).sum();

    let finished_collections = stats.iter().filter(|x| x.solved()).count();
    let finished_levels: usize = stats.iter().map(|x| x.solved_levels).sum();

    let collections_started = stats.iter().filter(|x| x.started() && !x.solved()).count();

    println!(
        "{}",
        Yellow.bold().paint("          Collections     Levels")
    );
    println!("------------------------------------");
    println!("Total    {:>11} {:>11}", num_collections, num_levels);
    println!(
        "Finished {:>11} {:>11}",
        finished_collections, finished_levels
    );
    println!("Started  {:>11}", collections_started);
}
