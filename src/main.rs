#![cfg_attr(
    clippy,
    warn(
        empty_enum, filter_map, if_not_else, invalid_upcast_comparisons, items_after_statements,
        mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        pub_enum_variant_names, shadow_same, single_match_else, string_add_assign, unicode_not_nfc,
        unseparated_literal_suffix, used_underscore_binding, wrong_pub_self_convention
    )
)]

// GUI
#[macro_use]
extern crate glium;
extern crate glium_text_rusttype;
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

use backend::{convert_savegames, print_collections_table, TITLE};

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
    }

    let collection = match matches.value_of("collection") {
        None | Some("") => {
            std::env::var("SOKOBAN_COLLECTION").unwrap_or_else(|_| "original".to_string())
        }
        Some(c) => c.to_string(),
    };

    let gui = Gui::new(&collection);
    gui.main_loop();
}
