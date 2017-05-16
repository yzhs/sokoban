extern crate sokoban;

use sokoban::*;

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
    let lvl = Level::parse(0, ORIGINAL_LEVEL_1).unwrap();
    let mut lvl = CurrentLevel::new(lvl);
    assert_eq!(lvl.height(), 11);
    assert_eq!(lvl.width(), 19);

    let moves = "ullluuuLUllDlldddrRRRRRRRRRRRRurD\
                     llllllllllllllulldRRRRRRRRRRRRRRR\
                     lllllllluuululldDDuulldddrRRRRRRRRRRRdrUluR\
                     lldlllllluuulLulDDDuulldddrRRRRRRRRRRRurD\
                     lllllllluuulluuulDDDDDuulldddrRRRRRRRRRRR\
                     llllllluuulluuurDDllddddrrruuuLLulDDDuulldddrRRRRRRRRRRdrUluR";
    for (i, mv) in moves.chars().map(char_to_direction).enumerate() {
        assert!(lvl.try_move(mv).is_ok(),
                "Move #{} failed:\n{}\n",
                i,
                lvl.level);
    }
    assert!(lvl.is_finished(), "\n{}\n", lvl.level);
}

#[test]
fn test_path_finding() {
    use self::Direction::*;
    let lvl = Level::parse(0, ORIGINAL_LEVEL_1).unwrap();
    let mut lvl = CurrentLevel::new(lvl);
    for (i, mv) in "ullluuuLUllDlldddrRRRRRRRRRRRRurD\
                        llllllllllllllulldRRRRRRRRRRRRRRR"
                .chars()
                .map(char_to_direction)
                .enumerate() {
        assert!(lvl.try_move(mv).is_ok(),
                "Move #{} failed:\n{}\n",
                i,
                lvl.level);
    }
    let pos = Position { x: 5, y: 4 };
    lvl.find_path(pos);
    assert_eq!(lvl.worker_position, pos);

    for (i, mv) in "DDuulldddr".chars().map(char_to_direction).enumerate() {
        assert!(lvl.try_move(mv).is_ok(),
                "Move #{} failed:\n{}\n",
                i,
                lvl.level);
    }

    let pos = lvl.worker_position;
    lvl.move_until(Right, false);
    assert_eq!(pos, lvl.worker_position);

    lvl.move_until(Right, true);
    for (i, mv) in "drUluR\
                        lldlllllluuulLulDDDuulldddrRRRRRRRRRRRurD\
                          lllllllluuulluuulDDDDDuulldddrRRRRRRRRRRR\
                        llllllluuulluuurDDllddddrrruuuLLulDDDuulldddrRRRRRRRRRRdrUluR"
                .chars()
                .map(char_to_direction)
                .enumerate() {
        assert!(lvl.try_move(mv).is_ok(),
                "Move #{} failed:\n{}\n",
                i,
                lvl.level);
    }

    assert!(lvl.is_finished());
}
