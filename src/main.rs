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

use backend::{print_collections_table, TITLE};

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
        .get_matches();

    if matches.is_present("list") {
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
