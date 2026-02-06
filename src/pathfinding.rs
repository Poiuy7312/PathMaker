use crate::benchmarks::sobel_method;
use crate::components::board::Tile;
use crate::settings::GameSettings;
use jemalloc_ctl::{epoch, stats, thread};
use rand::seq::{IndexedRandom, SliceRandom};
use sdl2::sys::LeaveNotify;
use serde::Serialize;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

/// Trait for custom pathfinding algorithms
/// Implementers can create their own A*, Dijkstra, BFS, etc.

pub trait PathfindingAlgorithm {
    /// Find a path from start to goal
    /// Returns a Vec of waypoints from start to goal (inclusive)
    /// Returns empty Vec if no path exists
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> (Vec<(i32, i32)>, u128);

    /// Get the name of this algorithm (for debugging/UI)
    fn name(&self) -> &str;
}

#[derive(Clone)]
pub struct Agent {
    pub start: (i32, i32),
    pub goal: (i32, i32),
    pub position: (i32, i32),
    pub path: Vec<(i32, i32)>,
}

fn get_overall_path_weight(path: &Vec<(i32, i32)>, map: &HashMap<(i32, i32), Tile>) -> u128 {
    let mut total_weight: u128 = 0;
    for moves in path {
        if let Some(tile) = map.get(&moves) {
            total_weight += tile.weight as u128;
        }
    }
    return total_weight;
}

