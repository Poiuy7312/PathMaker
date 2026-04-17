//! # Benchmarks Module
//!
//! This module provides data structures and utilities for measuring and
//! storing pathfinding performance metrics.
//!
//! ## Metrics Collected
//! - **WCF (Weighted Complexity Factor)**: Measures terrain complexity using Sobel edge detection
//! - **Memory**: Bytes allocated during pathfinding
//! - **Time**: Duration of the pathfinding operation
//! - **Steps**: Number of nodes expanded by the algorithm
//! - **Path Cost**: Total weight of the resulting path
//!
//! ## Data Persistence
//! Benchmark results are serialized to JSON with both individual run data
//! and computed aggregates (averages and totals).

use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, fmt, thread};

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::components::board::{Board, Tile, TileType};
use crate::pathfinding::Agent;
use crate::util;

/// Container for pathfinding benchmark data.
///
/// Stores raw data from multiple pathfinding runs and provides methods
/// for computing aggregate statistics. Custom serialization includes
/// both raw data and computed averages/totals.
#[derive(Clone, Deserialize)]
pub struct PathData {
    /// Weighted Complexity Factor values for each run
    pub wcf: Vec<f64>,
    /// Memory allocated (bytes) for each run
    pub memory: Vec<u64>,
    /// Time taken for each run
    pub time: Vec<Duration>,
    /// Nodes expanded for each run
    pub steps: Vec<u32>,
    /// Total path cost for each run
    pub path_cost: Vec<u32>,
}

/// Custom serialization for PathData that includes computed aggregates.
///
/// The JSON output includes:
/// - Raw data arrays (wcf, memory, time, steps, path_cost)
/// - Average values (avg_*)
/// - Total values (total_*)
impl Serialize for PathData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PathData", 14)?;
        // Raw data
        state.serialize_field("wcf", &self.wcf)?;
        state.serialize_field("memory", &self.memory)?;
        state.serialize_field("time", &self.time)?;
        state.serialize_field("steps", &self.steps)?;
        state.serialize_field("path_cost", &self.path_cost)?;
        // Computed averages
        state.serialize_field("avg_wcf", &self.avg_wcf())?;
        state.serialize_field("avg_memory", &self.avg_memory())?;
        state.serialize_field("avg_time", &self.avg_time())?;
        state.serialize_field("avg_steps", &self.avg_steps())?;
        state.serialize_field("avg_path_cost", &self.avg_path_cost())?;
        // Totals
        state.serialize_field("total_memory", &self.total_memory())?;
        state.serialize_field("total_time", &self.total_time())?;
        state.serialize_field("total_steps", &self.total_steps())?;
        state.serialize_field("total_path_cost", &self.total_path_cost())?;
        state.end()
    }
}

impl PathData {
    /// Add a new set of benchmark measurements.
    ///
    /// # Arguments
    /// * `wcf` - Weighted Complexity Factor
    /// * `memory` - Memory allocated in bytes
    /// * `time` - Duration of the pathfinding
    /// * `steps` - Nodes expanded
    /// * `path_cost` - Total path weight
    pub fn update_all(
        &mut self,
        wcf: f64,
        memory: u64,
        time: Duration,
        steps: u32,
        path_cost: u32,
    ) {
        self.wcf.push(wcf);
        self.memory.push(memory);
        self.time.push(time);
        self.steps.push(steps);
        self.path_cost.push(path_cost);
    }

    /// Calculate average WCF across all runs.
    pub fn avg_wcf(&self) -> f64 {
        self.wcf.iter().sum::<f64>() / self.wcf.len() as f64
    }

    /// Calculate average memory usage across all runs.
    pub fn avg_memory(&self) -> u64 {
        self.memory.iter().sum::<u64>() / self.wcf.len().max(1) as u64
    }

    /// Calculate average steps (nodes expanded) across all runs.
    pub fn avg_steps(&self) -> u32 {
        self.steps.iter().sum::<u32>() / self.wcf.len().max(1) as u32
    }

    /// Calculate average execution time across all runs.
    pub fn avg_time(&self) -> Duration {
        self.time.iter().sum::<Duration>() / self.time.len().max(1) as u32
    }

