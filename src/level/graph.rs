use std::collections::{HashSet, VecDeque};

use crate::direction::*;
use crate::level::*;
use crate::move_::Move;
use crate::position::*;

/// A directed graph.
pub struct Graph<T> {
    pub neighbours: HashMap<T, Vec<T>>,
}

impl Graph<Position> {
    fn find_paths_starting_at(&self, from: Position) -> HashMap<Position, Vec<Position>> {
        let mut predecessors: HashMap<Position, Vec<Position>> = HashMap::new();

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from);

        while let Some(pos) = queue.pop_front() {
            if visited.contains(&pos) {
                continue;
            }

            visited.insert(pos);

            for &neighbour in &self.neighbours[&pos] {
                queue.push_back(neighbour);
                predecessors.entry(neighbour).or_default().push(pos);
            }
        }

        predecessors
    }

    pub fn find_crate_path(&self, from: Position, to: Position) -> Option<Path> {
        if !self.neighbours.contains_key(&to) {
            return None;
        }

        let predecessors = self.find_paths_starting_at(from);

        let mut positions = vec![to];

        loop {
            let pos = *positions.last().unwrap();
            if pos == from {
                break;
            }

            positions.push(predecessors[&pos][0]);
        }

        for p in &positions {
            info!("{:?}", p);
        }

        let mut steps = vec![];
        let len = positions.len();
        for i in 1..len {
            let direction = direction(positions[len - i], positions[len - i - 1]).unwrap();
            steps.push(Move {
                direction,
                moves_crate: true,
            });
        }

        for s in &steps {
            info!("{:?}", s);
        }

        let dir = steps.first().unwrap().direction;
        let start = from.neighbour(dir.reverse());

        Some(Path { start, steps })
    }
}
