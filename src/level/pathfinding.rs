use std::collections::{HashSet, VecDeque};

use crate::direction::*;
use crate::event::Event;
use crate::level::*;
use crate::level::graph::Graph;
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

    /// Follow the given path, if any.
    pub fn follow_path(&mut self, path: Path) {
        assert_eq!(self.worker_position, path.start);
        for Move { direction, .. } in path.steps {
            let is_ok = self.try_move(direction).is_ok();
            assert!(is_ok);
        }
    }

    /// Try to find a way to move the crate at `from` to `to`.
    pub fn find_path_with_crate(&self, from: Position, to: Position) -> Option<Path> {
        self.is_valid_for_path_with_crate(from, to)?;

        let graph = self.build_graph(from);
        self.visualise_graph(&graph);
        graph.find_crate_path(from, to)
    }

    fn move_worker_into_position(&mut self, crate_position: Position, r#move: &Move) -> Option<()> {
        let worker_pos = crate_position.neighbour(r#move.direction.reverse());
        info!("Moving worker to {:?}", worker_pos);
        let path = self.find_path(worker_pos)?;
        self.follow_path(path);
        Some(())
    }

    pub fn push_crate_along_path(&mut self, crate_path: Path) -> Option<()> {
        info!("Starting from {:?}", crate_path.start);
        assert!(!crate_path.steps.is_empty());


        self.move_worker_into_position(crate_path.start, &crate_path.steps[0])?;
        self.try_move(crate_path.steps[0].direction).ok().unwrap();

        for i in 1..crate_path.steps.len() {
            let crate_position = self.worker_position.neighbour(self.worker_direction());
            self.move_worker_into_position(crate_position, &crate_path.steps[i])?;
            self.try_move(crate_path.steps[i].direction).ok().unwrap();
        }

        Some(())
    }

    /// Create a graph of cells a crate `starting_from` can be moved to.
    fn build_graph(&self, starting_from: Position) -> Graph<Position> {
        let mut neighbours: HashMap<Position, Vec<_>> = HashMap::new();

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(starting_from);

        while let Some(pos) = queue.pop_front() {
            if visited.contains(&pos) {
                continue;
            }
            visited.insert(pos);
            neighbours.entry(pos).or_default();

            for neighbour in self.empty_neighbours(pos) {
                let dir = direction(neighbour, pos).unwrap();
                let opposite_neighbour = pos.neighbour(dir);

                if !self.is_empty(opposite_neighbour) && opposite_neighbour != starting_from {
                    continue;
                }

                queue.push_back(neighbour);
                neighbours.get_mut(&pos).unwrap().push(neighbour);
            }
        }

        Graph { neighbours }
    }

    /// Print a simple ASCII version of a graph in the context of the current level.
    fn visualise_graph(&self, graph: &Graph<Position>) {
        let mut line = "".to_string();
        for (index, &bg) in self.background.iter().enumerate() {
            let pos = self.position(index);
            let c = if graph.neighbours.contains_key(&pos) {
                '.'
            } else if bg == Background::Wall {
                '#'
            } else {
                ' '
            };
            line.push(c);
            if index % self.columns == self.columns - 1 {
                info!("{}", line);
                line.truncate(0);
            }
        }
    }

    fn is_valid_for_path_with_crate(&self, from: Position, to: Position) -> Option<()> {
        if from == to || !self.crates.contains_key(&from) || !self.is_empty(to) {
            warn!(
                "Cannot move crate from ({},{}) to ({},{}):",
                from.x, from.y, to.x, to.y
            );
            if from == to {
                warn!("same position");
            } else if !self.crates.contains_key(&from) {
                warn!("source is not a crate");
            } else {
                warn!("target is not empty");
            }
            None
        } else {
            Some(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn cannot_move_into_wall() {
        let s = "#####\n\
                 #@$.#\n\
                 #####";
        let sut = Level::parse(0, s).unwrap();
        let from = Position { x: 2, y: 1 };
        let to = Position { x: 0, y: 0 };
        assert!(sut.find_path_with_crate(from, to).is_none());
    }

    #[test]
    fn fails_when_no_path_exists() {
        let s = "######\n\
                 #$#@.#\n\
                 ######";
        let sut = Level::parse(0, s).unwrap();
        let from = Position { x: 1, y: 1 };
        let to = Position { x: 4, y: 1 };
        assert!(sut.find_path_with_crate(from, to).is_none());
    }

    #[test]
    fn find_trivial_path() {
        let s = "#####\n\
                 #@$.#\n\
                 #####";
        let sut = Level::parse(0, s).unwrap();
        let from = Position { x: 1, y: 1 };

        assert!(sut.find_path_with_crate(from, from).is_none());
    }

    #[test]
    fn find_simplest_nontrivial_path() {
        let s = "#####\n\
                 #@$.#\n\
                 #####";
        let sut = Level::parse(0, s).unwrap();
        let from = Position { x: 2, y: 1 };
        let to = Position { x: 3, y: 1 };

        let path = sut.find_path_with_crate(from, to);

        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.start, from);
        assert_eq!(path.steps.len(), 1);
        assert_eq!(path.steps[0].direction, Direction::Right);
    }
}
