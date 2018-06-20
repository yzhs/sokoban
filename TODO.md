# Bugs
* Sokoban crashes when switching to a different virtual screen before start-up
  is complete. It does not, however, crash when being moved to a different
  virtual screen.

# Code and architecture improvements
* Clean up UI code

* Write more tests
  - Module tests in the back end
  - Send Commands to the back end and make sure these lead to the correct
    Responses
  - Test loading and saving games

* Separate front and back end into threads

* Communication between front and back end using channels

# Features
* Use the `atomicwrites` crate for replacing existing saves

* Add a background image?

* Use different kinds of wall tiles to render corners differently from pieces of
  wall surrounded by other walls

* Add menu to select collections at runtime

* Add menu to select a specific level in a collection at runtime?
  - Render the levels in thumbnail format?

* Configuration file
  - Allow users to reconfigure key bindings
  - Automatically save when closing?

* Replay saved game
  - Show all steps
  - Show only the position before and after moving a crate
  - Show only every n-th state?

* Run length encoding in solution format?
  - Or maybe encode non-push moves by just specifying the destination
    coordinates?

* Support different kinds of levels or level formats
  - Run length encoding as an addition to the current format
  - One directory per collection with one file per level?
  - Compressed level files

* Different game modes
  - Multiple workers
  - Narrow walls, i.e. walls between two adjacent tiles
  - Numbered crates that have to be put into numbered goals?
  - Pull mode, i.e. the worker can only pull crates

    This can be used as an alternative approach to solving normal levels by
    switching the roles of crates and goals (and possibly moving the worker to a
    different position). That way, the normal level can be solved by reversing
    the solution of the pull mode level.

* Add a level editor?
