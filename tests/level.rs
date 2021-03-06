use sokoban_backend as backend;

use crate::backend::*;

const ORIGINAL_LEVEL_1: &str = r#"
    #####
    #   #
    #$  #
  ###  $##
  #  $ $ #
### # ## #   ######
#   # ## #####  ..#
# $  $          ..#
##### ### #@##  ..#
    #     #########
    #######
"#;

fn char_to_direction(c: char) -> Direction {
    use self::Direction::*;
    match c {
        'l' | 'L' => Left,
        'r' | 'R' => Right,
        'u' | 'U' => Up,
        'd' | 'D' => Down,
        _ => panic!("Invalid character"),
    }
}

#[test]
fn test_simple_moves() {
    let mut lvl: CurrentLevel = Level::parse(0, ORIGINAL_LEVEL_1).unwrap().into();
    assert_eq!(lvl.rows(), 11);
    assert_eq!(lvl.columns(), 19);

    let moves = "ullluuuLUllDlldddrRRRRRRRRRRRRurD\
                 llllllllllllllulldRRRRRRRRRRRRRRR\
                 lllllllluuululldDDuulldddrRRRRRRRRRRRdrUluR\
                 lldlllllluuulLulDDDuulldddrRRRRRRRRRRRurD\
                 lllllllluuulluuulDDDDDuulldddrRRRRRRRRRRR\
                 llllllluuulluuurDDllddddrrruuuLLulDDDuulldddrRRRRRRRRRRdrUluR";
    for (i, mv) in moves.chars().map(char_to_direction).enumerate() {
        assert!(
            !&lvl.try_move(mv).is_err(),
            "Move #{} failed:\n{}\n",
            i,
            lvl
        );
    }
    assert!(lvl.is_finished(), "\n{}\n", lvl);
}

#[test]
fn test_path_finding() {
    use self::Direction::*;
    let mut lvl: CurrentLevel = Level::parse(0, ORIGINAL_LEVEL_1).unwrap().into();
    for (i, mv) in "ullluuuLUllDlldddrRRRRRRRRRRRRurD\
                    llllllllllllllulldRRRRRRRRRRRRRRR"
        .chars()
        .map(char_to_direction)
        .enumerate()
    {
        assert!(
            !&lvl.try_move(mv).is_err(),
            "Move #{} failed:\n{}\n",
            i,
            lvl
        );
    }
    let pos = Position { x: 5, y: 4 };
    let path = lvl.find_path(pos).unwrap();
    lvl.follow_path(path);
    assert_eq!(lvl.worker_position(), pos);

    for (i, mv) in "DDuulldddr".chars().map(char_to_direction).enumerate() {
        assert!(
            !&lvl.try_move(mv).is_err(),
            "Move #{} failed:\n{}\n",
            i,
            lvl
        );
    }

    let pos = lvl.worker_position();
    let _ = lvl.move_as_far_as_possible(Right, false);
    assert_eq!(pos, lvl.worker_position());

    let _ = lvl.move_as_far_as_possible(Right, true);
    for (i, mv) in "drUluR\
                    lldlllllluuulLulDDDuulldddrRRRRRRRRRRRurD\
                    lllllllluuulluuulDDDDDuulldddrRRRRRRRRRRR\
                    llllllluuulluuurDDllddddrrruuuLLulDDDuulldddrRRRRRRRRRRdrUluR"
        .chars()
        .map(char_to_direction)
        .enumerate()
    {
        assert!(
            !&lvl.try_move(mv).is_err(),
            "Move #{} failed:\n{}\n",
            i,
            lvl
        );
    }

    assert!(lvl.is_finished());
}
