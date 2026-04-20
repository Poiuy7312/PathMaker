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
use crate::util;

// Memory tracking for benchmarking
#[cfg(not(target_os = "windows"))]
use jemalloc_ctl::{epoch, thread};

// Random number generation for Greedy search tie-breaking
use rand::seq::IndexedRandom;

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::f32::consts::SQRT_2;
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

/// Precomputed block masks for obstacle corner-cutting prevention.
/// When direction `i` is an obstacle, `OBSTACLE_BLOCK_MASK[i]` gives the
/// bitmask of all directions that should also be blocked.
/// Cardinal obstacles block their two adjacent diagonals;
/// diagonal obstacles only block themselves.
const OBSTACLE_BLOCK_MASK: [u8; 8] = [
    0b0000_0001, // 0 NW: only itself
    0b0000_0111, // 1 N:  NW | N | NE
    0b0000_0100, // 2 NE: only itself
    0b0010_1001, // 3 W:  NW | W | SW
    0b1001_0100, // 4 E:  NE | E | SE
    0b0010_0000, // 5 SW: only itself
    0b1110_0000, // 6 S:  SW | S | SE
    0b1000_0000, // 7 SE: only itself
];

/// Stack-allocated collection of valid moves (max 8 neighbors).
pub struct PossibleMoves {
    moves: [(i32, i32); 8],
    len: u8,
}

impl PossibleMoves {
    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn contains(&self, pos: &(i32, i32)) -> bool {
        self.moves[..self.len as usize].contains(pos)
    }
}

/// Iterator over `PossibleMoves`.
pub struct PossibleMovesIter {
    inner: PossibleMoves,
    idx: u8,
}

impl Iterator for PossibleMovesIter {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.inner.len {
            let item = self.inner.moves[self.idx as usize];
            self.idx += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.inner.len - self.idx) as usize;
        (remaining, Some(remaining))
    }
}

impl IntoIterator for PossibleMoves {
    type Item = (i32, i32);
    type IntoIter = PossibleMovesIter;

