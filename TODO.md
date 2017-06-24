* Clean up UI code
* Add a background image?
* Use different kinds of wall tiles to render corners differently from pieces of
  wall surrounded by other walls

* Show statistics for each level when using --list

* Separate front and back end into threads
* Communication between front and back end using channels

* Write more tests
  - Module tests in the back end
  - Send Commands to the back end and make sure these lead to the correct
    Responses
  - Test loading and saving games
* Configuration file
  - Allow users to reconfigure key bindings
  - Automatically save when closing?
* Run length encoding in solution format?
  - Or maybe encode non-push moves by just specifying the destination
    coordinates?
* Replay saved game
  - Show all steps
  - Show only the position before and after moving a crate
  - Show only ever n-th state?
* Support different kind of levels or level formats
  - Run length encoding as an addition to the current format
  - One directory per collection with one file per level?
  - Compressed level files
  - Multiple workers
  - Narrow walls, i.e. walls between two adjacent tiles

* Add a level editor?