impl Agent {
    pub fn get_path(
        &mut self,
        algorithm: &str,
        map: &HashMap<(i32, i32), Tile>,
    ) -> (bool, Vec<(i32, i32)>, f64, u64, Duration, u128, u128) {
        let allocated = thread::allocatedp::mib().unwrap();
        let mut all_possible_move: Vec<&(i32, i32)> = vec![];
        for (loc, tile) in map {
            if tile.is_traversable() {
                all_possible_move.push(loc);
            }
        }

        let now = Instant::now();
        epoch::advance().unwrap();
        let before = allocated.read().unwrap().get();
        // Call your function here

        // Capture final stats
        let (path, steps) = get_algorithm(algorithm).find_path(self.start, self.goal, &map);
        epoch::advance().unwrap();
        let after = allocated.read().unwrap().get();
        let time = now.elapsed();

        return (
            true,
            path,
            sobel_method(&map),
            after - before,
            time,
            steps,
            get_overall_path_weight(&self.path, map),
        );
    }
    pub fn goal_reached(&self) -> bool {
        return self.position == self.goal;
    }
    pub fn is_path_possible(
        &self,
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> bool {
        let start = self.start;
        let goal = self.goal;
        if start == goal {
            return true;
        }

        let mut queue = VecDeque::new();
        let mut visited = std::collections::HashSet::new();
        let mut parent: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

        queue.push_back(self.start);
        visited.insert(start);

        let mut steps = 0;

        while let Some(current) = queue.pop_front() {
            if steps >= map.len() as u128 {
                break;
            }
            steps += 1;
            if current == goal {
                // reconstruct path
                return true;
            }

            // get neighbors
            let neighbors = vec![
                (current.0 + 1, current.1),
                (current.0, current.1 + 1),
                (current.0.saturating_sub(1), current.1),
                (current.0, current.1.saturating_sub(1)),
            ];

            for neighbor in neighbors {
                if let Some(tile) = map.get(&neighbor) {
                    if tile.is_traversable() && !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        parent.insert(neighbor, current);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        false // no path found
    }
}

pub fn get_algorithm(algorithm: &str) -> Box<dyn PathfindingAlgorithm> {
    println!("{}", algorithm);
    match algorithm.trim() {
        "A* search" => {
            println!("Using A star algorithm");
            return Box::new(AStarSearch);
        }
        "Breadth First Search" => {
            println!("Using BFS");
            return Box::new(BreadthFirstSearch);
        }
        _ => {
            println!("Using Greedy");
            return Box::new(GreedySearch);
        }
    }
}

pub struct GreedySearch;

impl PathfindingAlgorithm for GreedySearch {
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> (Vec<(i32, i32)>, u128) {
        let mut current = start;
        let mut path: Vec<(i32, i32)> = vec![];
        let mut black_list: Vec<(i32, i32)> = vec![];
        let mut steps: u128 = 0;
        while current != goal {
            steps += 1;
            let mut good_moves: Vec<(i32, i32)> = vec![];
            let mut bad_moves: Vec<(i32, i32)> = vec![];
            let mut neighbors = vec![
                (current.0 + 1, current.1),
                (current.0, current.1 + 1),
                (current.0.saturating_sub(1), current.1),
                (current.0, current.1.saturating_sub(1)),
            ];

            neighbors.retain(|a| {
                if !black_list.contains(a) {
                    if let Some(tile) = map.get(&a) {
                        if tile.is_traversable() {
                            return true;
                        }
                    }
                }
                return false;
            });

            for neighbor in &neighbors {
                if (neighbor.0 - goal.0).abs() < (current.0 - goal.0).abs()
                    || (neighbor.1 - goal.1).abs() < (current.1 - goal.1).abs()
                {
                    good_moves.push(*neighbor);
                    break;
                } else {
                    bad_moves.push(*neighbor);
                }
            }
            if let Some(chosen_move) = good_moves.choose(&mut rand::rng()) {
                current = *chosen_move;
                path.push(*chosen_move);
            } else if let Some(chosen_move) = bad_moves.choose(&mut rand::rng()) {
                black_list.push(current);
                current = *chosen_move;
                path.push(*chosen_move);
            }
        }

        path.reverse();
        return (path, steps);
    }

    fn name(&self) -> &str {
        "Greedy"
    }
}

/// Simple BFS pathfinding implementation
pub struct BreadthFirstSearch;

impl PathfindingAlgorithm for BreadthFirstSearch {
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), Tile>,
    ) -> (Vec<(i32, i32)>, u128) {
        if start == goal {
            return (vec![start], 1);
        }

        let mut queue = VecDeque::new();
        let mut visited = std::collections::HashSet::new();
        let mut parent: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        let mut steps = 0;

        while let Some(current) = queue.pop_front() {
            steps += 1;
            if current == goal {
                // reconstruct path
                let mut path = vec![goal];
                let mut node = goal;
                while let Some(&prev) = parent.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                return (path, steps);
            }

            // get neighbors
            let neighbors = vec![
                (current.0 + 1, current.1),
                (current.0, current.1 + 1),
                (current.0.saturating_sub(1), current.1),
                (current.0, current.1.saturating_sub(1)),
            ];

            for neighbor in neighbors {
                if let Some(tile) = map.get(&neighbor) {
                    if tile.is_traversable() && !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        parent.insert(neighbor, current);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        (vec![], steps) // no path found
    }

    fn name(&self) -> &str {
        "Breadth-First Search"
    }
}

/// A* pathfinding implementation
pub struct AStarSearch;

impl PathfindingAlgorithm for AStarSearch {
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> (Vec<(i32, i32)>, u128) {
        #[derive(Clone, Eq, PartialEq)]
        struct Node {
            cost: i32,
            position: (i32, i32),
        }

        impl Ord for Node {
            fn cmp(&self, other: &Self) -> Ordering {
                other.cost.cmp(&self.cost)
            }
        }

        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        fn heuristic(pos: (i32, i32), goal: (i32, i32)) -> i32 {
            return (goal.0 as i32 - pos.0 as i32).abs() + (goal.1 as i32 - pos.1 as i32).abs();
        }

        let mut open_set: BinaryHeap<Node> = BinaryHeap::new();
        let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
        let mut parent: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

        open_set.push(Node {
            cost: heuristic(start, goal),
            position: start,
        });
        g_score.insert(start, 0);

        let mut steps: u128 = 0;

        while let Some(Node {
            position: current, ..
        }) = open_set.pop()
        {
            steps += 1;
            if current == goal {
                // reconstruct path
                let mut path = vec![goal];
                let mut node = goal;
                while let Some(&prev) = parent.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                return (path, steps);
            }

            let neighbors = vec![
                (current.0 + 1, current.1),
                (current.0, current.1 + 1),
                (current.0.saturating_sub(1), current.1),
                (current.0, current.1.saturating_sub(1)),
            ];

            for neighbor in neighbors {
                if let Some(tile) = map.get(&neighbor) {
                    if tile.is_traversable() {
                        let move_cost = if tile.weight > 1 {
                            tile.weight as i32
                        } else {
                            1
                        };
                        let tentative_g = g_score.get(&current).unwrap_or(&i32::MAX) + move_cost;
                        if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                            parent.insert(neighbor, current);
                            g_score.insert(neighbor, tentative_g);
                            let f = tentative_g + heuristic(neighbor, goal);
                            open_set.push(Node {
                                cost: f,
                                position: neighbor,
                            });
                        }
                    }
                }
            }
        }

        (vec![], steps) // no path found
    }

    fn name(&self) -> &str {
        "A* Search"
    }
}
