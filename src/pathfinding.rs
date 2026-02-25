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
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

const DELTAS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

fn get_diagonals(delta_location: (i32, i32)) -> (i32, i32) {
    if delta_location.0 != 0 && delta_location.1 == 0 {
        return (0, 1);
    } else if delta_location.0 == 0 && delta_location.1 != 0 {
        return (1, 0);
    } else {
        return (0, 0);
    }
}

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
    ) -> (Vec<(i32, i32)>, u32);

    fn returns_full_path(&self) -> bool;
    fn reconstruct_path(&self, path: Vec<(i32, i32)>) -> Vec<(i32, i32)>;

    /// Get the name of this algorithm (for debugging/UI)
    fn name(&self) -> &str;
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Agent {
    pub start: (i32, i32),
    pub goal: (i32, i32),
    pub position: (i32, i32),
    pub path: Vec<(i32, i32)>,
}

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
        let max_steps = map.len() * 10; // safeguard, adjust as needed
        while let Some(current) = queue.pop_front() {
            steps += 1;
            if steps > max_steps {
                break;
            }
            if current == goal {
                return true;
            }
            let neighbors = get_possible_moves(current, map);
            for neighbor in neighbors {
                // Only queue unvisited neighbors
                if visited.insert(neighbor) {
                    parent.insert(neighbor, current);
                    queue.push_back(neighbor);
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

pub struct GreedySearch;

impl PathfindingAlgorithm for GreedySearch {
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

/// Simple BFS pathfinding implementation
pub struct BreadthFirstSearch;

impl PathfindingAlgorithm for BreadthFirstSearch {
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

/// A* pathfinding implementation
pub struct AStarSearch;

impl PathfindingAlgorithm for AStarSearch {
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

pub struct JPSW {
    // Cache for neighborhood successors: (parent, current) -> set of successors
    successor_cache: RefCell<HashMap<(Option<(i32, i32)>, (i32, i32), u64), HashSet<(i32, i32)>>>,
    // Cache for orthogonal jumps: (pos, dir, terrain_hash) -> (end_pos, cost)
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
        for i in 0..path.len() - 1 {
            let segment = self.reconstruct_segment(path[i], path[i + 1]);
            // Add all but the last cell (to avoid duplicates)
            full_path.extend(&segment[..segment.len() - 1]);
        }
        println!("{:#?}", full_path);
        full_path
    }
}
