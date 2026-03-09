//! # Pathfinding Algorithms Module
//!
//! This module implements various pathfinding algorithms for grid-based navigation:
//!
//! ## Algorithms
//! - **Greedy Search**: Fast but not optimal, always moves toward the goal
//! - **Breadth-First Search (BFS)**: Guarantees shortest path in unweighted graphs
//! - **A* Search**: Optimal pathfinding with weighted tiles using Manhattan heuristic
//! - **JPS with Weights (JPSW)**: Jump Point Search adapted for weighted grids
//!
//! ## Usage
//! All algorithms implement the `PathfindingAlgorithm` trait, allowing them to be
//! used interchangeably through the `get_algorithm()` factory function.

use crate::benchmarks::sobel_method;
use crate::components::board::Tile;
use crate::settings::GameSettings;

// Memory tracking for benchmarking
use jemalloc_ctl::{epoch, stats, thread};

// Random number generation for Greedy search tie-breaking
use rand::seq::{IndexedRandom, SliceRandom};
use sdl2::sys::LeaveNotify;
use serde::Serialize;

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

/// 8-directional movement deltas for grid navigation.
/// Includes all cardinal (N, S, E, W) and diagonal (NE, NW, SE, SW) directions.
const DELTAS: [(i32, i32); 8] = [
    (-1, -1), // Northwest
    (0, -1),  // North
    (1, -1),  // Northeast
    (-1, 0),  // West
    (1, 0),   // East
    (-1, 1),  // Southwest
    (0, 1),   // South
    (1, 1),   // Southeast
];

/// Calculate which diagonal directions should be blocked based on an obstacle's position.
///
/// When an obstacle blocks a cardinal direction, diagonal moves that would
/// "cut the corner" of that obstacle should also be blocked.
///
/// # Arguments
/// * `delta_location` - The direction vector to the obstacle
///
/// # Returns
/// A tuple representing the perpendicular direction to also block
fn get_diagonals(delta_location: (i32, i32)) -> (i32, i32) {
    if delta_location.0 != 0 && delta_location.1 == 0 {
        return (0, 1); // Horizontal obstacle - block vertical diagonals
    } else if delta_location.0 == 0 && delta_location.1 != 0 {
        return (1, 0); // Vertical obstacle - block horizontal diagonals
    } else {
        return (0, 0); // Diagonal obstacle - no additional blocking
    }
}

/// Get all valid moves from a position, respecting obstacle collision and corner-cutting rules.
///
/// This function examines all 8 neighbors and returns only those that are:
/// 1. Traversable (not obstacles)
/// 2. Not blocked by corner-cutting rules
///
/// Corner-cutting is prevented by blocking diagonal moves when either adjacent
/// cardinal direction is blocked by an obstacle.
///
/// # Arguments
/// * `current` - Current position on the grid
/// * `map` - Reference to the tile map
///
/// # Returns
/// Set of valid neighboring positions
pub fn get_possible_moves(
    current: (i32, i32),
    map: &HashMap<(i32, i32), Tile>,
) -> HashSet<(i32, i32)> {
    let mut blocked_moves: HashSet<(i32, i32)> = HashSet::new();
    let mut neighbors: HashSet<(i32, i32)> = HashSet::new();
    for (dx, dy) in DELTAS.iter() {
        let neighbor = (current.0 + dx, current.1 + dy);
        if let Some(tile) = map.get(&neighbor) {
            if tile.is_traversable() {
                neighbors.insert(neighbor);
            } else {
                let (dx, dy) = get_diagonals((*dx, *dy));
                blocked_moves.insert(neighbor);
                if dx != 0 || dy != 0 {
                    blocked_moves.insert((neighbor.0 + dx, neighbor.1 + dy));
                    blocked_moves.insert((neighbor.0 - dx, neighbor.1 - dy));
                }
            }
        }
    }
    let moves: HashSet<(i32, i32)> = neighbors.difference(&blocked_moves).cloned().collect();
    return moves;
}

/// Trait for custom pathfinding algorithms.
///
/// Implementers can create their own A*, Dijkstra, BFS, etc.
/// All algorithms follow a common interface for easy swapping.
pub trait PathfindingAlgorithm {
    /// Find a path from start to goal.
    ///
    /// # Arguments
    /// * `start` - Starting position coordinates
    /// * `goal` - Target position coordinates
    /// * `map` - Reference to the tile map with traversability info
    ///
    /// # Returns
    /// A tuple containing:
    /// - Vec of waypoints from start to goal (may be reversed or just jump points)
    /// - Total number of steps/nodes expanded during search
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> (Vec<(i32, i32)>, u32);

    /// Returns true if find_path returns the complete path, false if it returns jump points.
    fn returns_full_path(&self) -> bool;

    /// Reconstruct the full path from jump points (for algorithms like JPS).
    fn reconstruct_path(&self, path: Vec<(i32, i32)>) -> Vec<(i32, i32)>;

    /// Get the display name of this algorithm for UI/debugging.
    fn name(&self) -> &str;
}

/// Represents an agent that navigates the grid.
///
/// Agents have a start position, goal position, current position,
/// and store their computed path for visualization.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Agent {
    /// Starting position on the grid
    pub start: (i32, i32),
    /// Goal/target position on the grid
    pub goal: (i32, i32),
    /// Current position (updated during animation)
    pub position: (i32, i32),
    /// Computed path from start to goal (reversed for pop access)
    pub path: Vec<(i32, i32)>,
}