    /// Calculate average path cost across all runs.
    pub fn avg_path_cost(&self) -> u32 {
        self.path_cost.iter().sum::<u32>() / self.path_cost.len().max(1) as u32
    }

    /// Calculate total memory used across all runs.
    pub fn total_memory(&self) -> u64 {
        self.memory.iter().sum::<u64>()
    }

    /// Calculate total steps across all runs.
    pub fn total_steps(&self) -> u32 {
        self.steps.iter().sum::<u32>()
    }

    /// Calculate total execution time across all runs.
    pub fn total_time(&self) -> Duration {
        self.time.iter().sum::<Duration>()
    }

    /// Calculate total path cost across all runs.
    pub fn total_path_cost(&self) -> u32 {
        self.path_cost.iter().sum::<u32>()
    }
}

/// Display implementation for human-readable benchmark output.
impl fmt::Display for PathData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, " Avg WCF: {}\n Avg Memory: {}\n Avg Steps: {}\n Avg Time: {:#?} \n Avg PathCost: {}\n Total Memory: {}\n Total Steps: {}\n Total Time: {:#?} \n Total PathCost: {}", self.avg_wcf(), self.avg_memory(), self.avg_steps(), self.avg_time(),self.avg_path_cost(),self.total_memory(),self.total_steps(),self.total_time(),self.total_path_cost())
    }
}

/// Calculate terrain complexity using the Sobel edge detection method.
///
/// This function computes a Weighted Complexity Factor (WCF) that measures
/// how varied the terrain weights are across the grid. Higher values indicate
/// more uniform terrain, while lower values indicate more varied/complex terrain.
///
/// ## Algorithm
/// 1. For each floor tile, apply Sobel X and Y kernels to the 3x3 neighborhood
/// 2. Compute gradient magnitude G = sqrt(Gx² + Gy²)
/// 3. Accumulate complexity factor: C += 1 - WEIGHT_FACTOR × log₂(G)
/// 4. Normalize by number of traversable tiles
///
/// ## Sobel Kernels
/// The Sobel operator detects edges/gradients in the weight distribution:
/// - X kernel detects horizontal weight changes
/// - Y kernel detects vertical weight changes
///
/// # Arguments
/// * `grid` - Reference to the tile map
///
/// # Returns
/// Normalized complexity factor (0.0 to 1.0, higher = more uniform)
pub fn sobel_method(grid: &Vec<Tile>, width: u32, height: u32) -> f64 {
    // Sobel kernels for edge detection in X and Y directions
    const X_KERNEL: [[i32; 3]; 3] = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
    const Y_KERNEL: [[i32; 3]; 3] = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];

    /// Weight factor for complexity calculation
    const WEIGHT_FACTOR: f64 = 0.01;

    let traversable_count = grid.iter().filter(|a| a.is_traversable()).count() as f64;

    let mut c_value: f64 = 0.0;

    for idx in 0..grid.len() {
        let c = (idx % width as usize) as i32;
        let r = (idx / width as usize) as i32;

        if !grid[idx].is_traversable() {
            continue;
        }

        let mut weight_conv_x = 0;
        let mut weight_conv_y = 0;

        // Inline the neighbor offsets to avoid allocations
        let deltas = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (0, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];

        for (di, &(dc, dr)) in deltas.iter().enumerate() {
            let neighbor_pos = (c + dc, r + dr);
            if let Some(tile) =
                util::get_idx_from_coordinate(neighbor_pos, width, height).and_then(|i| grid.get(i))
            {
                if tile.is_floor() {
                    let col = di % 3;
                    let row = di / 3;
                    let weight = tile.weight as i32;
                    weight_conv_x += weight * X_KERNEL[row][col];
                    weight_conv_y += weight * Y_KERNEL[row][col];
                }
            }
        }

        let g_value =
            ((weight_conv_x * weight_conv_x + weight_conv_y * weight_conv_y) as f64).sqrt();
        if g_value != 0.0 {
            c_value += 1.0 - WEIGHT_FACTOR * g_value.log2();
        }
    }

    return c_value / traversable_count;
}

