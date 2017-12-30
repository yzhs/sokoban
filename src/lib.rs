#![feature(ascii_ctype, try_from)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(quickcheck_macros))]

#[macro_use]
extern crate log;
extern crate colog;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate rmp_serde; // MessagePack

extern crate find_folder;
#[macro_use]
extern crate lazy_static;
extern crate app_dirs;

extern crate quick_xml; // XML parser
extern crate ansi_term; // Colored output
extern crate natord; // Sort strings respecting numeric value, i.e. "9" before "10"

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

pub use collection::*;
pub use command::*;
pub use game::*;
pub use direction::*;
pub use level::*;
pub use macros::*;
pub use move_::*;
pub use position::*;
pub use util::*;
