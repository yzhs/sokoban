#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        empty_enum,
        filter_map,
        if_not_else,
        invalid_upcast_comparisons,
        items_after_statements,
        mut_mut,
        nonminimal_bool,
        option_map_unwrap_or,
        option_map_unwrap_or_else,
        pub_enum_variant_names,
        shadow_same,
        single_match_else,
        string_add_assign,
        unicode_not_nfc,
        unseparated_literal_suffix,
        used_underscore_binding,
        wrong_pub_self_convention
    )
)]

// GUI
#[macro_use]
extern crate glium;
extern crate glium_text;
extern crate image;

// Logging
extern crate colog;
#[macro_use]
extern crate log;

extern crate clap; // Argument handling
#[macro_use]
extern crate lazy_static; // Mutable globals

extern crate sokoban_backend as backend;

mod gui;

use std::env;

use backend::{convert_savegames, print_collections_table, print_stats, Collection, Game, TITLE};

fn main() {
    use clap::{App, Arg};
    use gui::Gui;
    colog::init();

    let matches = App::new(TITLE)
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("collection")
                .help("The level collection to load during startup")
                .index(1),
        )
        .arg(
            Arg::with_name("list")
                .help("Print a list of available level sets")
                .short("l")
                .long("list"),
        )
        .arg(
            Arg::with_name("stats")
                .help("Print some statistics")
                .short("s")
                .long("stats"),
        )
        .arg(
            Arg::with_name("convert-savegames")
                .help("Load and store all savegames to convert them to the latest file format")
                .long("convert-savegames"),
        )
        .get_matches();

    if matches.is_present("convert-savegames") {
        convert_savegames();
        return;
    } else if matches.is_present("list") {
        print_collections_table();
        return;
    } else if matches.is_present("stats") {
        print_stats();
        return;
    }

    let collection_name = match matches.value_of("collection") {
        None | Some("") => {
            env::var("SOKOBAN_COLLECTION").unwrap_or_else(|_| "original".to_string())
        }
        Some(c) => c.to_string(),
    };

    // With WINIT_HIDPI_FACTOR > 1, the textures become blurred. As we do not have a good use for
    // the DPI factor, we may as well fix it at 1.
    env::set_var("WINIT_HIDPI_FACTOR", "1");

    let collection = Collection::parse(&collection_name).expect("Failed to load level set");
    let game = Game::new(collection);
    let gui = Gui::new(game);
    gui.main_loop();
}
