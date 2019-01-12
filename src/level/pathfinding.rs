use std::collections::VecDeque;

use crate::direction::*;
use crate::event::Event;
use crate::level::*;
use crate::move_::Move;
use crate::position::*;

pub struct Path {
    pub start: Position,
    pub steps: Vec<Move>,
}

impl Level {
    /// Try to find a shortest path from the workers current position to `to` and execute it if one
    /// exists. Otherwise, emit `Event::NoPathFound`.
    pub fn find_path(&mut self, to: Position) -> Option<Path> {
        let columns = self.columns();
        let rows = self.rows();

        if self.worker_position == to || !self.is_empty(to) {
            return None;
        }

        let mut distances = vec![::std::usize::MAX; columns * rows];
        distances[self.index(to)] = 0;

        let mut path_exists = false;
        let mut queue = VecDeque::with_capacity(500);
        queue.push_back(to);

        while let Some(pos) = queue.pop_front() {
            if pos == self.worker_position {
                path_exists = true;
                break;
            }

            // Is there a neighbour of pos to which we do not currently know the shortest path?
            for neighbour in self.empty_neighbours(pos) {
                let new_dist = distances[self.index(pos)] + 1;
                let neighbour_dist = &mut distances[self.index(neighbour)];

                if *neighbour_dist > new_dist {
                    *neighbour_dist = new_dist;
                    queue.push_back(neighbour);
                }
            }
        }

        if !path_exists {
            self.notify(&Event::NoPathFound);
            return None;
        }

        let mut path = Path {
            start: self.worker_position,
            steps: vec![],
        };

        // Move worker along the path
        let mut pos = self.worker_position;
        while pos != to {
            for neighbour in self.empty_neighbours(pos) {
                if distances[self.index(neighbour)] < distances[self.index(pos)] {
                    let dir = direction(pos, neighbour).unwrap();
                    pos = neighbour;
                    path.steps.push(Move {
                        direction: dir,
                        moves_crate: false,
                    });
                }
            }
        }

        Some(path)
    }

    pub fn find_path_with_crate(&mut self, from: Position, to: Position) -> Option<Path> {
        None
    }

    /// Follow the given path, if any.
    pub fn follow_path(&mut self, path: Option<Path>) {
        if let Some(path) = path {
            assert_eq!(self.worker_position, path.start);
            for Move { direction, .. } in path.steps {
                let is_ok = self.try_move(direction).is_ok();
                assert!(is_ok);
            }
        }
    }
}
