use std::collections::HashMap;

/// Trait for custom pathfinding algorithms
/// Implementers can create their own A*, Dijkstra, BFS, etc.
pub trait PathfindingAlgorithm {
    /// Find a path from start to goal
    /// Returns a Vec of waypoints from start to goal (inclusive)
    /// Returns empty Vec if no path exists
    fn find_path(
        &self,
        start: (u32, u32),
        goal: (u32, u32),
        map: &HashMap<(u32, u32), crate::components::board::Tile>,
        obstacles: &std::collections::HashSet<(u32, u32)>,
    ) -> Vec<(u32, u32)>;

    /// Get the name of this algorithm (for debugging/UI)
    fn name(&self) -> &str;
}

/// Simple BFS pathfinding implementation
pub struct BreadthFirstSearch;

impl PathfindingAlgorithm for BreadthFirstSearch {
    fn find_path(
        &self,
        start: (u32, u32),
        goal: (u32, u32),
        map: &HashMap<(u32, u32), crate::components::board::Tile>,
        obstacles: &std::collections::HashSet<(u32, u32)>,
    ) -> Vec<(u32, u32)> {
        use std::collections::VecDeque;

        if start == goal {
            return vec![start];
        }

        let mut queue = VecDeque::new();
        let mut visited = std::collections::HashSet::new();
        let mut parent: HashMap<(u32, u32), (u32, u32)> = HashMap::new();

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
                if map.contains_key(&neighbor)
                    && !obstacles.contains(&neighbor)
                    && !visited.contains(&neighbor)
                {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current);
                    queue.push_back(neighbor);
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
        start: (u32, u32),
        goal: (u32, u32),
        map: &HashMap<(u32, u32), crate::components::board::Tile>,
        obstacles: &std::collections::HashSet<(u32, u32)>,
    ) -> Vec<(u32, u32)> {
        use std::cmp::Ordering;
        use std::collections::BinaryHeap;

        #[derive(Clone, Eq, PartialEq)]
        struct Node {
            cost: u32,
            position: (u32, u32),
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

        fn heuristic(pos: (u32, u32), goal: (u32, u32)) -> u32 {
            (((goal.0 as i32 - pos.0 as i32).abs() + (goal.1 as i32 - pos.1 as i32).abs()) as u32)
        }

        let mut open_set = BinaryHeap::new();
        let mut g_score: HashMap<(u32, u32), u32> = HashMap::new();
        let mut parent: HashMap<(u32, u32), (u32, u32)> = HashMap::new();

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
                path.reverse();
                return path;
            }

            let neighbors = vec![
                (current.0 + 1, current.1),
                (current.0, current.1 + 1),
                (current.0.saturating_sub(1), current.1),
                (current.0, current.1.saturating_sub(1)),
            ];

            for neighbor in neighbors {
                if map.contains_key(&neighbor) && !obstacles.contains(&neighbor) {
                    let tentative_g = g_score.get(&current).unwrap_or(&u32::MAX) + 1;

                    if tentative_g < *g_score.get(&neighbor).unwrap_or(&u32::MAX) {
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

        vec![] // no path found
    }

    fn name(&self) -> &str {
        "A* Search"
    }
}
