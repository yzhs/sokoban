#![feature(ascii_ctype, try_from)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(quickcheck_macros))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        empty_enum, filter_map, if_not_else, invalid_upcast_comparisons, items_after_statements,
        mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        pub_enum_variant_names, shadow_same, single_match_else, string_add_assign, unicode_not_nfc,
        unseparated_literal_suffix, used_underscore_binding, wrong_pub_self_convention
    )
)]

/// Colored output
extern crate colog;
#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;

extern crate serde;
extern crate serde_cbor;
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
use save::CollectionState;
pub use util::*;

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
                let collection = Collection::parse_metadata(name).unwrap();
                let state = CollectionState::load(collection.short_name());

                let padded_short_name = format!("{:<24}", name);
                let padded_full_name = format!("{:<36}", collection.name());

                if state.collection_solved {
                    println!(
                        " {}{}           {}",
                        Green.paint(padded_short_name),
                        Green.bold().paint(padded_full_name),
                        Green.paint("done")
                    );
                } else {
                    let num_solved = state.number_of_solved_levels();
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