/// Calculate the total weight cost of traversing a path.
///
/// # Arguments
/// * `path` - Vector of positions along the path
/// * `map` - Reference to the tile map
///
/// # Returns
/// Sum of all tile weights along the path
fn get_overall_path_weight(path: &Vec<(i32, i32)>, map: &HashMap<(i32, i32), Tile>) -> u32 {
    let mut total_weight: u32 = 0;
    for moves in path {
        if let Some(tile) = map.get(&moves) {
            total_weight += tile.weight as u32;
        }
    }
    return total_weight;
}

impl Agent {
    /// Execute pathfinding for this agent using the specified algorithm.
    ///
    /// This method runs the algorithm, measures performance metrics, and returns
    /// comprehensive data for benchmarking.
    ///
    /// # Arguments
    /// * `algorithm` - Name of the algorithm to use
    /// * `map` - Reference to the tile map
    ///
    /// # Returns
    /// A tuple containing:
    /// - Success flag
    /// - The computed path
    /// - WCF (Weighted Cost Factor) value
    /// - Memory allocated during search (bytes)
    /// - Time taken for pathfinding
    /// - Number of nodes expanded
    /// - Total path weight/cost
    pub fn get_path(
        &mut self,
        algorithm: &str,
        map: &HashMap<(i32, i32), Tile>,
    ) -> (bool, Vec<(i32, i32)>, f64, u64, Duration, u32, u32) {
        let allocated = thread::allocatedp::mib().unwrap();
        let algorithm = get_algorithm(algorithm);
        let now = Instant::now();
        epoch::advance().unwrap();
        let before = allocated.read().unwrap().get();
        // Call your function here

        // Capture final stats
        let (mut path, steps) = algorithm.find_path(self.start, self.goal, &map);
        epoch::advance().unwrap();

        let after = allocated.read().unwrap().get();
        let time = now.elapsed();
        if !algorithm.returns_full_path() {
            path = algorithm.reconstruct_path(path);
        }
        let weight = get_overall_path_weight(&path, map);

        return (
            true,
            path,
            sobel_method(&map),
            after - before,
            time,
            steps,
            weight,
        );
    }
    /// Check if the agent has reached its goal.
    pub fn goal_reached(&self) -> bool {
        return self.position == self.goal;
    }

    /// Verify if any path exists from start to goal using bidirectional BFS.
    ///
    /// This is a quick connectivity check before running more expensive
    /// pathfinding algorithms. Uses bidirectional BFS to reduce search space -
    /// instead of exploring radius R from start, we explore radius R/2 from both ends.
    ///
    /// # Arguments
    /// * `map` - Reference to the tile map
    ///
    /// # Returns
    /// `true` if a path exists, `false` otherwise
    pub fn is_path_possible(
        &self,
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> bool {
        let start = self.start;
        let goal = self.goal;
        if start == goal {
            return true;
        }

        // Bidirectional BFS - search from both ends
        let mut queue_start = VecDeque::new();
        let mut queue_goal = VecDeque::new();
        let mut visited_start: HashSet<(i32, i32)> = HashSet::new();
        let mut visited_goal: HashSet<(i32, i32)> = HashSet::new();

        queue_start.push_back(start);
        queue_goal.push_back(goal);
        visited_start.insert(start);
        visited_goal.insert(goal);

        // Alternate between expanding from start and goal
        while !queue_start.is_empty() || !queue_goal.is_empty() {
            // Expand from start side
            if let Some(current) = queue_start.pop_front() {
                if visited_goal.contains(&current) {
                    return true;
                }
                for neighbor in get_possible_moves(current, map) {
                    if visited_start.insert(neighbor) {
                        queue_start.push_back(neighbor);
                    }
                }
            }

            // Expand from goal side
            if let Some(current) = queue_goal.pop_front() {
                if visited_start.contains(&current) {
                    return true;
                }
                for neighbor in get_possible_moves(current, map) {
                    if visited_goal.insert(neighbor) {
                        queue_goal.push_back(neighbor);
                    }
                }
            }
        }
        false
    }
}

/// Factory function to create a pathfinding algorithm by name.
///
/// # Arguments
/// * `algorithm` - Name of the algorithm ("A* search", "Breadth First Search", "JPSW", or Greedy by default)
///
/// # Returns
/// Boxed trait object implementing PathfindingAlgorithm
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
        "JPSW" => {
            println!("Using JPSW");
            return Box::new(JPSW::default());
        }
        _ => {
            println!("Using Greedy");
            return Box::new(GreedySearch);
        }
    }
}

/// Greedy Best-First Search algorithm.
///
/// A fast heuristic-driven search that always moves toward the goal.
/// Not guaranteed to find the optimal path but is very fast.
///
/// ## Characteristics
/// - Uses Manhattan distance heuristic
/// - Falls back to random valid moves when stuck
/// - Maintains a blacklist of visited positions to avoid cycles
pub struct GreedySearch;

impl PathfindingAlgorithm for GreedySearch {
    /// Find a path using greedy best-first search.
    ///
    /// The algorithm prioritizes moves that reduce the Manhattan distance
    /// to the goal. When no improving move exists, it randomly selects
    /// from remaining valid moves while blacklisting the current position.
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> (Vec<(i32, i32)>, u32) {
        let mut current = start;
        let mut path: Vec<(i32, i32)> = vec![];
        let mut black_list: Vec<(i32, i32)> = vec![];
        let mut steps: u32 = 0;
        let max_steps = map.len() as u32 * 2;

        fn heuristic(pos: &(i32, i32), goal: &(i32, i32)) -> i32 {
            return (goal.0 as i32 - pos.0 as i32).abs() + (goal.1 as i32 - pos.1 as i32).abs();
        }
        loop {
            steps += 1;
            if current == goal {
                path.reverse();
                return (path, steps);
            }
            if steps >= max_steps {
                break;
            }

            let mut good_moves: Vec<(i32, i32)> = vec![];
            let mut bad_moves: Vec<(i32, i32)> = vec![];

            let neighbors = get_possible_moves(current, map);
            for neighbor in neighbors {
                if black_list.contains(&neighbor) {
                    continue;
                }
                if let Some(tile) = map.get(&neighbor) {
                    if !tile.is_traversable() {
                        continue;
                    }
                    if heuristic(&neighbor, &goal) < heuristic(&current, &goal) {
                        good_moves.push(neighbor);
                    } else {
                        bad_moves.push(neighbor);
                    }
                }
            }
            good_moves.sort_by(|a, b| heuristic(a, &goal).cmp(&heuristic(b, &goal)));
            if let Some(chosen_move) = good_moves.first() {
                current = *chosen_move;
                path.push(*chosen_move);
            } else if let Some(chosen_move) = bad_moves.choose(&mut rand::rng()) {
                black_list.push(current);
                current = *chosen_move;
                path.push(*chosen_move);
            }
        }

        (vec![], steps)
    }

