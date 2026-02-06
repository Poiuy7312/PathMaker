use std::collections::HashMap;

use crate::components::board::{self, Board, Tile};

fn calc_wcf(grid: &HashMap<(i32, i32), Tile>) {}

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
