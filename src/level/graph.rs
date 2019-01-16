use std::collections::{HashSet, VecDeque};
use std::hash::Hash;

use crate::direction::*;
use crate::level::*;
use crate::move_::Move;
use crate::position::*;

/// A directed graph.
pub struct Graph<T: Eq> {
    pub neighbours: HashMap<T, Vec<T>>,
}

impl<T: Clone + Eq + Hash> Graph<T> {
    fn find_paths_starting_at(&self, from: T) -> HashMap<T, Vec<T>> {
        let mut predecessors: HashMap<T, Vec<T>> = HashMap::new();

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from);

        while let Some(pos) = queue.pop_front() {
            if visited.contains(&pos) {
                continue;
            }

            visited.insert(pos.clone());

            for neighbour in &self.neighbours[&pos] {
                queue.push_back(neighbour.clone());
                predecessors
                    .entry(neighbour.clone())
                    .or_default()
                    .push(pos.clone());
            }
        }

        predecessors
    }
}

impl Graph<Position> {
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

        info!("crate positions in reverse order: {:?}", positions);

        let mut steps = vec![];
        let len = positions.len();
        for i in 1..len {
            let direction = direction(positions[len - i], positions[len - i - 1]).unwrap();
            steps.push(Move {
                direction,
                moves_crate: true,
            });
        }

        info!("the crate has to move as follows: {:?}", steps);

        Some(Path { start: from, steps })
    }
}