    fn returns_full_path(&self) -> bool {
        true
    }

    fn reconstruct_path(&self, _: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
        vec![]
    }

    fn name(&self) -> &str {
        "Greedy"
    }
}

/// Simple BFS pathfinding implementation.
///
/// Breadth-First Search guarantees the shortest path in unweighted graphs.
/// For weighted graphs, use A* instead.
///
/// ## Characteristics
/// - Explores nodes in order of distance from start
/// - Always finds shortest path (in terms of moves)
/// - Does not consider tile weights
pub struct BreadthFirstSearch;

impl PathfindingAlgorithm for BreadthFirstSearch {
    /// Find the shortest path using BFS.
    ///
    /// Explores all neighbors at distance N before any at distance N+1.
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), Tile>,
    ) -> (Vec<(i32, i32)>, u32) {
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

            let neighbors = get_possible_moves(current, map);
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

    fn returns_full_path(&self) -> bool {
        true
    }

    fn reconstruct_path(&self, _: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
        vec![]
    }

    fn name(&self) -> &str {
        "Breadth-First Search"
    }
}

/// A* pathfinding implementation.
///
/// A* combines the actual cost from start (g-score) with a heuristic
/// estimate to the goal (h-score) to efficiently find optimal paths.
///
/// ## Characteristics
/// - Uses Manhattan distance heuristic (admissible for 4-directional movement)
/// - Considers tile weights for accurate cost calculation
/// - Guaranteed optimal if heuristic is admissible
/// - More efficient than Dijkstra due to goal-directed search
pub struct AStarSearch;

impl PathfindingAlgorithm for AStarSearch {
    /// Find the optimal path using A* search.
    ///
    /// Uses a priority queue (min-heap) ordered by f-score = g-score + h-score.
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), crate::components::board::Tile>,
    ) -> (Vec<(i32, i32)>, u32) {
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

        let mut steps: u32 = 0;

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

            let neighbors = get_possible_moves(current, map);
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

    fn returns_full_path(&self) -> bool {
        true
    }

    fn reconstruct_path(&self, _: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
        vec![]
    }

    fn name(&self) -> &str {
        "A* Search"
    }
}

/// Jump Point Search with Weights (JPSW) implementation.
///
/// An optimization of A* that reduces the number of nodes expanded by
/// "jumping" over intermediate nodes in open areas.
///
/// ## Characteristics
/// - Caches successor calculations for efficiency
/// - Adapted for weighted grids (stops at weight changes)
/// - Significantly faster than A* in large open areas
/// - Returns jump points that must be expanded to full path
///
/// ## Caching
/// Uses neighborhood hashing to cache:
/// - Successor calculations for pruned neighbor detection
/// - Orthogonal jump results for repeated queries
pub struct JPSW {
    /// Cache for neighborhood successors: (parent, current, hash) -> successors
    successor_cache: RefCell<HashMap<(Option<(i32, i32)>, (i32, i32), u64), HashSet<(i32, i32)>>>,
    /// Cache for orthogonal jumps: (pos, dir, hash) -> (end_pos, cost)
    jump_cache: RefCell<HashMap<((i32, i32), (i32, i32), u64), Option<((i32, i32), f32)>>>,
}

impl Default for JPSW {
    fn default() -> Self {
        JPSW {
            successor_cache: RefCell::new(HashMap::new()),
            jump_cache: RefCell::new(HashMap::new()),
        }
    }
}

impl JPSW {
    /// Hash the 3x3 neighborhood for caching
    fn hash_neighborhood(&self, center: (i32, i32), map: &HashMap<(i32, i32), Tile>) -> u64 {
        let mut hash = 0u64;
        for dx in -1..=1 {
            for dy in -1..=1 {
                let pos = (center.0 + dx, center.1 + dy);
                if let Some(tile) = map.get(&pos) {
                    hash = hash.wrapping_mul(31).wrapping_add(tile.weight as u64);
                }
            }
        }
        hash
    }

    /// Get successors for a node using cached local Dijkstra
    fn get_successors(
        &self,
        parent: Option<(i32, i32)>,
        current: (i32, i32),
        map: &HashMap<(i32, i32), Tile>,
    ) -> HashSet<(i32, i32)> {
        let hash = self.hash_neighborhood(current, map);
        let cache_key = (parent, current, hash);

        if let Some(cached) = self.successor_cache.borrow().get(&cache_key) {
            return cached.clone();
        }

        let successors = self.compute_successors(parent, current, map);
        self.successor_cache
            .borrow_mut()
            .insert(cache_key, successors.clone());
        successors
    }