#[derive(Copy, Clone)]
/// Configuration for a single benchmark scenario.
pub struct BenchmarkConfig {
    /// Grid width/height in tiles
    pub grid_size: u32,
    /// Obstacle percentage (0-100)
    pub obstacle_pct: u32,
    /// Weighted tile percentage (0-100)
    pub weighted_pct: u32,
    /// Max weight value for weighted tiles
    pub weight_range: u8,
}

/// Returns a default set of benchmark configurations that sweep across
/// grid sizes, obstacle densities, weighted-tile densities, and weight ranges.
pub fn default_benchmark_configs() -> Vec<BenchmarkConfig> {
    let grid_sizes = [64, 128, 256, 512];
    let obstacle_pcts = [0, 25, 50];
    let weighted_pcts = [0, 25, 50, 100];
    let weight_ranges: [u8; 4] = [1, 10, 100, 255];

    let mut configs = Vec::new();
    for &gs in &grid_sizes {
        for &op in &obstacle_pcts {
            for (&wp, &wr) in weighted_pcts.iter().zip(weight_ranges.iter()) {
                configs.push(BenchmarkConfig {
                    grid_size: gs,
                    obstacle_pct: op,
                    weighted_pct: wp,
                    weight_range: wr,
                });
            }
        }
    }
    configs
}

