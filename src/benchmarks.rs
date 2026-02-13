use std::fmt;
use std::{collections::HashMap, time::Duration};

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::components::board::{self, Board, Tile};

#[derive(Clone, Deserialize)]
pub struct PathData {
    pub wcf: Vec<f64>,
    pub memory: Vec<u64>,
    pub time: Vec<Duration>,
    pub steps: Vec<u32>,
    pub path_cost: Vec<u32>,
}

impl Serialize for PathData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PathData", 14)?;
        state.serialize_field("wcf", &self.wcf)?;
        state.serialize_field("memory", &self.memory)?;
        state.serialize_field("time", &self.time)?;
        state.serialize_field("steps", &self.steps)?;
        state.serialize_field("path_cost", &self.path_cost)?;
        state.serialize_field("avg_wcf", &self.avg_wcf())?;
        state.serialize_field("avg_memory", &self.avg_memory())?;
        state.serialize_field("avg_time", &self.avg_time())?;
        state.serialize_field("avg_steps", &self.avg_steps())?;
        state.serialize_field("avg_path_cost", &self.avg_path_cost())?;
        state.serialize_field("total_memory", &self.total_memory())?;
        state.serialize_field("total_time", &self.total_time())?;
        state.serialize_field("total_steps", &self.total_steps())?;
        state.serialize_field("total_path_cost", &self.total_path_cost())?;
        state.end()
    }
}

impl PathData {
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

    pub fn avg_wcf(&self) -> f64 {
        self.wcf.iter().sum::<f64>() / self.wcf.len() as f64
    }
    pub fn avg_memory(&self) -> u64 {
        self.memory.iter().sum::<u64>() / self.wcf.len() as u64
    }
    pub fn avg_steps(&self) -> u32 {
        self.steps.iter().sum::<u32>() / self.wcf.len() as u32
    }

    pub fn avg_time(&self) -> Duration {
        self.time.iter().sum::<Duration>() / self.time.len() as u32
    }

    pub fn avg_path_cost(&self) -> u32 {
        self.path_cost.iter().sum::<u32>() / self.path_cost.len() as u32
    }

    pub fn total_memory(&self) -> u64 {
        self.memory.iter().sum::<u64>()
    }
    pub fn total_steps(&self) -> u32 {
        self.steps.iter().sum::<u32>()
    }

    pub fn total_time(&self) -> Duration {
        self.time.iter().sum::<Duration>()
    }

    pub fn total_path_cost(&self) -> u32 {
        self.path_cost.iter().sum::<u32>()
    }
}

impl fmt::Display for PathData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, " Avg WCF: {}\n Avg Memory: {}\n Avg Steps: {}\n Avg Time: {:#?} \n Avg PathCost: {}\n Total Memory: {}\n Total Steps: {}\n Total Time: {:#?} \n Total PathCost: {}", self.avg_wcf(), self.avg_memory(), self.avg_steps(), self.avg_time(),self.avg_path_cost(),self.total_memory(),self.total_steps(),self.total_time(),self.total_path_cost())
    }
}

pub fn sobel_method(grid: &HashMap<(i32, i32), Tile>) -> f64 {
    const X_KERNEL: [[i32; 3]; 3] = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
    const Y_KERNEL: [[i32; 3]; 3] = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];
    const WEIGHT_FACTOR: f64 = 0.01;

    let traversable_count = grid.values().filter(|a| a.is_floor()).count() as f64;

    let mut c_value: f64 = 0.0;

    // Iterate grid directly instead of using coordinates
    for (&(c, r), _) in grid.iter() {
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
        if let Some(tile) = grid.get(&(c, r)) {
            if !tile.is_floor() {
                continue;
            }
        }

        for (idx, &(dc, dr)) in deltas.iter().enumerate() {
            let neighbor_pos = (c + dc, r + dr);
            if let Some(tile) = grid.get(&neighbor_pos) {
                if tile.is_floor() {
                    let col = (idx % 3) as usize;
                    let row = (idx / 3) as usize;
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