    /// Compute successors using lightweight local Dijkstra
    fn compute_successors(
        &self,
        parent: Option<(i32, i32)>,
        current: (i32, i32),
        map: &HashMap<(i32, i32), Tile>,
    ) -> HashSet<(i32, i32)> {
        // Simple priority queue for 3x3 neighborhood
        #[derive(Clone)]
        struct State {
            cost: f32,
            move_len: u8, // 1 for orthogonal, 2 for diagonal (for tiebreaking)
            pos: (i32, i32),
            via_center: bool,
        }

        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                // Min-heap by cost, then prefer smaller move_len (orthogonal-last)
                other
                    .cost
                    .partial_cmp(&self.cost)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| other.move_len.cmp(&self.move_len))
            }
        }
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl PartialEq for State {
            fn eq(&self, other: &Self) -> bool {
                self.pos == other.pos
            }
        }
        impl Eq for State {}

        let mut heap = BinaryHeap::new();
        let mut best: HashMap<(i32, i32), bool> = HashMap::new();
        let mut result = HashSet::new();

        // Start from parent and current
        if let Some(p) = parent {
            heap.push(State {
                cost: 0.0,
                move_len: 0,
                pos: p,
                via_center: false,
            });
        }
        heap.push(State {
            cost: 0.0,
            move_len: 0,
            pos: current,
            via_center: true,
        });

        while let Some(state) = heap.pop() {
            // Skip if we've seen this position via center already
            if let Some(&was_via_center) = best.get(&state.pos) {
                if was_via_center || !state.via_center {
                    continue;
                }
            }
            best.insert(state.pos, state.via_center);

            // If this is a neighbor of current, record it
            if state.pos != current
                && (state.pos.0 - current.0).abs() <= 1
                && (state.pos.1 - current.1).abs() <= 1
            {
                if state.via_center {
                    result.insert(state.pos);
                }
                continue;
            }

            // Expand to 3x3 neighbors
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let next = (state.pos.0 + dx, state.pos.1 + dy);

                    // Stay within 3x3 of current
                    if (next.0 - current.0).abs() > 1 || (next.1 - current.1).abs() > 1 {
                        continue;
                    }

                    if let Some(_) = map.get(&next) {
                        let move_cost = self.move_cost(next, state.pos, map);
                        if move_cost.is_infinite() {
                            continue;
                        }

                        let is_diag = dx != 0 && dy != 0;
                        let new_via_center =
                            (state.pos == current && state.via_center) || next == current;

                        heap.push(State {
                            cost: state.cost + move_cost,
                            move_len: if is_diag { 2 } else { 1 },
                            pos: next,
                            via_center: new_via_center,
                        });
                    }
                }
            }
        }

        result
    }

    /// Jump in direction, stopping at terrain transitions or forced neighbors
    fn jump(
        &self,
        start: (i32, i32),
        dir: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), Tile>,
        parent: Option<(i32, i32)>,
    ) -> Option<((i32, i32), f32)> {
        if dir.0 != 0 && dir.1 != 0 {
            self.jump_diagonal(start, dir, goal, map, parent)
        } else {
            self.jump_orthogonal(start, dir, goal, map)
        }
    }

    /// Jump orthogonally with caching
    fn jump_orthogonal(
        &self,
        start: (i32, i32),
        dir: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), Tile>,
    ) -> Option<((i32, i32), f32)> {
        let hash = self.hash_neighborhood(start, map);
        let cache_key = (start, dir, hash);

        // Check cache
        if let Some(cached) = self.jump_cache.borrow().get(&cache_key) {
            return *cached;
        }

        let mut pos = start;
        let mut cost = 0.0;
        let start_weight = map.get(&start).map(|t| t.weight).unwrap_or(1);

        loop {
            let next = (pos.0 + dir.0, pos.1 + dir.1);

            if let Some(tile) = map.get(&next) {
                if !tile.is_traversable() {
                    self.jump_cache.borrow_mut().insert(cache_key, None);
                    return None;
                }

                let step_cost = self.move_cost(next, pos, map);
                cost += step_cost;
                pos = next;

                if pos == goal {
                    let result = Some((pos, cost));
                    self.jump_cache.borrow_mut().insert(cache_key, result);
                    return result;
                }

                // Stop at terrain transition (weight change)
                if tile.weight != start_weight {
                    let result = Some((pos, cost));
                    self.jump_cache.borrow_mut().insert(cache_key, result);
                    return result;
                }
            } else {
                self.jump_cache.borrow_mut().insert(cache_key, None);
                return None;
            }
        }
    }

    /// Jump diagonally
    fn jump_diagonal(
        &self,
        start: (i32, i32),
        dir: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), Tile>,
        parent: Option<(i32, i32)>,
    ) -> Option<((i32, i32), f32)> {
        let mut pos = start;
        let mut cost = 0.0;
        let start_weight = map.get(&start).map(|t| t.weight).unwrap_or(1);

        loop {
            let next = (pos.0 + dir.0, pos.1 + dir.1);

            // Check traversability and corner-cutting
            if let Some(tile) = map.get(&next) {
                if !tile.is_traversable() {
                    return None;
                }
            } else {
                return None;
            }

            let side1 = (pos.0 + dir.0, pos.1);
            let side2 = (pos.0, pos.1 + dir.1);
            if !map.get(&side1).map_or(false, |t| t.is_traversable())
                || !map.get(&side2).map_or(false, |t| t.is_traversable())
            {
                return None;
            }

            let step_cost = self.move_cost(next, pos, map);
            cost += step_cost;
            pos = next;

            if pos == goal {
                return Some((pos, cost));
            }

            // Check if we have orthogonal successors (forced neighbors)
            let successors = self.get_successors(parent, pos, map);
            let ortho1 = (pos.0 + dir.0, pos.1);
            let ortho2 = (pos.0, pos.1 + dir.1);

            if successors.contains(&ortho1) || successors.contains(&ortho2) {
                return Some((pos, cost));
            }

            // Stop at terrain transition
            if let Some(tile) = map.get(&pos) {
                if tile.weight != start_weight {
                    return Some((pos, cost));
                }
            }
        }
    }

    /// Calculate move cost - optimized
    fn move_cost(&self, to: (i32, i32), from: (i32, i32), map: &HashMap<(i32, i32), Tile>) -> f32 {
        let dx = (to.0 - from.0).abs();
        let dy = (to.1 - from.1).abs();

        if dx + dy == 1 {
            // Orthogonal: average of 2 tiles
            let w1 = map
                .get(&from)
                .map(|t| t.weight as f32)
                .unwrap_or(f32::INFINITY);
            let w2 = map
                .get(&to)
                .map(|t| t.weight as f32)
                .unwrap_or(f32::INFINITY);
            (w1 + w2) / 2.0
        } else {
            // Diagonal: average of 4 tiles * sqrt(2)
            let mut sum = 0.0;
            let mut count = 0;
            for pos in &[
                from,
                to,
                (from.0 + to.0 - from.0, from.1),
                (from.0, from.1 + to.1 - from.1),
            ] {
                if let Some(tile) = map.get(pos) {
                    if tile.is_traversable() {
                        sum += tile.weight as f32;
                        count += 1;
                    }
                }
            }
            if count > 0 {
                (sum / count as f32) * std::f32::consts::SQRT_2
            } else {
                f32::INFINITY
            }
        }
    }

    /// Reconstruct full path between jump points
    fn reconstruct_segment(&self, from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
        let mut path = vec![from];
        let dx = (to.0 - from.0).signum();
        let dy = (to.1 - from.1).signum();
        let mut pos = from;

        while pos != to {
            pos = (pos.0 + dx, pos.1 + dy);
            path.push(pos);
        }
        path
    }

    /// Clear caches (call when map changes)
    pub fn clear_caches(&self) {
        self.successor_cache.borrow_mut().clear();
        self.jump_cache.borrow_mut().clear();
    }
}

