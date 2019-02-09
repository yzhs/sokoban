use std::collections::{HashMap, VecDeque};

use crate::level::{Background, Level};
use crate::position::*;
use crate::util::*;

/// Dynamic part of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Foreground {
    None,
    Worker,
    Crate,
}

fn char_to_cell(chr: char) -> Option<(Background, Foreground)> {
    match chr {
        '#' => Some((Background::Wall, Foreground::None)),
        ' ' => Some((Background::Empty, Foreground::None)),
        '$' => Some((Background::Floor, Foreground::Crate)),
        '@' => Some((Background::Floor, Foreground::Worker)),
        '.' => Some((Background::Goal, Foreground::None)),
        '*' => Some((Background::Goal, Foreground::Crate)),
        '+' => Some((Background::Goal, Foreground::Worker)),
        _ => None,
    }
}

pub(crate) struct LevelBuilder {
    rank: usize,
    columns: usize,
    rows: usize,
    background: Vec<Background>,
    crates: HashMap<Position, usize>,
    worker_position: Position,
}

fn is_empty_or_comment(s: &str) -> bool {
    s.is_empty() || s.trim().starts_with(';')
}

impl LevelBuilder {
    pub fn new(rank: usize, level_string: &str) -> Result<Self, SokobanError> {
        let lines: Vec<_> = level_string
            .lines()
            .filter(|x| !is_empty_or_comment(x))
            .collect();
        let rows = lines.len();
        if rows == 0 {
            return Err(SokobanError::NoLevel(rank));
        }
        let columns = lines.iter().map(|x| x.len()).max().unwrap();
        if columns == 0 {
            return Err(SokobanError::NoLevel(rank));
        }

        let mut found_worker = false;
        let mut worker_position = Position { x: 0, y: 0 };
        let mut background = vec![Background::Empty; columns * rows];
        let mut crates = Vec::with_capacity(20);

        let mut goals_minus_crates = 0_i32;

        let mut found_level_description = false;
        for (y, line) in lines.iter().enumerate() {
            let mut inside = false;
            for (x, chr) in line.chars().enumerate() {
                let (bg, fg) = char_to_cell(chr).unwrap_or_else(|| {
                    panic!("Invalid character '{}' in line {}, column {}.", chr, y, x)
                });
                let index = y * columns + x;
                background[index] = bg;
                found_level_description = true;

                // Count goals still to be filled and make sure that there are exactly as many
                // goals as there are crates.
                if bg == Background::Goal && fg != Foreground::Crate {
                    goals_minus_crates += 1;
                } else if bg != Background::Goal && fg == Foreground::Crate {
                    goals_minus_crates -= 1;
                }
                if fg == Foreground::Crate {
                    crates.push(Position::new(x, y));
                }

                // Try to figure out whether a given cell is inside the walls.
                if !inside && bg.is_wall() {
                    inside = true;
                }

                if inside
                    && bg == Background::Empty
                    && index >= columns
                    && background[index - columns] != Background::Empty
                {
                    background[index] = Background::Floor;
                }

                // Find the initial worker position.
                if fg == Foreground::Worker {
                    if found_worker {
                        return Err(SokobanError::TwoWorkers(rank));
                    }
                    worker_position = Position::new(x, y);
                    found_worker = true;
                }
            }
        }
        if !found_level_description {
            return Err(SokobanError::NoLevel(rank));
        } else if !found_worker {
            return Err(SokobanError::NoWorker(rank));
        } else if goals_minus_crates != 0 {
            return Err(SokobanError::CratesGoalsMismatch(rank, goals_minus_crates));
        }

        let swap = |(a, b)| (b, a);
        let crates = crates.into_iter().enumerate().map(swap).collect();
        Ok(Self {
            rank,
            columns,
            rows,
            background,
            crates,
            worker_position,
        })
    }

    pub fn build(mut self) -> Level {
        self.correct_outside_cells();
        Level {
            rank: self.rank,
            columns: self.columns,
            rows: self.rows,
            background: self.background,
            crates: self.crates,
            worker_position: self.worker_position,
        }
    }

    /// Fix the mistakes of the heuristic used in `new()` for detecting which cells are on the
    /// inside.
    fn correct_outside_cells(&mut self) {
        let columns = self.columns;

        let mut queue = VecDeque::new();
        let mut visited = vec![false; self.background.len()];
        visited[self.worker_position.to_index(columns)] = true;

        let mut inside = visited.clone();

        queue.push_back(self.worker_position);

        for crate_pos in self.crates.keys() {
            visited[crate_pos.to_index(columns)] = true;
            queue.push_back(*crate_pos);
        }

        for (i, &bg) in self.background.iter().enumerate() {
            match bg {
                Background::Wall => visited[i] = true,
                Background::Goal if !visited[i] => {
                    inside[i] = true;
                    visited[i] = true;
                    queue.push_back(Position::from_index(i, columns));
                }
                _ => (),
            }
        }

        // Flood fill from all positions added above
        while let Some(pos) = queue.pop_front() {
            use crate::Direction::*;
            let i = pos.to_index(columns);
            if let Background::Wall = self.background[i] {
                continue;
            } else {
                inside[i] = true;
            }
            for n in [Up, Down, Left, Right].iter().map(|&x| pos.neighbour(x)) {
                // The outermost rows and columns may only contain empty space and walls, so
                // n has to bee within bounds.
                let j = n.to_index(columns);
                if !visited[j] {
                    visited[j] = true;
                    queue.push_back(n);
                }
            }
        }

        for (i, bg) in self.background.iter_mut().enumerate() {
            if !inside[i] && *bg == Background::Floor {
                *bg = Background::Empty;
            }
        }
    }
}
// }}}
