// GUI
#[macro_use]
extern crate glium;
extern crate glium_text_rusttype;
extern crate image;

// Logging
#[macro_use]
extern crate log;
extern crate colog;

extern crate ansi_term; // Colored output
extern crate clap; // Argument handling
extern crate natord; // Sort strings respecting numeric value, i.e. "9" before "10"
#[macro_use]
extern crate lazy_static; // Non-constant globals

extern crate sokoban_backend as backend;

mod gui;

use backend::*;

fn print_collections_table() {
    use ansi_term::Colour::{Blue, Green, White, Yellow};

    #[cfg(windows)]
    ansi_term::enable_ansi_support();

    println!(" {}               {}",
             Yellow.bold().paint("File name"),
             Yellow.bold().paint("Collection name"));
    println!("{0}{0}{0}{0}{0}", "----------------");

    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(ASSETS.join("levels"))
        .unwrap()
        .map(|x| x.unwrap().path().to_owned())
        .collect();
    paths.sort_by(|x, y| {
                      natord::compare(x.file_stem().unwrap().to_str().unwrap(),
                                      y.file_stem().unwrap().to_str().unwrap())
                  });

    for path in paths {
        if let Some(ext) = path.extension() {
            use std::ffi::OsStr;
            if ext == OsStr::new("lvl") || ext == OsStr::new("slc") {
                let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
                let collection = Collection::load(name).unwrap();

                let padded_short_name = format!("{:<24}", name);
                let padded_full_name = format!("{:<36}", collection.name);

                if collection.is_solved() {
                    println!(" {}{}{:>10} {}",
                             Green.paint(padded_short_name),
                             Green.bold().paint(padded_full_name),
                             "",
                             Green.paint("done"));
                } else {
                    let solved = if collection.number_of_solved_levels() == 0 {
                        White.paint("solved")
                    } else {
                        Blue.paint("solved")
                    };
                    println!(" {}{}{:>10} {}",
                             padded_short_name,
                             White.bold().paint(padded_full_name),
                             format!("{}/{}",
                                     collection.number_of_solved_levels(),
                                     collection.number_of_levels()),
                             solved);
                }
            }
        }
    }
}

fn main() {
    use clap::{App, Arg};
    use gui::{Gui, TITLE};
    colog::init();

    let matches = App::new(TITLE)
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("collection")
                 .help("The level collection to load during startup")
                 .index(1))
        .arg(Arg::with_name("list")
                 .help("Print a list of available level sets")
                 .short("l")
                 .long("list"))
        .get_matches();

    // Print a list of available collections
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
