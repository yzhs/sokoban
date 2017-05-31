# Sokoban
This is a Sokoban clone written in [Rust](https://rust-lang.org) using the
[Glium](https://github.com/tomaka/glium) as a graphics back-end.

## Installation
Download Sokoban:
```sh
git clone https://github.com/yzhs/sokoban
cd sokoban
```
To compile the game, you will need a recent nightly version of Rust. Assuming
you are using nightly by default, you can run the game using `cargo run
--release`.

## Controls
Like in the original game, you move around using the arrow keys. There are also
some additional controls for convenience:

* Pressing `Shift` and an arrow key is the equivalent of hitting that arrow key
  repeatedly until the worker has stopped moving, i.e. go as far as possible in
  the given direction.
* `Ctrl` and an arrow key moves as far as possible in the given direction
  *without moving a crate*.
* `U` or `Ctrl+Z` undo the last move.
* `Shift+U` or `Ctrl+Shift+Z` redo one move.
* `Escape` resets the current level.
* `P` loads the previous level.
* `N` goes to the next level, but only if you have (now or in a previous
  session) solved the current level.
* `Q` exits the game.
