use sokoban_backend as backend;

use std::sync::mpsc::channel;

use crate::backend::{
    Collection, Command, Direction, Game, Position,
};

fn main() {
    colog::init();

    let collection_name = "microban_1";

    let collection = Collection::parse(&collection_name).expect("Failed to load level set");
    let mut game = Game::new(collection);

    let (sender, receiver) = channel();
    game.listen_to(receiver);

    sender.send(Command::Move(Direction::Down)).unwrap();
    sender.send(Command::Move(Direction::Left)).unwrap();
    sender.send(Command::Move(Direction::Up)).unwrap();

    let from = Position { x: 1, y: 2 };
    let to = Position { x: 3, y: 3 };
    let cmd = Command::MoveCrateToTarget { from, to };
    sender.send(cmd).unwrap();
    game.execute();

    let to = Position { x: 1, y: 1 };
    let cmd = Command::MoveCrateToTarget { from, to };
    sender.send(cmd).unwrap();
    game.execute();

    let from = Position { x: 3, y: 4 };
    let cmd = Command::MoveCrateToTarget { from, to };
    sender.send(cmd).unwrap();
    game.execute();
}