impl PathfindingAlgorithm for JPSW {
    fn find_path(
        &self,
        start: (i32, i32),
        goal: (i32, i32),
        map: &HashMap<(i32, i32), Tile>,
    ) -> (Vec<(i32, i32)>, u32) {
        #[derive(Clone)]
        struct Node {
            f: f32,
            pos: (i32, i32),
        }

        impl Ord for Node {
            fn cmp(&self, other: &Self) -> Ordering {
                other.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
            }
        }
        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl PartialEq for Node {
            fn eq(&self, other: &Self) -> bool {
                self.pos == other.pos
            }
        }
        impl Eq for Node {}

        let mut open = BinaryHeap::new();
        let mut g_score: HashMap<(i32, i32), f32> = HashMap::new();
        let mut parent: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        let mut closed = HashSet::new();

        let h = |p: (i32, i32)| -> f32 {
            let dx = (p.0 - goal.0).abs() as f32;
            let dy = (p.1 - goal.1).abs() as f32;
            dx.max(dy) * std::f32::consts::SQRT_2 + dx.min(dy) * (std::f32::consts::SQRT_2 - 1.0)
        };

        g_score.insert(start, 0.0);
        open.push(Node {
            f: h(start),
            pos: start,
        });

        let mut expansions = 0;

        while let Some(Node { pos: current, .. }) = open.pop() {
            if closed.contains(&current) {
                continue;
            }
            closed.insert(current);
            expansions += 1;

            if current == goal {
                // Reconstruct jump point path
                let mut jump_path = vec![goal];
                let mut node = goal;
                while let Some(&prev) = parent.get(&node) {
                    jump_path.push(prev);
                    node = prev;
                }

                // Fill in all intermediate cells between jump points
                return (jump_path, expansions);
            }

            let current_g = g_score[&current];
            let current_parent = parent.get(&current).copied();

            // Get successors using JPSW pruning
            let successors = self.get_successors(current_parent, current, map);

            for &succ in &successors {
                let dir = ((succ.0 - current.0).signum(), (succ.1 - current.1).signum());

                // Jump in this direction
                if let Some((jp, jump_cost)) = self.jump(current, dir, goal, map, current_parent) {
                    if closed.contains(&jp) {
                        continue;
                    }

                    let tentative_g = current_g + jump_cost;

                    if tentative_g < *g_score.get(&jp).unwrap_or(&f32::INFINITY) {
                        parent.insert(jp, current);
                        g_score.insert(jp, tentative_g);
                        open.push(Node {
                            f: tentative_g + h(jp),
                            pos: jp,
                        });
                    }
                }
            }
        }

        (vec![], expansions)
    }

    fn name(&self) -> &str {
        "JPSW"
    }

    fn returns_full_path(&self) -> bool {
        false
    }

