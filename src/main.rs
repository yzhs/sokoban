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

// GUI
#[macro_use]
extern crate glium;

// Logging
#[macro_use]
extern crate log;

// Argument handling
#[macro_use]
extern crate lazy_static; // Mutable globals

use backend::{Command, LevelManagement};
use glium::glutin::{
    self, dpi,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
};

use sokoban_backend as backend;

mod gui;
use crate::gui::inputstate::*;

use std::{collections::VecDeque, env, sync::mpsc::channel};

use crate::backend::{
    convert_savegames, print_collections_table, print_stats, Collection, Game, TITLE,
};

fn main() {
    use crate::gui::Gui;
    use clap::{Arg, ArgAction};
    colog::init();

    let matches = clap::Command::new(TITLE)
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("collection")
                .value_name("collection")
                .help("The level collection to load during startup")
                .index(1),
        )
        .arg(
            Arg::new("list")
                .help("Print a list of available level sets")
                .short('l')
                .long("list")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("stats")
                .help("Print some statistics")
                .short('s')
                .long("stats")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("convert-savegames")
                .help("Load and store all savegames to convert them to the latest file format")
                .long("convert-savegames")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    if matches.get_flag("convert-savegames") {
        convert_savegames();
        return;
    } else if matches.get_flag("list") {
        print_collections_table();
        return;
    } else if matches.get_flag("stats") {
        print_stats();
        return;
    }

    let collection_name = match matches.get_one::<&str>("collection") {
        None | Some(&"") => {
            env::var("SOKOBAN_COLLECTION").unwrap_or_else(|_| "original".to_string())
        }
        Some(c) => c.to_string(),
    };

    // With WINIT_HIDPI_FACTOR > 1, the textures become blurred. As we do not have a good use for
    // the DPI factor, we may as well fix it at 1.
    env::set_var("WINIT_HIDPI_FACTOR", "1");

    let collection = Collection::parse(&collection_name).expect("Failed to load level set");
    let game = Game::new(collection);
    let event_loop = glutin::event_loop::EventLoop::new();
    let mut gui = Gui::new(game, &event_loop);

    let mut queue = VecDeque::new();
    let mut input_state: InputState = Default::default();
    let (sender, receiver) = channel();

    gui.game.listen_to(receiver);

    use glium::glutin::event::ElementState::*;

    event_loop.run(move |ev: Event<()>, window, control_flow| match ev {
        Event::WindowEvent { event, .. } => {
            let mut cmd = Command::Nothing;

            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Q),
                            ..
                        },
                    ..
                } => return,

                WindowEvent::KeyboardInput {
                    input: KeyboardInput { state: Pressed, .. },
                    ..
                }
                | WindowEvent::MouseInput { .. }
                    if gui.level_solved() =>
                {
                    cmd = Command::LevelManagement(LevelManagement::NextLevel)
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: Pressed,
                            virtual_keycode: Some(key),
                            modifiers,
                            ..
                        },
                    ..
                } => cmd = input_state.press_to_command(key, modifiers),

                WindowEvent::CursorMoved {
                    position: dpi::PhysicalPosition { x, y },
                    ..
                } => input_state.cursor_position = [x, y],
                WindowEvent::MouseInput {
                    state: Released,
                    button: btn,
                    modifiers,
                    ..
                } => cmd = gui.click_to_command(btn, modifiers, &mut input_state),

                WindowEvent::Resized(new_size) => {
                    gui.window_size = [new_size.width, new_size.height];
                    gui.background_texture = None;
                    gui.need_to_redraw = true;
                }

                //WindowEvent::Refresh => gui.need_to_redraw = true,
                _ => (),
            }

            sender.send(cmd).unwrap();

            gui.game.execute();
        }

        Event::RedrawRequested(_) => {
            gui.render();

            // We need to move the events from the channel into a deque so we can figure out how
            // many events are left. This information is needed to adjust the animation speed if a
            // large number of events is pending.
            gui.events
                .try_iter()
                .for_each(|event| queue.push_back(event));
            gui.handle_responses(&mut queue);
        }

        Event::Resumed
        | Event::Suspended { .. }
        | Event::DeviceEvent { .. }
        | Event::NewEvents(_)
        | Event::UserEvent(_)
        | Event::MainEventsCleared
        | Event::RedrawEventsCleared => {
            gui.render();

            // We need to move the events from the channel into a deque so we can figure out how
            // many events are left. This information is needed to adjust the animation speed if a
            // large number of events is pending.
            gui.events
                .try_iter()
                .for_each(|event| queue.push_back(event));
            gui.handle_responses(&mut queue);
        }

        Event::LoopDestroyed => (),
    });
}
