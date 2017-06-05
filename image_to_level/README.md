# image_to_level

Convert a bitmap image into a Sokoban level in the usual format, i.e. ' ' for
empty space, '#' for wall, '$' for crates not on a goal, '@' for the worker (if
not on a goal'), '.' for empty goals, '+' for the worker standing on a goal, and
'*' for a goal containing a crate.

It reads a given image file as follows:
* Treat the first row as a key where the pixels are (in order) the colours for
  background, walls, floor, worker not on a goal, crates on a goal, crates not
  on a goal, an empty goal, and worker on a goal.
* Create a string representation of the level as the remaining rows of the image
  area read. In particular, the first row is skipped completely.