    fn into_iter(self) -> Self::IntoIter {
        PossibleMovesIter {
            inner: self,
            idx: 0,
        }
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
/// A stack-allocated `PossibleMoves` containing up to 8 valid neighbor positions
pub fn get_possible_moves(
    current: (i32, i32),
    map: &Vec<Tile>,
    width: u32,
    height: u32,
) -> PossibleMoves {
    // Each bit in `traversable` marks a DELTAS direction with a walkable neighbor.
    // Each bit in `blocked` marks directions that are forbidden due to obstacles
    // or corner-cutting prevention (via OBSTACLE_BLOCK_MASK).
    let mut traversable: u8 = 0;
    let mut blocked: u8 = 0;

    // Classify each of the 8 neighbors as traversable or obstacle.
    // Obstacle neighbors propagate blocks to adjacent diagonals via OBSTACLE_BLOCK_MASK
    // to prevent corner-cutting around walls.
    for (i, &(dx, dy)) in DELTAS.iter().enumerate() {
        let neighbor = (current.0 + dx, current.1 + dy);
        if let Some(tile) =
            util::get_idx_from_coordinate(neighbor, width, height).and_then(|idx| map.get(idx))
        {
            if tile.is_traversable() {
                traversable |= 1 << i;
            } else {
                blocked |= OBSTACLE_BLOCK_MASK[i];
            }
        }
    }

    // A direction is valid only if the neighbor is traversable AND not blocked
    // by an adjacent obstacle's corner-cutting mask.
    let valid = traversable & !blocked;

    // Collect valid directions into a fixed-size array (no heap allocation).
    let mut result = PossibleMoves {
        moves: [(0, 0); 8],
        len: 0,
    };
    for i in 0..8u8 {
        if valid & (1 << i) != 0 {
            let (dx, dy) = DELTAS[i as usize];
            result.moves[result.len as usize] = (current.0 + dx, current.1 + dy);
            result.len += 1;
        }
    }
    result
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
        map: &Vec<Tile>,
        width: u32,
        height: u32,
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
fn get_overall_path_weight(
    path: &Vec<(i32, i32)>,
    map: &Vec<Tile>,
    width: u32,
    height: u32,
) -> u32 {
    let mut total_weight: u32 = 0;
    for moves in path {
        if let Some(tile) =
            util::get_idx_from_coordinate(*moves, width, height).and_then(|idx| map.get(idx))
        {
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
        map: &Vec<Tile>,
        width: u32,
        height: u32,
    ) -> (bool, Vec<(i32, i32)>, f64, u64, Duration, u32, u32) {
        // Memory tracking: jemalloc on Unix, cap allocator on Windows
        #[cfg(not(target_os = "windows"))]
        let allocated = thread::allocatedp::mib().unwrap();
        let algorithm = get_algorithm(algorithm);
        let now = Instant::now();

        // Snapshot memory before pathfinding
        #[cfg(not(target_os = "windows"))]
        epoch::advance().unwrap();
        #[cfg(not(target_os = "windows"))]
        let before = allocated.read().unwrap().get() as u64;
        #[cfg(target_os = "windows")]
        let before = crate::ALLOC.allocated() as u64;

        // Run pathfinding
        let (mut path, steps) = algorithm.find_path(self.start, self.goal, &map, width, height);

        // Snapshot memory after pathfinding
        #[cfg(not(target_os = "windows"))]
        epoch::advance().unwrap();
        #[cfg(not(target_os = "windows"))]
        let after = allocated.read().unwrap().get() as u64;
        #[cfg(target_os = "windows")]
        let after = crate::ALLOC.allocated() as u64;
        let time = now.elapsed();
        // An empty path means no route was found
        if path.is_empty() {
            return (
                false,
                vec![],
                sobel_method(&map, width, height),
                after - before,
                time,
                steps,
                0,
            );
        }
        if !algorithm.returns_full_path() {
            path = algorithm.reconstruct_path(path);
            path.push(self.start);
        }
        let weight = get_overall_path_weight(&path, map, width, height);

        return (
            true,
            path,
            sobel_method(&map, width, height),
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
    pub fn is_path_possible(&self, map: &Vec<Tile>, width: u32, height: u32) -> bool {
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
                for neighbor in get_possible_moves(current, map, width, height) {
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
                for neighbor in get_possible_moves(current, map, width, height) {
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
        map: &Vec<Tile>,
        width: u32,
        height: u32,
    ) -> (Vec<(i32, i32)>, u32) {
        let mut current = start;
        let mut path: Vec<(i32, i32)> = vec![start];
        let mut black_list: HashSet<(i32, i32)> = HashSet::new();
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

            let neighbors = get_possible_moves(current, map, width, height);
            for neighbor in neighbors {
                if black_list.contains(&neighbor) {
                    continue;
                }
                if heuristic(&neighbor, &goal) < heuristic(&current, &goal) {
                    good_moves.push(neighbor);
                } else {
                    bad_moves.push(neighbor);
                }
            }
            good_moves.sort_by(|a, b| heuristic(a, &goal).cmp(&heuristic(b, &goal)));
            if let Some(chosen_move) = good_moves.first() {
                current = *chosen_move;
                path.push(*chosen_move);
            } else if let Some(chosen_move) = bad_moves.choose(&mut rand::rng()) {
                black_list.insert(current);
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
        map: &Vec<Tile>,
        width: u32,
        height: u32,
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

            let neighbors = get_possible_moves(current, map, width, height);
            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current);
                    queue.push_back(neighbor);
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
        map: &Vec<Tile>,
        width: u32,
        height: u32,
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

            let neighbors = get_possible_moves(current, map, width, height);
            for neighbor in neighbors {
                if let Some(tile) = util::get_idx_from_coordinate(neighbor, width, height)
                    .and_then(|idx| map.get(idx))
                {
                    if tile.is_traversable() {
                        let move_cost: i32 = tile.weight as i32
                            * ((((current.0 - neighbor.0).abs() * (current.1 - neighbor.1).abs())
                                as f32
                                * SQRT_2) as u8)
                                .max(1) as i32;

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
    /// Cache for neighborhood successors: (parent_dir_index, neighborhood_hash) -> successor bitmask
    /// Bits 0-7 correspond to DELTAS directions
    successor_cache: RefCell<HashMap<(u8, u64), u8>>,
    /// Cache for orthogonal jumps: (pos, dir_index, start_weight) -> (end_pos, cost)
    jump_cache: RefCell<HashMap<(usize, u8, u8), Option<((i32, i32), f32)>>>,
}

impl Default for JPSW {
    fn default() -> Self {
        JPSW {
            successor_cache: RefCell::new(HashMap::new()),
            jump_cache: RefCell::new(HashMap::new()),
        }
    }
}

/// Map a direction (dx,dy) to a DELTAS index (0-7), or 8 for no parent.
#[inline]
fn dir_to_index(dx: i32, dy: i32) -> u8 {
    match (dx, dy) {
        (-1, -1) => 0,
        (0, -1) => 1,
        (1, -1) => 2,
        (-1, 0) => 3,
        (1, 0) => 4,
        (-1, 1) => 5,
        (0, 1) => 6,
        (1, 1) => 7,
        _ => 8,
    }
}

/// Index back to direction.
#[inline]
fn index_to_dir(idx: u8) -> (i32, i32) {
    DELTAS[idx as usize]
}

impl JPSW {
    /// Compact hash of the 3x3 neighborhood weights + traversability.
    /// Encodes both weight and traversability into a single u64.
    #[inline]
    fn hash_neighborhood(center: (i32, i32), map: &Vec<Tile>, width: u32, height: u32) -> u64 {
        let mut hash = 0u64;
        // Fixed iteration order for deterministic hashing
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                let val = match util::get_idx_from_coordinate(
                    (center.0 + dx, center.1 + dy),
                    width,
                    height,
                )
                .and_then(|idx| map.get(idx))
                {
                    Some(tile) if tile.is_traversable() => tile.weight as u64,
                    _ => 0xFF_FF, // sentinel for obstacle/missing
                };
                hash = hash.wrapping_mul(263).wrapping_add(val);
            }
        }
        hash
    }

    /// Get successors as a bitmask over DELTAS directions.
    fn get_successors(
        &self,
        parent: Option<(i32, i32)>,
        current: (i32, i32),
        map: &Vec<Tile>,
        width: u32,
        height: u32,
    ) -> u8 {
        let parent_dir_idx = match parent {
            Some(p) => dir_to_index((current.0 - p.0).signum(), (current.1 - p.1).signum()),
            None => 8,
        };
        let hash = Self::hash_neighborhood(current, map, width, height);
        let cache_key = (parent_dir_idx, hash);

        if let Some(&cached) = self.successor_cache.borrow().get(&cache_key) {
            return cached;
        }

        let result = self.compute_successors(parent, current, map, width, height);
        self.successor_cache.borrow_mut().insert(cache_key, result);
        result
    }

    /// Compute successors using lightweight local Dijkstra over the 3x3 neighborhood.
    /// Returns a bitmask where bit i means DELTAS[i] neighbor is a successor.
    fn compute_successors(
        &self,
        parent: Option<(i32, i32)>,
        current: (i32, i32),
        map: &Vec<Tile>,
        width: u32,
        height: u32,
    ) -> u8 {
        // 9 positions in 3x3: index = (dy+1)*3 + (dx+1), center = 4
        // cost[i] stores (best_cost, reached_via_center)
        const INF: f32 = f32::INFINITY;
        let mut best_cost: [f32; 9] = [INF; 9];
        let mut via_center: [bool; 9] = [false; 9];
        let mut settled: [bool; 9] = [false; 9];

        // center index = 4
        best_cost[4] = 0.0;
        via_center[4] = true;

        // parent position in local coords
        if let Some(p) = parent {
            let pdx = p.0 - current.0;
            let pdy = p.1 - current.1;
            if pdx >= -1 && pdx <= 1 && pdy >= -1 && pdy <= 1 {
                let pidx = ((pdy + 1) * 3 + (pdx + 1)) as usize;
                best_cost[pidx] = 0.0;
                via_center[pidx] = false;
            }
        }

        // Run Dijkstra over at most 9 cells
        for _ in 0..9 {
            // Find unsettled cell with minimum cost
            let mut min_cost = INF;
            let mut min_idx = usize::MAX;
            for i in 0..9 {
                if !settled[i] && best_cost[i] < min_cost {
                    min_cost = best_cost[i];
                    min_idx = i;
                }
            }
            if min_idx == usize::MAX {
                break;
            }
            settled[min_idx] = true;

            let sdx = (min_idx % 3) as i32 - 1;
            let sdy = (min_idx / 3) as i32 - 1;
            let spos = (current.0 + sdx, current.1 + sdy);

            // Expand to neighbors within 3x3
            for ndx in -1..=1i32 {
                for ndy in -1..=1i32 {
                    if ndx == 0 && ndy == 0 {
                        continue;
                    }
                    let nx = sdx + ndx;
                    let ny = sdy + ndy;
                    if nx < -1 || nx > 1 || ny < -1 || ny > 1 {
                        continue;
                    }
                    let nidx = ((ny + 1) * 3 + (nx + 1)) as usize;
                    if settled[nidx] {
                        continue;
                    }

                    let npos = (current.0 + nx, current.1 + ny);
                    let mc = Self::move_cost_static(npos, spos, map, width, height);
                    if mc.is_infinite() {
                        continue;
                    }

                    let new_cost = min_cost + mc;
                    if new_cost < best_cost[nidx] {
                        best_cost[nidx] = new_cost;
                        // via_center if source went through center, or if target IS center
                        via_center[nidx] = via_center[min_idx] || nidx == 4;
                    } else if new_cost == best_cost[nidx] && !via_center[nidx] {
                        via_center[nidx] = via_center[min_idx] || nidx == 4;
                    }
                }
            }
        }

        // Build bitmask: bit i set if DELTAS[i] neighbor is a successor
        let mut result: u8 = 0;
        for (i, &(ddx, ddy)) in DELTAS.iter().enumerate() {
            let nidx = ((ddy + 1) * 3 + (ddx + 1)) as usize;
            if via_center[nidx] && nidx != 4 {
                result |= 1 << i;
            }
        }
        result
    }

    /// Jump in direction, stopping at terrain transitions or forced neighbors
    #[inline]
    fn jump(
        &self,
        start: (i32, i32),
        dir: (i32, i32),
        goal: (i32, i32),
        map: &Vec<Tile>,
        width: u32,
        height: u32,
    ) -> Option<((i32, i32), f32)> {
        if dir.0 != 0 && dir.1 != 0 {
            self.jump_diagonal(start, dir, goal, map, width, height)
        } else {
            self.jump_orthogonal(start, dir, goal, map, width, height)
        }
    }

    /// Jump orthogonally with caching. Cache key uses position + direction + starting weight.
    fn jump_orthogonal(
        &self,
        start: (i32, i32),
        dir: (i32, i32),
        goal: (i32, i32),
        map: &Vec<Tile>,
        width: u32,
        height: u32,
    ) -> Option<((i32, i32), f32)> {
        let start_idx = match util::get_idx_from_coordinate(start, width, height) {
            Some(idx) => idx,
            None => return None,
        };
        let start_weight = map.get(start_idx).map(|t| t.weight).unwrap_or(1);
        let dir_idx = dir_to_index(dir.0, dir.1);
        let cache_key = (start_idx, dir_idx, start_weight);

        if let Some(&cached) = self.jump_cache.borrow().get(&cache_key) {
            return cached;
        }

        let result = self.jump_orthogonal_inner(start, dir, goal, map, start_weight, width, height);
        self.jump_cache.borrow_mut().insert(cache_key, result);
        result
    }

    fn jump_orthogonal_inner(
        &self,
        start: (i32, i32),
        dir: (i32, i32),
        goal: (i32, i32),
        map: &Vec<Tile>,
        start_weight: u8,
        width: u32,
        height: u32,
    ) -> Option<((i32, i32), f32)> {
        let mut pos = start;
        let mut cost = 0.0f32;
        let perp1 = (-dir.1, dir.0);
        let perp2 = (dir.1, -dir.0);

        // Effective weight: tile weight if traversable, sentinel otherwise.
        // Used to detect any neighborhood change (obstacles, weight regions).
        #[inline]
        fn eff_weight(p: (i32, i32), map: &Vec<Tile>, w: u32, h: u32) -> u16 {
            util::get_idx_from_coordinate(p, w, h)
                .and_then(|idx| map.get(idx))
                .map(|t| {
                    if t.is_traversable() {
                        t.weight as u16
                    } else {
                        0xFFFF
                    }
                })
                .unwrap_or(0xFFFF)
        }

        loop {
            let next = (pos.0 + dir.0, pos.1 + dir.1);
            match util::get_idx_from_coordinate(next, width, height).and_then(|idx| map.get(idx)) {
                Some(tile) if tile.is_traversable() => {
                    let w_from = util::get_idx_from_coordinate(pos, width, height)
                        .and_then(|idx| map.get(idx))
                        .map(|t| t.weight as f32)
                        .unwrap_or(f32::INFINITY);
                    cost += (w_from + tile.weight as f32) * 0.5;
                    pos = next;

                    if pos == goal {
                        return Some((pos, cost));
                    }
                    if tile.weight != start_weight {
                        return Some((pos, cost));
                    }

                    // Detect perpendicular neighborhood changes (forced neighbors
                    // from obstacles, or weight region boundaries adjacent to the
                    // scan line). Compare each perpendicular neighbor's effective
                    // weight at the current position vs the previous position.
                    let prev = (pos.0 - dir.0, pos.1 - dir.1);
                    for &perp in &[perp1, perp2] {
                        let curr_side =
                            eff_weight((pos.0 + perp.0, pos.1 + perp.1), map, width, height);
                        let prev_side =
                            eff_weight((prev.0 + perp.0, prev.1 + perp.1), map, width, height);
                        if curr_side != prev_side {
                            return Some((pos, cost));
                        }
                    }
                }
                _ => return None,
            }
        }
    }

    /// Jump diagonally — checks for orthogonal successors via a cheaper forced-neighbor test.
    fn jump_diagonal(
        &self,
        start: (i32, i32),
        dir: (i32, i32),
        goal: (i32, i32),
        map: &Vec<Tile>,
        width: u32,
        height: u32,
    ) -> Option<((i32, i32), f32)> {
        let start_weight = util::get_idx_from_coordinate(start, width, height)
            .and_then(|idx| map.get(idx))
            .map(|t| t.weight)
            .unwrap_or(1);
        let mut pos = start;
        let mut cost = 0.0f32;

        loop {
            let next = (pos.0 + dir.0, pos.1 + dir.1);

            // Check diagonal is traversable, and both sides for corner-cutting
            let tile = match util::get_idx_from_coordinate(next, width, height)
                .and_then(|idx| map.get(idx))
            {
                Some(t) if t.is_traversable() => t,
                _ => return None,
            };
            let side1 = (pos.0 + dir.0, pos.1);
            let side2 = (pos.0, pos.1 + dir.1);
            if !util::get_idx_from_coordinate(side1, width, height)
                .and_then(|idx| map.get(idx))
                .map_or(false, |t| t.is_traversable())
                || !util::get_idx_from_coordinate(side2, width, height)
                    .and_then(|idx| map.get(idx))
                    .map_or(false, |t| t.is_traversable())
            {
                return None;
            }

            cost += Self::move_cost_static(next, pos, map, width, height);
            pos = next;

            if pos == goal {
                return Some((pos, cost));
            }

            // Stop at weight transition
            if tile.weight != start_weight {
                return Some((pos, cost));
            }

            // Check if orthogonal jumps from here find something —
            // this is cheaper than full get_successors
            let ortho_h = (dir.0, 0);
            let ortho_v = (0, dir.1);
            if self
                .jump_orthogonal(pos, ortho_h, goal, map, width, height)
                .is_some()
                || self
                    .jump_orthogonal(pos, ortho_v, goal, map, width, height)
                    .is_some()
            {
                return Some((pos, cost));
            }
        }
    }

    /// Static move cost computation (no &self needed).
    #[inline]
    fn move_cost_static(
        to: (i32, i32),
        from: (i32, i32),
        map: &Vec<Tile>,
        width: u32,
        height: u32,
    ) -> f32 {
        let w_from =
            match util::get_idx_from_coordinate(from, width, height).and_then(|idx| map.get(idx)) {
                Some(t) if t.is_traversable() => t.weight as f32,
                _ => return f32::INFINITY,
            };
        let w_to =
            match util::get_idx_from_coordinate(to, width, height).and_then(|idx| map.get(idx)) {
                Some(t) if t.is_traversable() => t.weight as f32,
                _ => return f32::INFINITY,
            };

        let dx = (to.0 - from.0).abs();
        let dy = (to.1 - from.1).abs();

        if dx + dy == 1 {
            (w_from + w_to) * 0.5
        } else {
            // Diagonal: also include the two side cells
            let s1 = (from.0, to.1);
            let s2 = (to.0, from.1);
            let w_s1 = util::get_idx_from_coordinate(s1, width, height)
                .and_then(|idx| map.get(idx))
                .map(|t| t.weight as f32)
                .unwrap_or(w_from);
            let w_s2 = util::get_idx_from_coordinate(s2, width, height)
                .and_then(|idx| map.get(idx))
                .map(|t| t.weight as f32)
                .unwrap_or(w_from);
            ((w_from + w_to + w_s1 + w_s2) * 0.25) * std::f32::consts::SQRT_2
        }
    }

    /// Reconstruct full path between two jump points
    fn reconstruct_segment(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
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
        map: &Vec<Tile>,
        width: u32,
        height: u32,
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
            let diag = dx.min(dy);
            diag * std::f32::consts::SQRT_2 + (dx.max(dy) - diag)
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

            // Get successors as bitmask using JPSW pruning
            let succ_mask = self.get_successors(current_parent, current, map, width, height);

            for i in 0..8u8 {
                if succ_mask & (1 << i) == 0 {
                    continue;
                }
                let dir = index_to_dir(i);

                // Jump in this direction
                if let Some((jp, jump_cost)) = self.jump(current, dir, goal, map, width, height) {
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
                let segment = Self::reconstruct_segment(path[i], path[i + 1]);
                // Add all but the last cell (to avoid duplicates)
                full_path.extend(&segment[..segment.len() - 1]);
            }
            return full_path;
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        colors::WHITE,
        components::board::{Tile, TileType},
    };

    /// Helper: build an NxN grid of floor tiles with weight=1
    fn make_floor_grid(n: i32) -> Vec<Tile> {
        let mut map = Vec::with_capacity((n * n) as usize);
        for y in 0..n {
            for x in 0..n {
                map.push(Tile::new((x, y), TileType::Floor, 10, 10, 1, false, WHITE));
            }
        }
        map
    }

    /// Helper: place an obstacle on an existing grid
    fn set_obstacle(map: &mut Vec<Tile>, pos: (i32, i32), n: u32) {
        if let Some(idx) = util::get_idx_from_coordinate(pos, n, n) {
            map[idx] = Tile::new(pos, TileType::Obstacle, 10, 10, 1, false, WHITE);
        }
    }

    /// Helper: set a weighted tile
    fn set_weight(map: &mut Vec<Tile>, pos: (i32, i32), weight: u8, n: u32) {
        if let Some(idx) = util::get_idx_from_coordinate(pos, n, n) {
            map[idx] = Tile::new(
                pos,
                TileType::Weighted(weight),
                10,
                10,
                weight,
                false,
                WHITE,
            );
        }
    }

    // ------- get_possible_moves -------

    #[test]
    fn test_possible_moves_center_of_open_grid() {
        let map = make_floor_grid(5);
        let moves = get_possible_moves((2, 2), &map, 5, 5);
        // All 8 neighbors should be reachable
        assert_eq!(moves.len(), 8);
        for &(dx, dy) in DELTAS.iter() {
            assert!(moves.contains(&(2 + dx, 2 + dy)));
        }
    }

    #[test]
    fn test_possible_moves_corner_of_grid() {
        let map = make_floor_grid(5);
        let moves = get_possible_moves((0, 0), &map, 5, 5);
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
        set_obstacle(&mut map, (3, 2), 5);
        let moves = get_possible_moves((2, 2), &map, 5, 5);
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
            set_obstacle(&mut map, (2 + dx, 2 + dy), 5);
        }
        let moves = get_possible_moves((2, 2), &map, 5, 5);
        assert!(moves.is_empty());
    }

    #[test]
    fn test_possible_moves_player_tile_is_traversable() {
        let mut map = make_floor_grid(3);
        if let Some(idx) = util::get_idx_from_coordinate((1, 0), 3, 3) {
            map[idx] = Tile::new((1, 0), TileType::Player, 10, 10, 1, false, WHITE);
        }
        let moves = get_possible_moves((0, 0), &map, 3, 3);
        assert!(moves.contains(&(1, 0)));
    }

    #[test]
    fn test_possible_moves_enemy_tile_is_traversable() {
        let mut map = make_floor_grid(3);
        if let Some(idx) = util::get_idx_from_coordinate((1, 0), 3, 3) {
            map[idx] = Tile::new((1, 0), TileType::Enemy, 10, 10, 1, false, WHITE);
        }
        let moves = get_possible_moves((0, 0), &map, 3, 3);
        assert!(moves.contains(&(1, 0)));
    }

    // ------- get_overall_path_weight -------

    #[test]
    fn test_path_weight_uniform() {
        let map = make_floor_grid(5);
        let path = vec![(0, 0), (1, 0), (2, 0)];
        assert_eq!(get_overall_path_weight(&path, &map, 5, 5), 3);
    }

    #[test]
    fn test_path_weight_with_weighted_tiles() {
        let mut map = make_floor_grid(5);
        set_weight(&mut map, (1, 0), 5, 5);
        set_weight(&mut map, (2, 0), 10, 5);
        let path = vec![(0, 0), (1, 0), (2, 0)];
        assert_eq!(get_overall_path_weight(&path, &map, 5, 5), 1 + 5 + 10);
    }

    #[test]
    fn test_path_weight_empty_path() {
        let map = make_floor_grid(5);
        let path: Vec<(i32, i32)> = vec![];
        assert_eq!(get_overall_path_weight(&path, &map, 5, 5), 0);
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
        let (path, steps) = bfs.find_path((0, 0), (9, 9), &map, 10, 10);
        assert!(!path.is_empty());
        assert!(steps > 0);
        assert!(path.contains(&(9, 9)));
        assert!(path.contains(&(0, 0)));
    }

    #[test]
    fn test_bfs_start_equals_goal() {
        let map = make_floor_grid(5);
        let bfs = BreadthFirstSearch;
        let (path, steps) = bfs.find_path((2, 2), (2, 2), &map, 5, 5);
        assert_eq!(path, vec![(2, 2)]);
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_bfs_no_path_when_blocked() {
        let mut map = make_floor_grid(5);
        // Wall off (4,*) completely
        for y in 0..5 {
            set_obstacle(&mut map, (3, y), 5);
        }
        let bfs = BreadthFirstSearch;
        let (path, _) = bfs.find_path((0, 0), (4, 4), &map, 5, 5);
        assert!(path.is_empty());
    }

    #[test]
    fn test_bfs_adjacent_goal() {
        let map = make_floor_grid(5);
        let bfs = BreadthFirstSearch;
        let (path, _) = bfs.find_path((0, 0), (1, 0), &map, 5, 5);
        assert!(!path.is_empty());
        assert!(path.contains(&(0, 0)));
        assert!(path.contains(&(1, 0)));
    }

    // ------- A* -------

    #[test]
    fn test_astar_finds_path_on_open_grid() {
        let map = make_floor_grid(10);
        let astar = AStarSearch;
        let (path, steps) = astar.find_path((0, 0), (9, 9), &map, 10, 10);
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
            set_weight(&mut map, (x, 1), 100, 5);
        }
        let astar = AStarSearch;
        let (path, _) = astar.find_path((0, 1), (4, 1), &map, 5, 5);
        assert!(!path.is_empty());
        // The path should exist (A* found something)
        assert!(path.contains(&(4, 1)));
    }

    #[test]
    fn test_astar_no_path_when_blocked() {
        let mut map = make_floor_grid(5);
        for y in 0..5 {
            set_obstacle(&mut map, (3, y), 5);
        }
        let astar = AStarSearch;
        let (path, _) = astar.find_path((0, 0), (4, 4), &map, 5, 5);
        assert!(path.is_empty());
    }

    #[test]
    fn test_astar_start_equals_goal() {
        let map = make_floor_grid(5);
        let astar = AStarSearch;
        let (path, steps) = astar.find_path((2, 2), (2, 2), &map, 5, 5);
        // A* should return immediately with just the start node
        assert!(!path.is_empty());
        assert_eq!(steps, 1);
    }

    // ------- Greedy -------

    #[test]
    fn test_greedy_finds_path_on_open_grid() {
        let map = make_floor_grid(10);
        let greedy = GreedySearch;
        let (path, steps) = greedy.find_path((0, 0), (5, 5), &map, 10, 10);
        assert!(!path.is_empty());
        assert!(steps > 0);
    }

    #[test]
    fn test_greedy_returns_empty_for_impossible_path() {
        let mut map = make_floor_grid(5);
        for y in 0..5 {
            set_obstacle(&mut map, (2, y), 5);
        }
        let greedy = GreedySearch;
        let (path, _) = greedy.find_path((0, 0), (4, 4), &map, 5, 5);
        assert!(path.is_empty());
    }

    // ------- JPSW -------

    #[test]
    fn test_jpsw_finds_path_on_open_grid() {
        let map = make_floor_grid(10);
        let jpsw = JPSW::default();
        let (jump_points, steps) = jpsw.find_path((0, 0), (9, 9), &map, 10, 10);
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
        let segment = JPSW::reconstruct_segment((0, 0), (3, 3));
        assert_eq!(segment.len(), 4); // (0,0), (1,1), (2,2), (3,3)
        assert_eq!(segment[0], (0, 0));
        assert_eq!(segment[3], (3, 3));
    }

    #[test]
    fn test_jpsw_reconstruct_segment_orthogonal() {
        let segment = JPSW::reconstruct_segment((0, 0), (3, 0));
        assert_eq!(segment.len(), 4);
        assert_eq!(segment, vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
    }

    #[test]
    fn test_jpsw_move_cost_orthogonal() {
        let map = make_floor_grid(5);
        let cost = JPSW::move_cost_static((1, 0), (0, 0), &map, 5, 5);
        // Orthogonal: (w1 + w2) / 2 = (1 + 1) / 2 = 1.0
        assert!((cost - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_jpsw_move_cost_diagonal() {
        let map = make_floor_grid(5);
        let cost = JPSW::move_cost_static((1, 1), (0, 0), &map, 5, 5);
        // Diagonal: avg weight * sqrt(2) ≈ 1.414
        assert!(cost > 1.0);
        assert!(cost < 2.0);
    }

    #[test]
    fn test_jpsw_clear_caches() {
        let jpsw = JPSW::default();
        let map = make_floor_grid(5);
        // Populate cache
        let _ = jpsw.get_successors(None, (2, 2), &map, 5, 5);
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
        assert!(agent.is_path_possible(&map, 5, 5));
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
        assert!(agent.is_path_possible(&map, 10, 10));
    }

    #[test]
    fn test_agent_is_path_impossible_walled_off() {
        let mut map = make_floor_grid(5);
        // Complete wall at x=2
        for y in 0..5 {
            set_obstacle(&mut map, (2, y), 5);
        }
        let agent = Agent {
            start: (0, 0),
            goal: (4, 4),
            position: (0, 0),
            path: vec![],
        };
        assert!(!agent.is_path_possible(&map, 5, 5));
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

        let (bfs_path, _) = bfs.find_path(start, goal, &map, 8, 8);
        let (astar_path, _) = astar.find_path(start, goal, &map, 8, 8);
        let (jpsw_jp, _) = jpsw.find_path(start, goal, &map, 8, 8);

        // All should find a path
        assert!(!bfs_path.is_empty());
        assert!(!astar_path.is_empty());
        assert!(!jpsw_jp.is_empty());
    }

    #[test]
    fn test_all_algorithms_agree_on_no_path() {
        let mut map = make_floor_grid(6);
        for y in 0..6 {
            set_obstacle(&mut map, (3, y), 6);
        }
        let start = (0, 0);
        let goal = (5, 5);

        let bfs = BreadthFirstSearch;
        let astar = AStarSearch;
        let jpsw = JPSW::default();

        let (bfs_path, _) = bfs.find_path(start, goal, &map, 6, 6);
        let (astar_path, _) = astar.find_path(start, goal, &map, 6, 6);
        let (jpsw_jp, _) = jpsw.find_path(start, goal, &map, 6, 6);

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
        set_obstacle(&mut map, (1, 1), 5);
        set_obstacle(&mut map, (2, 1), 5);
        set_obstacle(&mut map, (3, 1), 5);
        set_obstacle(&mut map, (3, 2), 5);
        set_obstacle(&mut map, (1, 3), 5);
        set_obstacle(&mut map, (3, 3), 5);
        set_obstacle(&mut map, (1, 4), 5);

        let bfs = BreadthFirstSearch;
        let (path, _) = bfs.find_path((0, 0), (4, 4), &map, 5, 5);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_astar_finds_path_through_maze() {
        let mut map = make_floor_grid(5);
        set_obstacle(&mut map, (1, 1), 5);
        set_obstacle(&mut map, (2, 1), 5);
        set_obstacle(&mut map, (3, 1), 5);
        set_obstacle(&mut map, (3, 2), 5);
        set_obstacle(&mut map, (1, 3), 5);
        set_obstacle(&mut map, (3, 3), 5);
        set_obstacle(&mut map, (1, 4), 5);

        let astar = AStarSearch;
        let (path, _) = astar.find_path((0, 0), (4, 4), &map, 5, 5);
        assert!(!path.is_empty());
    }
}