/// Run benchmarks across multiple grid configurations and algorithms, writing results to CSV.
///
/// For each combination of (config, algorithm), generates `iterations` random grids,
/// runs pathfinding, and records per-run metrics to `output_path`.
///
/// # Arguments
/// * `configs` - Grid configurations to test
/// * `algorithms` - Algorithm names (e.g. "A* search", "Breadth First Search", "JPSW", "Greedy")
/// * `iterations` - Number of runs per (config, algorithm) pair
/// * `output_path` - Path to the output CSV file
pub fn run_overall_benchmark(
    configs: &[BenchmarkConfig],
    algorithms: &[&str],
    iterations: u32,
    output_path: &PathBuf,
) {
    let file = std::fs::File::create(output_path).expect("Failed to create CSV file");
    let mut wtr = csv::Writer::from_writer(file);

    wtr.write_record(&[
        "algorithm",
        "grid_size",
        "obstacle_pct",
        "weighted_pct",
        "weight_range",
        "run",
        "wcf",
        "memory_bytes",
        "time_ms",
        "steps",
        "path_cost",
    ])
    .expect("Failed to write CSV header");

    let num_cpus = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    for chunk in configs.chunks(num_cpus) {
        let mut handles = Vec::new();

        for config in chunk {
            let algorithms: Vec<String> = algorithms.iter().map(|s| s.to_string()).collect();
            let config = *config;
            let handle = thread::spawn(move || {
                let mut rows: Vec<[String; 11]> = Vec::new();
                for run in 0..iterations {
                    let mut board = Board {
                        location: sdl2::rect::Point::new(0, 0),
                        width: config.grid_size,
                        height: config.grid_size,
                        tile_amount_x: config.grid_size,
                        tile_amount_y: config.grid_size,
                        active: false,
                        id: String::from("benchmark_board"),
                        selected_piece_type: TileType::Obstacle,
                        cached_background: None,
                        cached_grid: std::cell::RefCell::new(None),
                        cached_texture: std::cell::RefCell::new(None),
                        texture_dirty: std::cell::RefCell::new(true),
                        multiple_agents: false,
                        multiple_goals: false,
                        agents: vec![],
                        starts: vec![0],
                        goals: vec![
                            ((config.grid_size - 1) * config.grid_size + (config.grid_size - 1))
                                as usize,
                        ],
                        updated_tiles: vec![],
                    };

                    /*board.generate_random_grid(
                        config.weight_range,
                        config.obstacle_pct as usize,
                        config.weighted_pct as usize,
                        true,
                    );*/

                    board.generate_organic_city(
                        0,
                        2,
                        config.weight_range as u32,
                        config.obstacle_pct as f32,
                        2,
                        config.weighted_pct as u32,
                        true,
                    );
                    for algorithm in &algorithms {
                        let grid = board.grid();

                        let start_coord = util::get_coordinate_from_idx(
                            board.starts[0],
                            board.tile_amount_x,
                            board.tile_amount_y,
                        );
                        let goal_coord = util::get_coordinate_from_idx(
                            board.goals[0],
                            board.tile_amount_x,
                            board.tile_amount_y,
                        );
                        let mut agent = Agent {
                            start: start_coord,
                            goal: goal_coord,
                            position: start_coord,
                            path: vec![],
                        };

                        if !agent.is_path_possible(&grid, board.tile_amount_x, board.tile_amount_y)
                        {
                            continue;
                        }

                        let (success, _path, wcf, memory, time, steps, path_cost) = agent.get_path(
                            algorithm,
                            &grid,
                            board.tile_amount_x,
                            board.tile_amount_y,
                        );

                        if !success || _path.is_empty() {
                            continue;
                        }

                        rows.push([
                            algorithm.to_string(),
                            config.grid_size.to_string(),
                            config.obstacle_pct.to_string(),
                            config.weighted_pct.to_string(),
                            config.weight_range.to_string(),
                            run.to_string(),
                            format!("{:.6}", wcf),
                            memory.to_string(),
                            format!("{:.4}", time.as_secs_f64() * 1000.0),
                            steps.to_string(),
                            path_cost.to_string(),
                        ]);
                    }
                }
                rows
            });
            handles.push(handle);
        }

        for handle in handles {
            if let Ok(rows) = handle.join() {
                for row in &rows {
                    wtr.write_record(row).expect("Failed to write CSV row");
                }
            }
        }
    }

    wtr.flush().expect("Failed to flush CSV writer");
    println!("Benchmark results written to {:#?}", output_path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        colors::WHITE,
        components::board::{Tile, TileType},
    };
    use std::time::Duration;

    fn make_empty_pathdata() -> PathData {
        PathData {
            wcf: vec![],
            memory: vec![],
            time: vec![],
            steps: vec![],
            path_cost: vec![],
        }
    }

    fn make_sample_pathdata() -> PathData {
        PathData {
            wcf: vec![0.8, 0.9, 1.0],
            memory: vec![1000, 2000, 3000],
            time: vec![
                Duration::from_millis(10),
                Duration::from_millis(20),
                Duration::from_millis(30),
            ],
            steps: vec![100, 200, 300],
            path_cost: vec![50, 60, 70],
        }
    }

    // ------- update_all -------

    #[test]
    fn test_update_all_adds_values() {
        let mut pd = make_empty_pathdata();
        pd.update_all(0.5, 1024, Duration::from_millis(100), 42, 10);
        assert_eq!(pd.wcf.len(), 1);
        assert_eq!(pd.memory.len(), 1);
        assert_eq!(pd.time.len(), 1);
        assert_eq!(pd.steps.len(), 1);
        assert_eq!(pd.path_cost.len(), 1);
        assert!((pd.wcf[0] - 0.5).abs() < 0.001);
        assert_eq!(pd.memory[0], 1024);
        assert_eq!(pd.steps[0], 42);
        assert_eq!(pd.path_cost[0], 10);
    }

    #[test]
    fn test_update_all_multiple_times() {
        let mut pd = make_empty_pathdata();
        for i in 0..5 {
            pd.update_all(
                i as f64,
                i as u64 * 100,
                Duration::from_millis(i as u64),
                i,
                i,
            );
        }
        assert_eq!(pd.wcf.len(), 5);
        assert_eq!(pd.memory.len(), 5);
    }

    // ------- Averages -------

    #[test]
    fn test_avg_wcf() {
        let pd = make_sample_pathdata();
        let avg = pd.avg_wcf();
        assert!((avg - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_avg_memory() {
        let pd = make_sample_pathdata();
        assert_eq!(pd.avg_memory(), 2000);
    }

    #[test]
    fn test_avg_steps() {
        let pd = make_sample_pathdata();
        assert_eq!(pd.avg_steps(), 200);
    }

    #[test]
    fn test_avg_time() {
        let pd = make_sample_pathdata();
        assert_eq!(pd.avg_time(), Duration::from_millis(20));
    }

    #[test]
    fn test_avg_path_cost() {
        let pd = make_sample_pathdata();
        assert_eq!(pd.avg_path_cost(), 60);
    }

    // ------- Totals -------

    #[test]
    fn test_total_memory() {
        let pd = make_sample_pathdata();
        assert_eq!(pd.total_memory(), 6000);
    }

    #[test]
    fn test_total_steps() {
        let pd = make_sample_pathdata();
        assert_eq!(pd.total_steps(), 600);
    }

    #[test]
    fn test_total_time() {
        let pd = make_sample_pathdata();
        assert_eq!(pd.total_time(), Duration::from_millis(60));
    }

    #[test]
    fn test_total_path_cost() {
        let pd = make_sample_pathdata();
        assert_eq!(pd.total_path_cost(), 180);
    }

    // ------- Display -------

    #[test]
    fn test_display_contains_avg_labels() {
        let pd = make_sample_pathdata();
        let display = format!("{}", pd);
        assert!(display.contains("Avg WCF"));
        assert!(display.contains("Avg Memory"));
        assert!(display.contains("Avg Steps"));
        assert!(display.contains("Avg Time"));
        assert!(display.contains("Avg PathCost"));
        assert!(display.contains("Total Memory"));
        assert!(display.contains("Total Steps"));
        assert!(display.contains("Total Time"));
        assert!(display.contains("Total PathCost"));
    }

    // ------- Serialization -------

    #[test]
    fn test_serialize_includes_computed_fields() {
        let pd = make_sample_pathdata();
        let json = serde_json::to_string(&pd).unwrap();
        assert!(json.contains("avg_wcf"));
        assert!(json.contains("avg_memory"));
        assert!(json.contains("avg_time"));
        assert!(json.contains("avg_steps"));
        assert!(json.contains("avg_path_cost"));
        assert!(json.contains("total_memory"));
        assert!(json.contains("total_time"));
        assert!(json.contains("total_steps"));
        assert!(json.contains("total_path_cost"));
    }

    #[test]
    fn test_serialize_includes_raw_data() {
        let pd = make_sample_pathdata();
        let json = serde_json::to_string(&pd).unwrap();
        assert!(json.contains("\"wcf\""));
        assert!(json.contains("\"memory\""));
        assert!(json.contains("\"time\""));
        assert!(json.contains("\"steps\""));
        assert!(json.contains("\"path_cost\""));
    }

    #[test]
    fn test_deserialize_pathdata() {
        let pd = make_sample_pathdata();
        let json = serde_json::to_string(&pd).unwrap();
        let loaded: PathData = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.wcf.len(), 3);
        assert_eq!(loaded.memory.len(), 3);
        assert_eq!(loaded.steps.len(), 3);
        assert_eq!(loaded.path_cost.len(), 3);
    }

    // ------- sobel_method -------

    #[test]
    fn test_sobel_uniform_grid() {
        // All floor tiles with same weight → no edges → high WCF
        let mut grid = Vec::new();
        for y in 0..10 {
            for x in 0..10 {
                grid.push(Tile::new((x, y), TileType::Floor, 10, 10, 1, false, WHITE));
            }
        }
        let wcf = sobel_method(&grid, 10, 10);
        // Uniform grid should have a defined value (no NaN)
        assert!(!wcf.is_nan());
    }

    #[test]
    fn test_sobel_varied_grid() {
        let mut grid = Vec::new();
        for y in 0..10 {
            for x in 0..10 {
                let weight = if x < 5 { 1 } else { 100 };
                grid.push(Tile::new(
                    (x, y),
                    TileType::Floor,
                    10,
                    10,
                    weight,
                    false,
                    WHITE,
                ));
            }
        }
        let wcf = sobel_method(&grid, 10, 10);
        assert!(!wcf.is_nan());
    }

    #[test]
    fn test_sobel_with_obstacles_excluded() {
        // Obstacles should be skipped, only floor tiles contribute
        let mut grid = Vec::new();
        for y in 0..5 {
            for x in 0..5 {
                if x == 2 && y == 2 {
                    grid.push(Tile::new(
                        (x, y),
                        TileType::Obstacle,
                        10,
                        10,
                        1,
                        false,
                        WHITE,
                    ));
                } else {
                    grid.push(Tile::new((x, y), TileType::Floor, 10, 10, 1, false, WHITE));
                }
            }
        }
        let wcf = sobel_method(&grid, 5, 5);
        assert!(!wcf.is_nan());
    }
}