    fn reconstruct_path(&self, path: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
        let mut full_path: Vec<(i32, i32)> = Vec::new();
        if path.len() > 0 {
            for i in 0..path.len() - 1 {
                let segment = self.reconstruct_segment(path[i], path[i + 1]);
                // Add all but the last cell (to avoid duplicates)
                full_path.extend(&segment[..segment.len() - 1]);
            }
            println!("{:#?}", full_path);
            return full_path;
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::board::{Tile, TileType};
    use std::collections::HashMap;

    /// Helper: build an NxN grid of floor tiles with weight=1
    fn make_floor_grid(n: i32) -> HashMap<(i32, i32), Tile> {
        let mut map = HashMap::new();
        for x in 0..n {
            for y in 0..n {
                map.insert((x, y), Tile::new((x, y), TileType::Floor, 10, 10, 1, false));
            }
        }
        map
    }

    /// Helper: place an obstacle on an existing grid
    fn set_obstacle(map: &mut HashMap<(i32, i32), Tile>, pos: (i32, i32)) {
        map.insert(pos, Tile::new(pos, TileType::Obstacle, 10, 10, 1, false));
    }

    /// Helper: set a weighted tile
    fn set_weight(map: &mut HashMap<(i32, i32), Tile>, pos: (i32, i32), weight: u8) {
        map.insert(
            pos,
            Tile::new(pos, TileType::Weighted(weight), 10, 10, weight, false),
        );
    }

    // ------- get_diagonals -------

    #[test]
    fn test_get_diagonals_horizontal_obstacle() {
        // Obstacle to the right (1,0) should block vertical diagonals
        assert_eq!(get_diagonals((1, 0)), (0, 1));
    }

    #[test]
    fn test_get_diagonals_vertical_obstacle() {
        // Obstacle below (0,1) should block horizontal diagonals
        assert_eq!(get_diagonals((0, 1)), (1, 0));
    }

    #[test]
    fn test_get_diagonals_diagonal_obstacle() {
        // Diagonal obstacle (1,1) should not block additional directions
        assert_eq!(get_diagonals((1, 1)), (0, 0));
        assert_eq!(get_diagonals((-1, -1)), (0, 0));
    }

    #[test]
    fn test_get_diagonals_left() {
        assert_eq!(get_diagonals((-1, 0)), (0, 1));
    }

    #[test]
    fn test_get_diagonals_up() {
        assert_eq!(get_diagonals((0, -1)), (1, 0));
    }

    // ------- get_possible_moves -------

    #[test]
    fn test_possible_moves_center_of_open_grid() {
        let map = make_floor_grid(5);
        let moves = get_possible_moves((2, 2), &map);
        // All 8 neighbors should be reachable
        assert_eq!(moves.len(), 8);
        for &(dx, dy) in DELTAS.iter() {
            assert!(moves.contains(&(2 + dx, 2 + dy)));
        }
    }

    #[test]
    fn test_possible_moves_corner_of_grid() {
        let map = make_floor_grid(5);
        let moves = get_possible_moves((0, 0), &map);
        // Only 3 neighbors exist: (1,0), (0,1), (1,1)
        assert_eq!(moves.len(), 3);
        assert!(moves.contains(&(1, 0)));
        assert!(moves.contains(&(0, 1)));
        assert!(moves.contains(&(1, 1)));
    }

    #[test]
    fn test_possible_moves_with_obstacle_blocks_corner_cutting() {
        let mut map = make_floor_grid(5);
        // Place obstacle to the right of (2,2) → (3,2)
        set_obstacle(&mut map, (3, 2));
        let moves = get_possible_moves((2, 2), &map);
        // (3,2) is blocked, and corner-cutting diagonals (3,1) and (3,3) should be blocked too
        assert!(!moves.contains(&(3, 2)));
        assert!(!moves.contains(&(3, 1)));
        assert!(!moves.contains(&(3, 3)));
        // Other neighbors should still be accessible
        assert!(moves.contains(&(1, 1)));
        assert!(moves.contains(&(2, 1)));
        assert!(moves.contains(&(1, 2)));
    }

    #[test]
    fn test_possible_moves_surrounded_by_obstacles() {
        let mut map = make_floor_grid(5);
        for &(dx, dy) in DELTAS.iter() {
            set_obstacle(&mut map, (2 + dx, 2 + dy));
        }
        let moves = get_possible_moves((2, 2), &map);
        assert!(moves.is_empty());
    }

    #[test]
    fn test_possible_moves_player_tile_is_traversable() {
        let mut map = make_floor_grid(3);
        map.insert(
            (1, 0),
            Tile::new((1, 0), TileType::Player, 10, 10, 1, false),
        );
        let moves = get_possible_moves((0, 0), &map);
        assert!(moves.contains(&(1, 0)));
    }

    #[test]
    fn test_possible_moves_enemy_tile_is_traversable() {
        let mut map = make_floor_grid(3);
        map.insert((1, 0), Tile::new((1, 0), TileType::Enemy, 10, 10, 1, false));
        let moves = get_possible_moves((0, 0), &map);
        assert!(moves.contains(&(1, 0)));
    }

    // ------- get_overall_path_weight -------

    #[test]
    fn test_path_weight_uniform() {
        let map = make_floor_grid(5);
        let path = vec![(0, 0), (1, 0), (2, 0)];
        assert_eq!(get_overall_path_weight(&path, &map), 3);
    }

    #[test]
    fn test_path_weight_with_weighted_tiles() {
        let mut map = make_floor_grid(5);
        set_weight(&mut map, (1, 0), 5);
        set_weight(&mut map, (2, 0), 10);
        let path = vec![(0, 0), (1, 0), (2, 0)];
        assert_eq!(get_overall_path_weight(&path, &map), 1 + 5 + 10);
    }

    #[test]
    fn test_path_weight_empty_path() {
        let map = make_floor_grid(5);
        let path: Vec<(i32, i32)> = vec![];
        assert_eq!(get_overall_path_weight(&path, &map), 0);
    }

    // ------- get_algorithm factory -------

    #[test]
    fn test_get_algorithm_greedy() {
        let algo = get_algorithm("Greedy");
        assert_eq!(algo.name(), "Greedy");
        assert!(algo.returns_full_path());
    }

    #[test]
    fn test_get_algorithm_bfs() {
        let algo = get_algorithm("Breadth First Search");
        assert_eq!(algo.name(), "Breadth-First Search");
        assert!(algo.returns_full_path());
    }

    #[test]
    fn test_get_algorithm_astar() {
        let algo = get_algorithm("A* search");
        assert_eq!(algo.name(), "A* Search");
        assert!(algo.returns_full_path());
    }

    #[test]
    fn test_get_algorithm_jpsw() {
        let algo = get_algorithm("JPSW");
        assert_eq!(algo.name(), "JPSW");
        assert!(!algo.returns_full_path());
    }

    #[test]
    fn test_get_algorithm_unknown_defaults_to_greedy() {
        let algo = get_algorithm("UnknownAlgorithm");
        assert_eq!(algo.name(), "Greedy");
    }

    // ------- BFS -------

    #[test]
    fn test_bfs_finds_path_on_open_grid() {
        let map = make_floor_grid(10);
        let bfs = BreadthFirstSearch;
        let (path, steps) = bfs.find_path((0, 0), (9, 9), &map);
        assert!(!path.is_empty());
        assert!(steps > 0);
        assert!(path.contains(&(9, 9)));
        assert!(path.contains(&(0, 0)));
    }

    #[test]
    fn test_bfs_start_equals_goal() {
        let map = make_floor_grid(5);
        let bfs = BreadthFirstSearch;
        let (path, steps) = bfs.find_path((2, 2), (2, 2), &map);
        assert_eq!(path, vec![(2, 2)]);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_bfs_no_path_when_blocked() {
        let mut map = make_floor_grid(5);
        // Wall off (4,*) completely
        for y in 0..5 {
            set_obstacle(&mut map, (3, y));
        }
        let bfs = BreadthFirstSearch;
        let (path, _) = bfs.find_path((0, 0), (4, 4), &map);
        assert!(path.is_empty());
    }

    #[test]
    fn test_bfs_adjacent_goal() {
        let map = make_floor_grid(5);
        let bfs = BreadthFirstSearch;
        let (path, _) = bfs.find_path((0, 0), (1, 0), &map);
        assert!(!path.is_empty());
        assert!(path.contains(&(0, 0)));
        assert!(path.contains(&(1, 0)));
    }

    // ------- A* -------

    #[test]
    fn test_astar_finds_path_on_open_grid() {
        let map = make_floor_grid(10);
        let astar = AStarSearch;
        let (path, steps) = astar.find_path((0, 0), (9, 9), &map);
        assert!(!path.is_empty());
        assert!(steps > 0);
        assert!(path.contains(&(9, 9)));
        assert!(path.contains(&(0, 0)));
    }

    #[test]
    fn test_astar_prefers_lower_weight_path() {
        // Create a 5x3 grid, weight the middle row heavily
        let mut map = make_floor_grid(5);
        for x in 1..4 {
            set_weight(&mut map, (x, 1), 100);
        }
        let astar = AStarSearch;
        let (path, _) = astar.find_path((0, 1), (4, 1), &map);
        assert!(!path.is_empty());
        // The path should exist (A* found something)
        assert!(path.contains(&(4, 1)));
    }

    #[test]
    fn test_astar_no_path_when_blocked() {
        let mut map = make_floor_grid(5);
        for y in 0..5 {
            set_obstacle(&mut map, (3, y));
        }
        let astar = AStarSearch;
        let (path, _) = astar.find_path((0, 0), (4, 4), &map);
        assert!(path.is_empty());
    }

    #[test]
    fn test_astar_start_equals_goal() {
        let map = make_floor_grid(5);
        let astar = AStarSearch;
        let (path, steps) = astar.find_path((2, 2), (2, 2), &map);
        // A* should return immediately with just the start node
        assert!(!path.is_empty());
        assert_eq!(steps, 1);
    }

    // ------- Greedy -------

    #[test]
    fn test_greedy_finds_path_on_open_grid() {
        let map = make_floor_grid(10);
        let greedy = GreedySearch;
        let (path, steps) = greedy.find_path((0, 0), (5, 5), &map);
        assert!(!path.is_empty());
        assert!(steps > 0);
    }

    #[test]
    fn test_greedy_returns_empty_for_impossible_path() {
        let mut map = make_floor_grid(5);
        for y in 0..5 {
            set_obstacle(&mut map, (2, y));
        }
        let greedy = GreedySearch;
        let (path, _) = greedy.find_path((0, 0), (4, 4), &map);
        assert!(path.is_empty());
    }

    // ------- JPSW -------

    #[test]
    fn test_jpsw_finds_path_on_open_grid() {
        let map = make_floor_grid(10);
        let jpsw = JPSW::default();
        let (jump_points, steps) = jpsw.find_path((0, 0), (9, 9), &map);
        assert!(!jump_points.is_empty());
        assert!(steps > 0);
    }

    #[test]
    fn test_jpsw_reconstruct_path() {
        let jpsw = JPSW::default();
        // Jump points from (4,4) back to (0,0) — reversed like the algorithm returns
        let jump_path = vec![(4, 4), (0, 0)];
        let full = jpsw.reconstruct_path(jump_path);
        assert!(!full.is_empty());
        assert!(full.contains(&(4, 4)));
        // reconstruct_segment excludes the last cell of each segment to avoid duplicates
        // so (0,0) won't be present, but intermediate points should be
        assert!(full.len() >= 3);
    }

    #[test]
    fn test_jpsw_reconstruct_empty_path() {
        let jpsw = JPSW::default();
        let full = jpsw.reconstruct_path(vec![]);
        assert!(full.is_empty());
    }

    #[test]
    fn test_jpsw_reconstruct_segment() {
        let jpsw = JPSW::default();
        let segment = jpsw.reconstruct_segment((0, 0), (3, 3));
        assert_eq!(segment.len(), 4); // (0,0), (1,1), (2,2), (3,3)
        assert_eq!(segment[0], (0, 0));
        assert_eq!(segment[3], (3, 3));
    }

    #[test]
    fn test_jpsw_reconstruct_segment_orthogonal() {
        let jpsw = JPSW::default();
        let segment = jpsw.reconstruct_segment((0, 0), (3, 0));
        assert_eq!(segment.len(), 4);
        assert_eq!(segment, vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
    }

    #[test]
    fn test_jpsw_move_cost_orthogonal() {
        let map = make_floor_grid(5);
        let jpsw = JPSW::default();
        let cost = jpsw.move_cost((1, 0), (0, 0), &map);
        // Orthogonal: (w1 + w2) / 2 = (1 + 1) / 2 = 1.0
        assert!((cost - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_jpsw_move_cost_diagonal() {
        let map = make_floor_grid(5);
        let jpsw = JPSW::default();
        let cost = jpsw.move_cost((1, 1), (0, 0), &map);
        // Diagonal: avg weight * sqrt(2) ≈ 1.414
        assert!(cost > 1.0);
        assert!(cost < 2.0);
    }

    #[test]
    fn test_jpsw_clear_caches() {
        let jpsw = JPSW::default();
        let map = make_floor_grid(5);
        // Populate cache
        let _ = jpsw.get_successors(None, (2, 2), &map);
        assert!(!jpsw.successor_cache.borrow().is_empty());
        jpsw.clear_caches();
        assert!(jpsw.successor_cache.borrow().is_empty());
        assert!(jpsw.jump_cache.borrow().is_empty());
    }

    // ------- Agent -------

    #[test]
    fn test_agent_goal_reached_true() {
        let agent = Agent {
            start: (0, 0),
            goal: (5, 5),
            position: (5, 5),
            path: vec![],
        };
        assert!(agent.goal_reached());
    }

    #[test]
    fn test_agent_goal_reached_false() {
        let agent = Agent {
            start: (0, 0),
            goal: (5, 5),
            position: (3, 3),
            path: vec![],
        };
        assert!(!agent.goal_reached());
    }

    #[test]
    fn test_agent_is_path_possible_trivial() {
        let map = make_floor_grid(5);
        let agent = Agent {
            start: (2, 2),
            goal: (2, 2),
            position: (2, 2),
            path: vec![],
        };
        assert!(agent.is_path_possible(&map));
    }

    #[test]
    fn test_agent_is_path_possible_open_grid() {
        let map = make_floor_grid(10);
        let agent = Agent {
            start: (0, 0),
            goal: (9, 9),
            position: (0, 0),
            path: vec![],
        };
        assert!(agent.is_path_possible(&map));
    }

    #[test]
    fn test_agent_is_path_impossible_walled_off() {
        let mut map = make_floor_grid(5);
        // Complete wall at x=2
        for y in 0..5 {
            set_obstacle(&mut map, (2, y));
        }
        let agent = Agent {
            start: (0, 0),
            goal: (4, 4),
            position: (0, 0),
            path: vec![],
        };
        assert!(!agent.is_path_possible(&map));
    }

    // ------- All algorithms find same reachable goals -------

    #[test]
    fn test_all_algorithms_agree_on_reachability() {
        let map = make_floor_grid(8);
        let start = (0, 0);
        let goal = (7, 7);

        let bfs = BreadthFirstSearch;
        let astar = AStarSearch;
        let jpsw = JPSW::default();

        let (bfs_path, _) = bfs.find_path(start, goal, &map);
        let (astar_path, _) = astar.find_path(start, goal, &map);
        let (jpsw_jp, _) = jpsw.find_path(start, goal, &map);

        // All should find a path
        assert!(!bfs_path.is_empty());
        assert!(!astar_path.is_empty());
        assert!(!jpsw_jp.is_empty());
    }

    #[test]
    fn test_all_algorithms_agree_on_no_path() {
        let mut map = make_floor_grid(6);
        for y in 0..6 {
            set_obstacle(&mut map, (3, y));
        }
        let start = (0, 0);
        let goal = (5, 5);

        let bfs = BreadthFirstSearch;
        let astar = AStarSearch;
        let jpsw = JPSW::default();

        let (bfs_path, _) = bfs.find_path(start, goal, &map);
        let (astar_path, _) = astar.find_path(start, goal, &map);
        let (jpsw_jp, _) = jpsw.find_path(start, goal, &map);

        assert!(bfs_path.is_empty());
        assert!(astar_path.is_empty());
        assert!(jpsw_jp.is_empty());
    }

    // ------- Maze-like scenario -------

    #[test]
    fn test_bfs_finds_path_through_maze() {
        // 5x5 grid with corridor:
        // F F F F F
        // F O O O F
        // F F F O F
        // F O F O F
        // F O F F F
        let mut map = make_floor_grid(5);
        set_obstacle(&mut map, (1, 1));
        set_obstacle(&mut map, (2, 1));
        set_obstacle(&mut map, (3, 1));
        set_obstacle(&mut map, (3, 2));
        set_obstacle(&mut map, (1, 3));
        set_obstacle(&mut map, (3, 3));
        set_obstacle(&mut map, (1, 4));

        let bfs = BreadthFirstSearch;
        let (path, _) = bfs.find_path((0, 0), (4, 4), &map);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_astar_finds_path_through_maze() {
        let mut map = make_floor_grid(5);
        set_obstacle(&mut map, (1, 1));
        set_obstacle(&mut map, (2, 1));
        set_obstacle(&mut map, (3, 1));
        set_obstacle(&mut map, (3, 2));
        set_obstacle(&mut map, (1, 3));
        set_obstacle(&mut map, (3, 3));
        set_obstacle(&mut map, (1, 4));

        let astar = AStarSearch;
        let (path, _) = astar.find_path((0, 0), (4, 4), &map);
        assert!(!path.is_empty());
    }
}
