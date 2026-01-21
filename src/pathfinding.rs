use crate::components::board::Tile;
use crate::settings::GameSettings;
use rand::seq::{IndexedRandom, SliceRandom};
use std::cell::RefCell;
use std::collections::HashMap;

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
    ) -> Vec<(i32, i32)>;

    /// Get the name of this algorithm (for debugging/UI)
    fn name(&self) -> &str;
}

pub struct Agent {
    pub start: (i32, i32),
    pub goal: (i32, i32),
    pub position: (i32, i32),
    pub path: Vec<(i32, i32)>,
}

impl Agent {
    pub fn get_next_move(
        &mut self,
        algorithm: &str,
        map: &HashMap<(i32, i32), Tile>,
    ) -> ((i32, i32), (i32, i32)) {
        if self.path.len() == 0 {
            self.path = AStarSearch.find_path(self.start, self.goal, &map);
        }
        println!("{:#?}", self.path);
        let current_position = self.position;
        println!("{:#?}", self.position);
        self.position = self.path.pop().expect("No moves given");
        return (current_position, self.position);
    }
    pub fn get_goal(&self) -> (i32, i32) {
        return self.goal;
    }
}

pub fn get_algorithm(algorithm: &str) -> Box<dyn PathfindingAlgorithm> {
    match algorithm {
        "A*" => return Box::new(AStarSearch),
        "BFS" => return Box::new(BreadthFirstSearch),
        _ => return Box::new(GreedySearch),
    }
}

pub struct GreedySearch;

impl PathfindingAlgorithm for GreedySearch {
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> Vec<(i32, i32)> {
        let mut current = start;
        let mut path: Vec<(i32, i32)> = vec![];
        let mut black_list: Vec<(i32, i32)> = vec![];
        while current != goal {
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

        println!("Yepp");
        path.reverse();
        return path;
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
    ) -> Vec<(i32, i32)> {
        use std::collections::VecDeque;

        if start == goal {
            return vec![start];
        }

        let mut queue = VecDeque::new();
        let mut visited = std::collections::HashSet::new();
        let mut parent: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == goal {
                // reconstruct path
                let mut path = vec![goal];
                let mut node = goal;
                while let Some(&prev) = parent.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                path.reverse();
                println!("{:#?}", path);
                return path;
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

        vec![] // no path found
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
    ) -> Vec<(i32, i32)> {
        use std::cmp::Ordering;
        use std::collections::BinaryHeap;

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

        while let Some(Node {
            position: current, ..
        }) = open_set.pop()
        {
            if current == goal {
                // reconstruct path
                let mut path = vec![goal];
                let mut node = goal;
                while let Some(&prev) = parent.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                return path;
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
                        let tentative_g = g_score.get(&current).unwrap_or(&i32::MAX) + 1;
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

        vec![] // no path found
    }

    fn name(&self) -> &str {
        "A* Search"
    }
}
