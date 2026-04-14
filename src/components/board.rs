//! # Board Component Module
//!
//! This module implements the main game board for pathfinding visualization.
//! The board is a grid of tiles that can be:
//! - Floor tiles (traversable with optional weight)
//! - Obstacle tiles (impassable)
//! - Player tiles (start positions)
//! - Enemy tiles (goal positions)
//!
//! ## Features
//! - Click-to-place tile editing
//! - Random and city-style map generation
//! - Pathfinding execution with multi-threaded agent support
//! - JSON serialization for save/load functionality

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fmt, fs, thread, u8};

extern crate sdl2;

use rand::seq::index::sample;
use rand::seq::IteratorRandom;
use rand::seq::SliceRandom;
use rand::{rng, Rng};
use sdl2::mouse::MouseState;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::benchmarks::PathData;
use crate::components::Component;
use crate::pathfinding::Agent;
use crate::{colors::*, fileDialog, settings, util};

/// Enumeration of possible tile types on the game board.
///
/// Each type has different behavior for pathfinding and rendering.
#[derive(Copy, Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum TileType {
    /// Impassable obstacle - blocks all movement
    Obstacle,
    /// Standard floor tile with weight 1
    Floor,
    /// Player/agent starting position (rendered green)
    Player,
    /// Enemy/goal position (rendered red)
    Enemy,
    /// Floor tile with custom traversal weight (higher = slower)
    Weighted(u8),
    /// Path tile (rendered blue)
    Path,
}

/// Represents a single tile on the game board.
///
/// Tiles maintain their position, type, dimensions, and rendering state.
/// The `dirty` flag is used for efficient redrawing - only dirty tiles are redrawn.
#[derive(Copy, Clone)]
pub struct Tile {
    /// Position on the grid (tile coordinates, not pixels)
    pub position: (i32, i32),
    /// Current type of this tile (obstacle, floor, player, etc.)
    tile_type: TileType,
    /// Height in pixels when rendered
    height: u32,
    /// Width in pixels when rendered
    width: u32,
    /// Traversal weight (1 = normal, higher = slower)
    pub weight: u8,
    /// Whether this tile needs to be redrawn
    dirty: bool,
    /// Cached rectangle for efficient rendering
    cached_rectangle: Option<Rect>,
    cached_color: Color,
}
impl Serialize for Tile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Tile", 6)?;
        state.serialize_field("position", &self.position)?;
        state.serialize_field("tile_type", &self.tile_type)?;
        state.serialize_field("height", &self.height)?;
        state.serialize_field("width", &self.width)?;
        state.serialize_field("weight", &self.weight)?;
        state.serialize_field("dirty", &self.dirty)?;
        state.serialize_field("cached_color", &self.cached_color.rgb())?;
        state.end()
    }
}

impl Tile {
    /// Create a new tile with the specified properties.
    ///
    /// # Arguments
    /// * `position` - Grid position (will be converted to pixel position)
    /// * `tile_type` - Type of tile
    /// * `height` - Tile height in pixels
    /// * `width` - Tile width in pixels
    /// * `weight` - Traversal weight
    /// * `dirty` - Initial dirty state
    pub(crate) fn new(
        position: (i32, i32),
        tile_type: TileType,
        height: u32,
        width: u32,
        weight: u8,
        dirty: bool,
        color: Color,
    ) -> Self {
        // Convert grid position to pixel position
        let position = (position.0 * width as i32, position.1 * height as i32);
        Tile {
            position,
            tile_type,
            height,
            width,
            dirty,
            cached_rectangle: None,
            cached_color: color,
            weight: weight.max(1),
        }
    }

    /// Calculate the screen rectangle for this tile.
    fn get_rect(&self, board_origin: Point) -> Rect {
        Rect::new(
            board_origin.x() + self.position.0,
            board_origin.y() + self.position.1,
            self.width,
            self.height,
        )
    }

    /// Draw this tile to the canvas.
    ///
    /// Colors tiles based on type:
    /// - Obstacle: Black
    /// - Floor: White (tinted based on weight if weighted tile)
    /// - Player: Green
    /// - Enemy: Red
    fn draw(&mut self, change_layout: bool, board_origin: Point, canvas: &mut Canvas<Window>) {
        if change_layout {
            self.cached_rectangle = None;
        } else if !self.dirty {
            #[cfg(target_os = "windows")]
            return;
        }
        let tile_rect = match self.cached_rectangle {
            Some(rect) => rect,
            None => self.get_rect(board_origin),
        };
        self.cached_rectangle = Some(tile_rect);

        let c = self.cached_color;

        canvas.set_draw_color(c);
        canvas.fill_rect(tile_rect).unwrap();

        self.dirty = false;
    }

    /// Check if this tile can be walked through.
    ///
    /// All non-obstacle tiles are traversable (floor, player, enemy, weighted).
    pub fn is_traversable(&self) -> bool {
        return self.tile_type != TileType::Obstacle;
    }

    /// Check if this tile is a standard floor tile.
    pub fn is_floor(&self) -> bool {
        return self.tile_type == TileType::Floor;
    }

    /// Change the tile's type and mark it as dirty for redraw.
    fn change_tile_type(&mut self, new_type: TileType) {
        if self.tile_type != new_type {
            self.tile_type = new_type;
            self.dirty = true;
            match new_type {
                TileType::Obstacle => self.cached_color = BLACK,
                TileType::Enemy => self.cached_color = RED,
                TileType::Player => self.cached_color = GREEN,
                TileType::Floor => self.cached_color = Self::calc_floor_color(self.weight),
                TileType::Path => self.cached_color = BLUE,
                TileType::Weighted(weight) => self.cached_color = Self::calc_floor_color(weight),
            }
        }
    }

    #[inline]
    fn calc_floor_color(weight: u8) -> Color {
        if weight > 1 {
            Color::RGB(255, 230 - (weight / 2) as u8, 255 - weight as u8)
        } else {
            WHITE
        }
    }
}

/// The main game board for pathfinding visualization.
///
/// Contains a grid of tiles and manages agent pathfinding.
/// Supports serialization for save/load functionality.
pub struct Board {
    /// Screen position of the board's top-left corner
    pub location: Point,
    /// Total height in pixels
    pub height: u32,
    /// Total width in pixels
    pub width: u32,
    /// Number of tiles horizontally
    pub tile_amount_x: u32,
    /// Number of tiles vertically
    pub tile_amount_y: u32,
    /// Currently selected tile type for editing
    pub selected_piece_type: TileType,
    /// Unique identifier
    pub id: String,
    /// List of player/agent starting positions
    pub starts: Vec<usize>,
    /// List of goal positions
    pub goals: Vec<usize>,
    /// Whether the board is interactive
    pub active: bool,
    /// Allow multiple agents simultaneously
    pub multiple_agents: bool,
    /// Allow multiple goal positions
    pub multiple_goals: bool,
    /// Active pathfinding agents
    pub agents: Vec<Agent>,
    /// Cached background rectangle
    pub cached_background: Option<Rect>,
    /// Cached tile grid (RefCell for interior mutability)
    pub cached_grid: RefCell<Option<Vec<Tile>>>,
    /// Cached board texture for efficient rendering
    pub cached_texture: RefCell<Option<Texture<'static>>>,
    /// Whether the board texture needs re-rendering
    pub texture_dirty: RefCell<bool>,
}

/// Deserialize a Board from JSON.
///
/// Reconstructs the grid from the serialized tile data.
impl<'de> Deserialize<'de> for Board {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        struct BoardData {
            height: u32,
            width: u32,
            tile_amount_x: u32,
            tile_amount_y: u32,
            starts: Vec<usize>,
            goals: Vec<usize>,
            multiple_agents: bool,
            multiple_goals: bool,
            tiles: Vec<[String; 4]>, // Array of [position, type, weight]
        }

        let data = BoardData::deserialize(deserializer)?;

        // Rebuild grid from tiles
        let mut grid = Vec::with_capacity((data.tile_amount_x * data.tile_amount_y) as usize);
        let tile_width = data.width / data.tile_amount_x;
        let tile_height = data.height / data.tile_amount_y;

        for tile_data in data.tiles {
            //println!("Yes");
            let pos_str = &tile_data[0];
            let type_str = &tile_data[1];
            let weight_str = &tile_data[2];
            let color_str = &tile_data[3];
            let color: Vec<&str> = color_str.split(",").collect();
            let parts: Vec<&str> = pos_str.split(',').collect();
            if parts.len() == 2 && color.len() == 3 {
                if let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                    let tile_type = match type_str.as_str() {
                        "Floor" => TileType::Floor,
                        "Obstacle" => TileType::Obstacle,
                        "Player" => TileType::Player,
                        "Enemy" => TileType::Enemy,
                        "Path" => TileType::Path,
                        _ => TileType::Floor,
                    };
                    let weight = weight_str.parse::<u8>().unwrap();

                    if let (Ok(r), Ok(g), Ok(b)) = (
                        color[0].parse::<u8>(),
                        color[1].parse::<u8>(),
                        color[2].parse::<u8>(),
                    ) {
                        // Saved positions are pixel coords; convert back to grid coords
                        // since Tile::new() multiplies by tile dimensions
                        let pos = (x / tile_width as i32, y / tile_height as i32);
                        grid.push(Tile::new(
                            pos,
                            tile_type,
                            tile_height,
                            tile_width,
                            weight,
                            false,
                            Color::RGB(r, g, b),
                        ));
                    }
                }
            }
        }

        Ok(Board {
            location: Point::new(0, 0),
            height: data.height,
            width: data.width,
            tile_amount_x: data.tile_amount_x,
            tile_amount_y: data.tile_amount_y,
            selected_piece_type: TileType::Obstacle,
            id: String::from("game_board"),
            active: true,
            multiple_agents: data.multiple_agents,
            multiple_goals: data.multiple_goals,
            cached_background: None,
            cached_grid: RefCell::new(Some(grid)),
            cached_texture: RefCell::new(None),
            texture_dirty: RefCell::new(false),
            agents: vec![],
            goals: data.goals,
            starts: data.starts,
        })
    }
}

/// Serialize the Board to JSON.
///
/// Converts the grid to a compact format suitable for storage.
impl Serialize for Board {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Board", 7)?;
        state.serialize_field("height", &self.height)?;
        state.serialize_field("width", &self.width)?;
        state.serialize_field("tile_amount_x", &self.tile_amount_x)?;
        state.serialize_field("tile_amount_y", &self.tile_amount_y)?;
        state.serialize_field("starts", &self.starts)?;
        state.serialize_field("goals", &self.goals)?;
        state.serialize_field("multiple_agents", &self.multiple_agents)?;
        state.serialize_field("multiple_goals", &self.multiple_goals)?;
        self.ensure_grid();
        let grid_borrow = self.cached_grid.borrow();
        let tiles: Vec<(String, TileType, String, String)> = grid_borrow
            .as_ref()
            .unwrap()
            .iter()
            .map(|tile| {
                (
                    format!("{},{}", tile.position.0, tile.position.1),
                    tile.tile_type,
                    format!("{}", tile.weight),
                    format!(
                        "{},{},{}",
                        tile.cached_color.rgb().0,
                        tile.cached_color.rgb().1,
                        tile.cached_color.rgb().2
                    ),
                )
            })
            .collect();
        state.serialize_field("tiles", &tiles)?;
        state.end()
    }
}

/// Component trait implementation for Board.
///
/// Provides standard UI interaction methods for the game board.
impl Component for Board {
    /// Handle a mouse click on the board.
    ///
    /// Calculates which tile was clicked and updates it based on
    /// the currently selected piece type.
    fn on_click(&mut self, mouse_point: Point) -> (bool, Option<String>) {
        let rect = self.get_rect();
        if !self.active || !rect.contains_point(mouse_point) {
            return (false, None);
        }

        // Calculate which tile was clicked directly instead of iterating
        let tile_width = self.tile_width() as i32;
        let tile_height = self.tile_height() as i32;
        let relative_x = mouse_point.x() - self.location.x();
        let relative_y = mouse_point.y() - self.location.y();

        if relative_x < 0 || relative_y < 0 {
            return (false, None);
        }

        let tile_x = relative_x / tile_width;
        let tile_y = relative_y / tile_height;

        if tile_x >= self.tile_amount_x as i32 || tile_y >= self.tile_amount_y as i32 {
            return (false, None);
        }

        let mut tile_clicked: bool = false;

        let pos_idx = (tile_y * self.tile_amount_x as i32 + tile_x) as usize;
        self.ensure_grid();
        let mut borrow = self.cached_grid.borrow_mut();
        let grid = borrow.as_mut().unwrap();

        if grid.get(pos_idx).is_some() {
            tile_clicked = true;
        }
        if tile_clicked {
            if !self.multiple_agents && self.selected_piece_type == TileType::Player {
                if !self.starts.is_empty() {
                    for start in &self.starts {
                        grid[*start].change_tile_type(TileType::Floor);
                    }
                    self.starts.clear();
                }
            }

            if !self.multiple_goals && self.selected_piece_type == TileType::Enemy {
                if !self.goals.is_empty() {
                    for goal in &self.goals {
                        grid[*goal].change_tile_type(TileType::Floor);
                    }
                    self.goals.clear();
                }
            }

            if let Some(tile) = grid.get_mut(pos_idx) {
                match self.selected_piece_type {
                    TileType::Obstacle => tile.change_tile_type(TileType::Obstacle),
                    TileType::Enemy => {
                        if tile.tile_type != TileType::Enemy {
                            self.goals.push(pos_idx);
                            tile.change_tile_type(TileType::Enemy);
                        }
                    }
                    TileType::Player => {
                        if tile.tile_type != TileType::Player {
                            self.starts.push(pos_idx);
                            tile.change_tile_type(TileType::Player);
                        }
                    }
                    TileType::Weighted(weight) => {
                        tile.weight = weight;
                        tile.cached_color = Tile::calc_floor_color(weight);
                        tile.dirty = true;
                    }
                    _ => {}
                }

                self.mark_texture_dirty();
                return (true, Some(self.get_id()));
            }
        }

        return (false, None);
    }

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect();
        return component.contains_point(mouse_position);
    }

    fn get_id(&self) -> String {
        return self.id.to_string();
    }

    fn get_location(&self) -> Point {
        self.location
    }

    fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
        self.cached_background = None;
    }

    fn change_active(&mut self, new_value: bool) {
        self.active = new_value;
        self.cached_background = None;
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn get_width(&self) -> u32 {
        self.width
    }
    fn get_height(&self) -> u32 {
        self.height
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
        self.cached_background = None;
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
        self.cached_background = None;
    }
}

impl Board {
    /// Load a board from a JSON string.
    ///
    /// # Arguments
    /// * `board_json` - JSON string containing serialized board data
    ///
    /// # Returns
    /// A new Board instance with the loaded data
    pub fn load_board_file(&self, board_json: String) -> Result<Self, &'static str> {
        match serde_json::from_str(&board_json) {
            Ok(result) => return Ok(result),
            Err(_) => return Err("Invalid JSON"),
        }
    }

    /// Ensure the cached grid is populated (no clone).
    fn ensure_grid(&self) {
        if self.cached_grid.borrow().is_some() {
            return;
        }
        let tile_amount: usize = (self.tile_amount_x * self.tile_amount_y) as usize;
        let mut grid = Vec::with_capacity(tile_amount);
        let tile_width = self.tile_width();
        let tile_height = self.tile_height();
        for tile in 0..tile_amount {
            let num: u8 = 1;
            grid.push(Tile::new(
                util::get_coordinate_from_idx(tile, self.tile_amount_x, self.tile_amount_y),
                TileType::Floor,
                tile_height,
                tile_width,
                num,
                true,
                WHITE,
            ));
        }
        self.cached_grid.borrow_mut().replace(grid);
    }

    /// Get an owned clone of the tile grid. Prefer borrowing cached_grid directly.
    pub fn grid(&self) -> Vec<Tile> {
        self.ensure_grid();
        self.cached_grid.borrow().as_ref().unwrap().clone()
    }

    /// Get the width of each tile in pixels.
    pub fn tile_width(&self) -> u32 {
        self.width / self.tile_amount_x
    }

    /// Get the height of each tile in pixels.
    pub fn tile_height(&self) -> u32 {
        self.height / self.tile_amount_y
    }

    fn get_random_agents(&mut self) {
        let tile_amount = (self.tile_amount_x * self.tile_amount_y) as usize;
        let amount = self.starts.len().max(1) * 2;
        println!("{}", amount);
        self.starts.clear();
        self.goals.clear();
        println!("{}", amount);
        let mut rng = rand::rng();
        let locations = sample(&mut rng, tile_amount, amount).into_vec();
        let (starts, goals) = locations.split_at(locations.len() / 2);
        for (i, start) in starts.iter().enumerate() {
            self.starts.push(*start);
            self.goals.push(goals[i]);
        }
    }

    /// Generate a random grid with obstacles and weighted tiles.
    ///
    /// # Arguments
    /// * `weight_range` - Maximum weight value for weighted tiles
    /// * `obstacle_percentage` - Percentage of tiles that will be obstacles
    /// * `weighted_percentage` - Percentage of tiles that will be weighted
    pub fn generate_random_grid(
        &mut self,
        weight_range: u8,
        obstacle_percentage: usize,
        weighted_percentage: usize,
        random_agents: bool,
    ) {
        if random_agents {
            println!("Yes");
            self.get_random_agents();
        }
        println!("{:#?}", self.starts);
        println!("{:#?}", self.goals);
        let tile_amount = (self.tile_amount_x * self.tile_amount_y) as usize;
        let mut grid: Vec<Tile> = Vec::with_capacity(tile_amount);
        let tile_width = self.tile_width();
        let tile_height = self.tile_height();
        let mut rng = rand::rng();
        let weighted_number = (tile_amount as f32 * (weighted_percentage as f32 / 100.0)) as usize;
        let obstacle_number = if obstacle_percentage == 0 {
            0
        } else {
            (tile_amount as u32 / (2 * self.starts.len().max(1) as u32))
                .min((tile_amount as f32 * (obstacle_percentage as f32 / 100.0)) as u32)
                as usize
        };

        for j in 0..self.tile_amount_y {
            for i in 0..self.tile_amount_x {
                let position_idx: usize = (j * self.tile_amount_x + i) as usize;
                let position = (i as i32, j as i32);
                let num: u8 = 1;
                if self.starts.contains(&position_idx) {
                    grid.push(Tile::new(
                        position,
                        TileType::Player,
                        tile_height,
                        tile_width,
                        num,
                        true,
                        GREEN,
                    ));
                } else if self.goals.contains(&position_idx) {
                    grid.push(Tile::new(
                        position,
                        TileType::Enemy,
                        tile_height,
                        tile_width,
                        num,
                        true,
                        RED,
                    ));
                } else {
                    grid.push(Tile::new(
                        position,
                        TileType::Floor,
                        tile_height,
                        tile_width,
                        num,
                        true,
                        Tile::calc_floor_color(num),
                    ));
                }
            }
        }

        // Collect all floor tile positions
        let selected: Vec<usize> = grid
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_floor())
            .map(|(i, _)| i)
            .collect();
        let selected_length = selected.len();
        let total_special = (obstacle_number + weighted_number).min(selected_length);

        let mut selected: Vec<usize> = selected
            .into_iter()
            .choose_multiple(&mut rng, total_special);
        selected.shuffle(&mut rng);

        // Choose unique positions for obstacles and weighted tiles in one pass

        // Assign obstacles
        for &pos in selected.iter().take(obstacle_number) {
            grid[pos].change_tile_type(TileType::Obstacle);
        }

        // Assign weighted tiles (skip those already made obstacles)
        for &pos in selected.iter().skip(obstacle_number) {
            let weight = rng.random_range(1..=weight_range.min(255));
            grid[pos].weight = weight;
            grid[pos].cached_color = Tile::calc_floor_color(weight);
        }

        self.cached_grid.borrow_mut().replace(grid);
    }

    /// Generate a city-style grid with roads and buildings.
    ///
    /// Creates a grid with:
    /// - Grid of roads at random intervals
    /// - Rectangular buildings placed in non-road areas
    /// - Roads have lower weights for faster traversal
    ///
    /// # Arguments
    /// * `road_weight` - Weight value for road tiles (lower = faster)
    /// * `road_min_spacing` - Minimum tiles between roads
    /// * `road_max_spacing` - Maximum tiles between roads
    /// * `building_density` - Percentage of area covered by buildings (0-100)
    /// * `building_min_size` - Minimum building dimension
    /// * `building_max_size` - Maximum building dimension
    pub fn generate_organic_city(
        &mut self,
        road_weight: u8,
        road_min_spacing: u32,
        road_max_spacing: u32,
        building_density: f32, // 0.0 to 1.0
        building_min_size: u32,
        building_max_size: u32,
        random_agents: bool,
    ) {
        self.cached_background = None;
        if random_agents {
            self.get_random_agents();
        }
        let tile_amount = (self.tile_amount_x * self.tile_amount_y) as usize;
        let mut grid: Vec<Tile> = Vec::with_capacity(tile_amount);
        let tile_width = self.tile_width();
        let tile_height = self.tile_height();
        let mut rng = rand::rng();
        let mut grid_allocation: Vec<u8> = vec![0; tile_amount];
        // road = 1
        // floor = 0
        // obstacle = 2

        let mut x = 0;
        while x < self.tile_amount_x {
            for j in 0..self.tile_amount_y {
                let idx = (j as usize) * self.tile_amount_x as usize + (x as usize);
                if self.starts.contains(&idx) || self.goals.contains(&idx) {
                    continue;
                }
                grid_allocation[idx] = 1;
            }
            x += rng.random_range(road_min_spacing..=road_max_spacing);
        }

        // Generate vertical roads
        let mut y = 0;
        while y < self.tile_amount_y {
            for i in 0..self.tile_amount_x {
                let idx = (i as usize) + (y as usize) * self.tile_amount_x as usize;
                if self.starts.contains(&idx) || self.goals.contains(&idx) {
                    continue;
                }
                grid_allocation[idx] = 1;
            }
            y += rng.random_range(road_min_spacing..=road_max_spacing);
        }

        let average_building_size = (building_min_size + building_max_size) as f32;

        let num_buildings =
            ((tile_amount as f32 / average_building_size) * (building_density / 100.0)) as u32;

        for _ in 0..num_buildings {
            let start_x = rng.random_range(0..self.tile_amount_x as i32);
            let start_y = rng.random_range(0..self.tile_amount_y as i32);
            let width = rng.random_range(building_min_size..=building_max_size) as i32;
            let height = rng.random_range(building_min_size..=building_max_size) as i32;

            for x in start_x..=(start_x + width).min(self.tile_amount_x as i32 - 1) {
                for y in start_y..=(start_y + height).min(self.tile_amount_y as i32 - 1) {
                    let idx = (x as usize) + (y as usize) * self.tile_amount_x as usize;
                    if self.starts.contains(&idx) || self.goals.contains(&idx) {
                        continue;
                    }
                    if grid_allocation[idx] == 0 {
                        grid_allocation[idx] = 2;
                    }
                }
            }
        }

        // Initialize all as floors
        for j in 0..self.tile_amount_y {
            for i in 0..self.tile_amount_x {
                let position = (i as i32, j as i32);
                let idx = (j as usize) * self.tile_amount_x as usize + (i as usize);
                let tile_type = if self.starts.contains(&idx) {
                    TileType::Player
                } else if self.goals.contains(&idx) {
                    TileType::Enemy
                } else if grid_allocation[idx] == 2 {
                    TileType::Obstacle
                } else {
                    TileType::Floor
                };
                if grid_allocation[idx] == 1 {
                    grid.push(Tile::new(
                        position,
                        tile_type,
                        tile_height,
                        tile_width,
                        road_weight,
                        true,
                        Tile::calc_floor_color(road_weight),
                    ));
                } else if grid_allocation[idx] == 2 {
                    grid.push(Tile::new(
                        position,
                        tile_type,
                        tile_height,
                        tile_width,
                        0,
                        true,
                        BLACK,
                    ));
                } else {
                    grid.push(Tile::new(
                        position,
                        tile_type,
                        tile_height,
                        tile_width,
                        255,
                        true,
                        Tile::calc_floor_color(255),
                    ));
                }
            }
        }

        // Generate horizontal roads
        // Place random rectangular buildings

        self.cached_grid.borrow_mut().replace(grid);
    }

    /// Save the board to a JSON file.
    ///
    /// # Arguments
    /// * `filepath` - Directory to save in
    /// * `file_name` - Name of the file (without extension)
    pub fn save_to_file(
        &self,
        filepath: &str,
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(filepath.to_owned() + "/" + file_name.trim() + ".json", json)?;
        Ok(())
    }

    /// Create pathfinding agents from start/goal positions.
    ///
    /// Each start position is paired with the corresponding goal position.
    fn create_agents(&mut self) -> Result<&'static str, &'static str> {
        if self.goals.is_empty() || self.starts.is_empty() {
            return Err("No goals or Agents on Board");
        }
        let goal_amount: usize = self.goals.len();
        let start_amount: usize = self.starts.len();
        let indexes = start_amount.max(goal_amount);

        for i in 0..indexes {
            let mut start_index = i;
            let mut goal_index = i;
            if goal_amount != start_amount {
                if i > goal_amount - 1 {
                    goal_index = i % goal_amount;
                } else if i > start_amount - 1 {
                    start_index = i % start_amount;
                }
            }

            let start = util::get_coordinate_from_idx(
                self.starts[start_index],
                self.tile_amount_x,
                self.tile_amount_y,
            );
            self.agents.push(Agent {
                start: start,
                goal: util::get_coordinate_from_idx(
                    self.goals[goal_index],
                    self.tile_amount_x,
                    self.tile_amount_y,
                ),
                position: start,
                path: vec![],
            });
        }
        return Ok("Agents Successfully created");
    }

    /// Initialize a data map for collecting benchmark metrics per agent.
    fn create_data_map(&self, amount: usize) -> HashMap<usize, PathData> {
        let mut data_map: HashMap<usize, PathData> = HashMap::with_capacity(amount);
        for i in 0..amount {
            data_map.insert(
                i,
                PathData {
                    wcf: vec![],
                    memory: vec![],
                    time: vec![],
                    steps: vec![],
                    path_cost: vec![],
                },
            );
        }
        return data_map;
    }

    /// Run pathfinding and animate agents moving to their goals.
    ///
    /// This is the main function for executing pathfinding experiments.
    /// It supports:
    /// - Multiple iterations with optional grid regeneration
    /// - Doubling experiment (doubles obstacles each iteration)
    /// - Multi-threaded pathfinding for multiple agents
    /// - Real-time visualization of agent movement
    ///
    /// # Arguments
    /// * `canvas` - SDL2 canvas for rendering
    /// * `algorithm` - Name of the pathfinding algorithm to use
    /// * `doubling` - If true, double obstacles each iteration
    /// * `dyn_gen` - If true, regenerate grid each iteration
    /// * `obstacles` - Initial obstacle percentage
    /// * `weighted_tiles` - Weighted tile percentage
    /// * `iterations` - Number of iterations to run
    /// * `weight_range` - Maximum tile weight
    /// * `gen_mode` - Generation mode (Random or City)
    pub fn run_board<'a>(
        &mut self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        algorithm: &str,
        doubling: bool,
        dyn_gen: bool,
        random_agents: bool,
        obstacles: u32,
        weighted_tiles: u32,
        iterations: usize,
        weight_range: u8,
        gen_mode: settings::GenerationMode,
    ) -> Result<String, &'static str> {
        if !random_agents {
            if self.agents.is_empty() {
                match self.create_agents() {
                    Err(e) => {
                        return Err(e);
                    }
                    _ => {}
                }
            }
        }
        let mut data_map: HashMap<usize, PathData> = self.create_data_map(iterations);
        let mut obstacles = obstacles as usize;
        for i in 0..iterations {
            let mut valid_iteration = false;
            while !valid_iteration {
                if doubling || dyn_gen {
                    match gen_mode {
                        settings::GenerationMode::Random => {
                            self.generate_random_grid(
                                weight_range.min(255),
                                obstacles as usize,
                                weighted_tiles as usize,
                                random_agents,
                            );
                        }
                        settings::GenerationMode::City => {
                            self.generate_organic_city(
                                0,
                                2,
                                weight_range as u32,
                                obstacles as f32,
                                2,
                                weighted_tiles,
                                random_agents,
                            );
                        }
                    }
                }
                if self.agents.is_empty() {
                    self.create_agents();
                }
                let grid: Arc<Vec<Tile>> = Arc::new(self.grid());
                valid_iteration = true;
                let mut agents_completed_count = 0;
                while agents_completed_count != self.starts.len() {
                    let mut handles = vec![];
                    // Spawn threads - each gets a cheap Arc clone (refcount bump, no data copy)
                    for agent_idx in 0..self.agents.len() {
                        let algorithm_str = algorithm.to_string();
                        let grid = Arc::clone(&grid);

                        let mut agent_clone = self.agents[agent_idx].clone();
                        let w = self.tile_amount_x;
                        let h = self.tile_amount_y;

                        let handle = thread::spawn(move || {
                            if agent_clone.is_path_possible(&grid, w, h) {
                                let (_, path, wcf, memory, time, steps, path_cost) =
                                    agent_clone.get_path(&algorithm_str, &grid, w, h);
                                if !path.is_empty() {
                                    (
                                        agent_idx,
                                        Some(path),
                                        Some(wcf),
                                        Some(memory),
                                        Some(time),
                                        Some(steps),
                                        Some(path_cost),
                                    )
                                } else {
                                    (
                                        agent_idx,
                                        None,
                                        Some(wcf),
                                        Some(memory),
                                        Some(time),
                                        Some(steps),
                                        None,
                                    )
                                }
                            } else {
                                (agent_idx, None, None, None, None, None, None)
                            }
                        });
                        handles.push(handle);
                    }

                    // Collect results and update board on main thread
                    for handle in handles {
                        if let Ok((index, path, wcf, memory, time, steps, path_cost)) =
                            handle.join()
                        {
                            if path.is_none() {
                                // If any path is not possible, regenerate
                                if steps.is_none() {
                                    valid_iteration = false;
                                    if !doubling && !dyn_gen {
                                        return Err("No possible Path");
                                    }
                                } else {
                                    if !doubling && !dyn_gen {
                                        return Err("Path is possible but algorithm couldn't find a solution in a reasonable amount of time");
                                    }
                                    break;
                                }
                            } else {
                                // Update agent
                                self.agents[index].path = path.unwrap();
                                if let Some(iteration_data) = data_map.get_mut(&i) {
                                    iteration_data.update_all(
                                        wcf.unwrap_or_default(),
                                        memory.unwrap_or_default(),
                                        time.unwrap_or_default(),
                                        steps.unwrap_or_default(),
                                        path_cost.unwrap_or_default(),
                                    );
                                }
                                agents_completed_count += 1;
                            }
                        }
                    }

                    if !valid_iteration {
                        self.agents.clear();
                        break;
                    }

                    self.draw(canvas, texture_creator);
                }
            }
            if doubling {
                obstacles *= 2;
            }
            self.draw(canvas, texture_creator);
        }
        let mut data_display = String::new();
        for (_, data) in &data_map {
            data_display += format!("{}", data).as_str();
        }
        fileDialog::save_data(&data_map);
        return Ok(data_display);
    }

    pub fn display_path_result(&mut self) -> bool {
        let mut grid = self.cached_grid.borrow_mut();
        let grid = grid.as_mut().unwrap();
        let w = self.tile_amount_x as u32;

        let mut all_finished = true;

        for agent in &mut self.agents {
            let path_len = agent.path.len();
            if path_len == 0 {
                continue;
            }
            if !agent.goal_reached() {
                all_finished = false;
            }

            if let Some(pos) = agent.path.pop() {
                if let Some(pos_idx) = util::get_idx_from_coordinate(pos, w, w) {
                    if let Some(old_idx) = util::get_idx_from_coordinate(agent.position, w, w) {
                        if let Some(tile) = grid.get_mut(old_idx) {
                            if tile.tile_type != TileType::Enemy {
                                tile.change_tile_type(TileType::Path);
                            }
                        }
                    }

                    if let Some(tile) = grid.get_mut(pos_idx) {
                        if tile.tile_type != TileType::Enemy {
                            tile.change_tile_type(TileType::Player);
                        }
                    }
                }
                agent.position = pos;
            } else {
                if let Some(end_idx) = util::get_idx_from_coordinate(agent.position, w, w) {
                    if let Some(tile) = grid.get_mut(end_idx) {
                        if tile.tile_type != TileType::Enemy {
                            tile.change_tile_type(TileType::Floor);
                        }
                    }
                }

                if let Some(goal_idx) = util::get_idx_from_coordinate(agent.goal, w, w) {
                    if let Some(tile) = grid.get_mut(goal_idx) {
                        tile.change_tile_type(TileType::Enemy);
                    }
                }
            }
        }

        if all_finished {
            return true;
        }

        false
    }

    pub fn clear_path(&mut self) {
        let mut grid = self.cached_grid.borrow_mut();
        let grid = grid.as_mut().unwrap();
        grid.iter_mut()
            .filter(|tile| tile.tile_type == TileType::Path)
            .for_each(|tile| tile.change_tile_type(TileType::Floor));
    }

    pub fn reset_board(&mut self) {
        let mut grid = self.cached_grid.borrow_mut();
        let grid = grid.as_mut().unwrap();
        let w = self.tile_amount_x as usize;

        self.agents.iter_mut().for_each(|agent| {
            let current = agent.position;
            let start = agent.start;

            let current_idx = current.1 as usize * w + current.0 as usize;
            if let Some(tile) = grid.get_mut(current_idx) {
                if tile.tile_type != TileType::Enemy {
                    tile.change_tile_type(TileType::Floor);
                }
            }

            let start_idx = start.1 as usize * w + start.0 as usize;
            if let Some(new_tile) = grid.get_mut(start_idx) {
                if new_tile.tile_type != TileType::Enemy {
                    new_tile.change_tile_type(TileType::Player);
                }
            }
        });
        self.agents.clear();
    }

    pub fn mark_texture_dirty(&self) {
        *self.texture_dirty.borrow_mut() = true;
    }

    /// Get the bounding rectangle of the board.
    fn get_rect(&self) -> Rect {
        if self.cached_background.is_none() {
            Rect::new(
                self.location.x(),
                self.location.y(),
                self.height,
                self.width,
            )
        } else {
            self.cached_background.expect("No background")
        }
    }

    /// Draw the entire board to the canvas.
    ///
    /// Uses cached texture when available, only re-renders when dirty.
    pub fn draw(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &TextureCreator<WindowContext>,
    ) {
        let is_dirty = *self.texture_dirty.borrow();

        if is_dirty || self.cached_texture.borrow().is_none() {
            let mut target = texture_creator
                .create_texture(
                    PixelFormatEnum::RGBA8888,
                    sdl2::render::TextureAccess::Target,
                    self.width,
                    self.height,
                )
                .unwrap();
            target.set_blend_mode(sdl2::render::BlendMode::Blend);

            canvas
                .with_texture_canvas(&mut target, |target_canvas| {
                    target_canvas.set_draw_color(WHITE);
                    target_canvas
                        .fill_rect(Rect::new(0, 0, self.width, self.height))
                        .unwrap();

                    let mut borrow = self.cached_grid.borrow_mut();
                    if let Some(grid) = borrow.as_mut() {
                        for tile in grid.iter_mut() {
                            tile.draw(true, Point::new(0, 0), target_canvas);
                        }
                    }
                })
                .expect("Failed to create canvas for texture");

            *self.cached_texture.borrow_mut() =
                Some(unsafe { std::mem::transmute::<Texture<'_>, Texture<'static>>(target) });
            *self.texture_dirty.borrow_mut() = false;
        }

        canvas
            .copy(
                self.cached_texture.borrow().as_ref().unwrap(),
                None,
                self.get_rect(),
            )
            .unwrap();
    }
}

pub use scanner::*;

pub mod scanner {
    use sdl2::rect::Point;

    use crate::{
        components::board::{Board, TileType},
        Tile,
    };
    pub enum file_type {
        Map,
        Image,
        JSON,
    }

    use std::{
        cell::RefCell,
        collections::HashSet,
        env,
        ffi::{OsStr, OsString},
        fs::{read_dir, read_to_string, write, DirEntry},
        path::{Path, PathBuf},
    };

    use crate::{colors::*, fileDialog, settings, util};

    pub fn board_from(
        file: &str,
        board_size: u32,
        tile_amount: u32,
    ) -> Result<Board, &'static str> {
        match Path::new(file).extension().and_then(OsStr::to_str) {
            Some(ext) => match ext {
                "map" => return board_from_map(file, board_size, tile_amount),
                "json" => return board_from_json(file),
                _ => return Err("Not a supported file type"),
            },
            None => return Err("No file given"),
        }
    }

    fn board_from_map(
        file: &str,
        board_size: u32,
        tile_amount: u32,
    ) -> Result<Board, &'static str> {
        let mut board_size = board_size;
        let mut tile_amount = tile_amount;
        match fileDialog::read_file(file) {
            Ok(result) => {
                for line in result.split("\n") {
                    if line.contains("height") || line.contains("width") {
                        let words: Vec<&str> = line.split_whitespace().collect();
                        match words[1].parse::<u32>() {
                            Ok(size) => {
                                tile_amount = size.min(512);
                                break;
                            }
                            Err(_) => {}
                        }
                    }
                }
                let mut tiles: Vec<Tile> = Vec::new();
                let mut tile_dim = board_size / tile_amount;
                if tile_dim <= 1 {
                    tile_dim = 2;
                    board_size = (tile_amount * 2).min(1024);
                }
                for (i, char) in result
                    .chars()
                    .filter(|c| c == &'.' || c == &'@')
                    .enumerate()
                {
                    if char == '.' {
                        tiles.push(Tile::new(
                            util::get_coordinate_from_idx(i, tile_amount, tile_amount),
                            super::TileType::Floor,
                            tile_dim,
                            tile_dim,
                            1,
                            true,
                            WHITE,
                        ));
                    } else if char == '@' {
                        tiles.push(Tile::new(
                            util::get_coordinate_from_idx(i, tile_amount, tile_amount),
                            super::TileType::Obstacle,
                            tile_dim,
                            tile_dim,
                            1,
                            true,
                            BLACK,
                        ));
                    }
                }

                Ok(Board {
                    location: Point::new(0, 0),
                    height: board_size,
                    width: board_size,
                    tile_amount_x: tile_amount,
                    tile_amount_y: tile_amount,
                    selected_piece_type: TileType::Obstacle,
                    id: String::from("game_board"),
                    active: true,
                    multiple_agents: false,
                    multiple_goals: false,
                    cached_background: None,
                    cached_grid: RefCell::new(Some(tiles)),
                    cached_texture: RefCell::new(None),
                    texture_dirty: RefCell::new(true),
                    agents: vec![],
                    goals: vec![],
                    starts: vec![],
                })
            }
            Err(_) => return Err("Invalid JSON"),
        }
    }

    fn board_from_json(file: &str) -> Result<Board, &'static str> {
        match fileDialog::read_file(file) {
            Ok(result) => match serde_json::from_str(&result) {
                Ok(result) => return Ok(result),
                Err(_) => return Err("Invalid JSON"),
            },
            Err(_) => return Err("Couldn't read file"),
        }
    }

    fn board_from_image(file: &str) -> Result<Vec<Tile>, &'static str> {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, num::NonZero};

    /// Helper: create a small Board for testing
    fn make_test_board(tiles_x: u32, tiles_y: u32) -> Board {
        Board {
            location: Point::new(0, 0),
            height: tiles_y * 10,
            width: tiles_x * 10,
            tile_amount_x: tiles_x,
            tile_amount_y: tiles_y,
            selected_piece_type: TileType::Floor,
            id: "test_board".to_string(),
            starts: vec![],
            goals: vec![],
            active: true,
            multiple_agents: false,
            multiple_goals: false,
            agents: vec![],
            cached_background: None,
            cached_grid: RefCell::new(None),
            cached_texture: RefCell::new(None),
            texture_dirty: RefCell::new(true),
        }
    }

    // ------- Tile -------

    #[test]
    fn test_tile_new_position_is_pixel_scaled() {
        let tile = Tile::new((3, 4), TileType::Floor, 10, 10, 1, false, WHITE);
        // Position is grid * width/height
        assert_eq!(tile.position, (30, 40));
    }

    #[test]
    fn test_tile_floor_is_traversable() {
        let tile = Tile::new((0, 0), TileType::Floor, 10, 10, 1, false, WHITE);
        assert!(tile.is_traversable());
    }

    #[test]
    fn test_tile_obstacle_is_not_traversable() {
        let tile = Tile::new((0, 0), TileType::Obstacle, 10, 10, 1, false, WHITE);
        assert!(!tile.is_traversable());
    }

    #[test]
    fn test_tile_player_is_traversable() {
        let tile = Tile::new((0, 0), TileType::Player, 10, 10, 1, false, WHITE);
        assert!(tile.is_traversable());
    }

    #[test]
    fn test_tile_enemy_is_traversable() {
        let tile = Tile::new((0, 0), TileType::Enemy, 10, 10, 1, false, WHITE);
        assert!(tile.is_traversable());
    }

    #[test]
    fn test_tile_weighted_is_traversable() {
        let tile = Tile::new((0, 0), TileType::Weighted(50), 10, 10, 50, false, WHITE);
        assert!(tile.is_traversable());
    }

    #[test]
    fn test_tile_is_floor_true() {
        let tile = Tile::new((0, 0), TileType::Floor, 10, 10, 1, false, WHITE);
        assert!(tile.is_floor());
    }

    #[test]
    fn test_tile_is_floor_false_for_obstacle() {
        let tile = Tile::new((0, 0), TileType::Obstacle, 10, 10, 1, false, WHITE);
        assert!(!tile.is_floor());
    }

    #[test]
    fn test_tile_is_floor_false_for_player() {
        let tile = Tile::new((0, 0), TileType::Player, 10, 10, 1, false, WHITE);
        assert!(!tile.is_floor());
    }

    #[test]
    fn test_tile_change_tile_type_marks_dirty() {
        let mut tile = Tile::new((0, 0), TileType::Floor, 10, 10, 1, false, WHITE);
        assert!(!tile.dirty);
        tile.change_tile_type(TileType::Obstacle);
        assert!(tile.dirty);
        assert!(!tile.is_traversable());
    }

    #[test]
    fn test_tile_change_tile_type_same_type_not_dirty() {
        let mut tile = Tile::new((0, 0), TileType::Floor, 10, 10, 1, false, WHITE);
        tile.change_tile_type(TileType::Floor);
        assert!(!tile.dirty); // no change
    }

    #[test]
    fn test_tile_get_rect() {
        let tile = Tile::new((2, 3), TileType::Floor, 10, 10, 1, false, WHITE);
        let rect = tile.get_rect(Point::new(100, 200));
        assert_eq!(rect.x(), 100 + 20); // board_origin.x + pixel position
        assert_eq!(rect.y(), 200 + 30);
        assert_eq!(rect.width(), 10);
        assert_eq!(rect.height(), 10);
    }

    #[test]
    fn test_tile_weight_default() {
        let tile = Tile::new((0, 0), TileType::Floor, 10, 10, 1, false, WHITE);
        assert_eq!(tile.weight, 1);
    }

    #[test]
    fn test_tile_weight_custom() {
        let tile = Tile::new((0, 0), TileType::Weighted(42), 10, 10, 42, false, WHITE);
        assert_eq!(tile.weight, 42);
    }

    // ------- TileType -------

    #[test]
    fn test_tiletype_equality() {
        assert_eq!(TileType::Floor, TileType::Floor);
        assert_eq!(TileType::Obstacle, TileType::Obstacle);
        assert_eq!(TileType::Player, TileType::Player);
        assert_eq!(TileType::Enemy, TileType::Enemy);
        assert_eq!(TileType::Path, TileType::Path);
        assert_ne!(TileType::Floor, TileType::Obstacle);
        assert_ne!(TileType::Player, TileType::Enemy);
    }

    #[test]
    fn test_tiletype_weighted_equality() {
        assert_eq!(TileType::Weighted(5), TileType::Weighted(5));
        assert_ne!(TileType::Weighted(5), TileType::Weighted(10));
    }

    #[test]
    fn test_tiletype_serialization_roundtrip() {
        let types = vec![
            TileType::Floor,
            TileType::Obstacle,
            TileType::Player,
            TileType::Enemy,
            TileType::Path,
            TileType::Weighted(100),
        ];
        for tt in types {
            let json = serde_json::to_string(&tt).unwrap();
            let loaded: TileType = serde_json::from_str(&json).unwrap();
            assert_eq!(tt, loaded);
        }
    }

    // ------- Board -------

    #[test]
    fn test_board_tile_width() {
        let board = make_test_board(40, 40);
        assert_eq!(board.tile_width(), 10); // 400 / 40
    }

    #[test]
    fn test_board_tile_height() {
        let board = make_test_board(40, 40);
        assert_eq!(board.tile_height(), 10);
    }

    #[test]
    fn test_board_grid_creates_floor_tiles() {
        let board = make_test_board(5, 5);
        let grid = board.grid();
        assert_eq!(grid.len(), 25);
        for tile in grid.iter() {
            assert!(tile.is_traversable());
        }
    }

    #[test]
    fn test_board_grid_is_cached() {
        let board = make_test_board(3, 3);
        let grid1 = board.grid();
        let grid2 = board.grid();
        assert_eq!(grid1.len(), grid2.len());
    }

    #[test]
    fn test_board_get_id() {
        let board = make_test_board(5, 5);
        assert_eq!(board.get_id(), "test_board");
    }

    #[test]
    fn test_board_component_active() {
        let mut board = make_test_board(5, 5);
        assert!(board.is_active());
        board.change_active(false);
        assert!(!board.is_active());
    }

    #[test]
    fn test_board_change_location() {
        let mut board = make_test_board(5, 5);
        board.change_location(Point::new(100, 200));
        assert_eq!(board.get_location(), Point::new(100, 200));
    }

    #[test]
    fn test_board_dimensions() {
        let mut board = make_test_board(5, 5);
        assert_eq!(board.get_width(), 50);
        assert_eq!(board.get_height(), 50);
        board.change_width(200);
        board.change_height(300);
        assert_eq!(board.get_width(), 200);
        assert_eq!(board.get_height(), 300);
    }

    #[test]
    fn test_board_mouse_over_inside() {
        let board = make_test_board(10, 10);
        assert!(board.mouse_over_component(Point::new(50, 50)));
    }

    #[test]
    fn test_board_mouse_over_outside() {
        let board = make_test_board(10, 10);
        assert!(!board.mouse_over_component(Point::new(200, 200)));
    }

    // ------- Board serialization -------

    #[test]
    fn test_board_serialize_deserialize() {
        let board = make_test_board(5, 5);
        let _ = board.grid(); // Initialize grid
        let json = serde_json::to_string(&board).unwrap();
        let loaded: Board = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.tile_amount_x, 5);
        assert_eq!(loaded.tile_amount_y, 5);
    }

    #[test]
    fn test_board_serialization_preserves_dimensions() {
        let board = make_test_board(20, 15);
        let _ = board.grid();
        let json = serde_json::to_string(&board).unwrap();
        let loaded: Board = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.width, 200);
        assert_eq!(loaded.height, 150);
        assert_eq!(loaded.tile_amount_x, 20);
        assert_eq!(loaded.tile_amount_y, 15);
    }

    #[test]
    fn test_board_serialization_preserves_starts_goals() {
        let mut board = make_test_board(10, 10);
        board.starts = vec![0, 11];
        board.goals = vec![99, 88];
        let _ = board.grid();
        let json = serde_json::to_string(&board).unwrap();
        let loaded: Board = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.starts, vec![0, 11]);
        assert_eq!(loaded.goals, vec![99, 88]);
    }

    // ------- Board generate_random_grid -------

    #[test]
    fn test_generate_random_grid_creates_grid() {
        let mut board = make_test_board(10, 10);
        board.starts = vec![0];
        board.goals = vec![99];
        board.generate_random_grid(10, 20, 10, false);
        let grid = board.grid();
        assert_eq!(grid.len(), 100);
    }

    #[test]
    fn test_generate_random_grid_has_obstacles() {
        let mut board = make_test_board(20, 20);
        board.starts = vec![0];
        board.goals = vec![399];
        board.generate_random_grid(5, 30, 0, false);
        let grid = board.grid();
        let obstacle_count = grid.iter().filter(|t| !t.is_traversable()).count();
        assert!(obstacle_count > 0);
    }

    // ------- Board create_data_map -------

    #[test]
    fn test_create_data_map() {
        let board = make_test_board(5, 5);
        let data_map = board.create_data_map(3);
        assert_eq!(data_map.len(), 3);
        for (_, pd) in &data_map {
            assert!(pd.wcf.is_empty());
            assert!(pd.memory.is_empty());
        }
    }

    // ------- Board save_to_file -------

    #[test]
    fn test_save_to_file() {
        let board = make_test_board(5, 5);
        let _ = board.grid();
        let dir = std::env::temp_dir();
        let dir_str = dir.to_str().unwrap();
        let result = board.save_to_file(dir_str, "pathmaker_test_save");
        assert!(result.is_ok());
        let path = dir.join("pathmaker_test_save.json");
        assert!(path.exists());
        let _ = std::fs::remove_file(path);
    }

    // ------- Board load_board_file -------

    #[test]
    fn test_load_board_file_parses_json() {
        let mut board = make_test_board(5, 5);
        let _ = board.grid();
        let json = serde_json::to_string(&board).unwrap();
        let loaded = board.load_board_file(json).unwrap();
        assert_eq!(loaded.tile_amount_x, 5);
        assert_eq!(loaded.tile_amount_y, 5);
    }

    // ------- Board generate_organic_city -------

    #[test]
    fn test_generate_organic_city_creates_grid() {
        let mut board = make_test_board(10, 10);
        board.generate_organic_city(1, 2, 4, 50.0, 2, 5, false);
        let grid = board.grid();
        assert_eq!(grid.len(), 100);
    }

    #[test]
    fn test_generate_organic_city_with_agents() {
        let mut board = make_test_board(10, 10);
        board.starts = vec![0];
        board.goals = vec![99];
        board.generate_organic_city(1, 2, 4, 50.0, 2, 5, true);
        let grid = board.grid();
        assert_eq!(grid.len(), 100);
        assert!(!board.starts.is_empty() || !board.goals.is_empty());
    }

    // ------- Board ensure_grid -------

    #[test]
    fn test_ensure_grid_populates_cache() {
        let board = make_test_board(5, 5);
        assert!(board.cached_grid.borrow().is_none());
        board.ensure_grid();
        assert!(board.cached_grid.borrow().is_some());
    }

    #[test]
    fn test_grid_returns_clone() {
        let board = make_test_board(3, 3);
        let grid1 = board.grid();
        let grid2 = board.grid();
        assert_eq!(grid1.len(), grid2.len());
    }

    // ------- Additional board tests -------

    #[test]
    fn test_board_get_location() {
        let board = make_test_board(5, 5);
        assert_eq!(board.get_location(), Point::new(0, 0));
    }

    #[test]
    fn test_board_on_click_outside_returns_false() {
        let mut board = make_test_board(10, 10);
        let (clicked, _) = board.on_click(Point::new(2000, 2000));
        assert!(!clicked);
    }

    #[test]
    fn test_board_tile_width_calculation() {
        let mut board = make_test_board(5, 5);
        board.change_width(100);
        board.change_height(100);
        assert_eq!(board.tile_width(), 20);
    }

    #[test]
    fn test_board_tile_height_calculation() {
        let mut board = make_test_board(5, 5);
        board.change_width(100);
        board.change_height(100);
        assert_eq!(board.tile_height(), 20);
    }

    // ------- Board display_path_result step-by-step -------

    #[test]
    fn test_display_path_result_completes_animation() {
        use crate::pathfinding::Agent;
        let mut board = make_test_board(5, 5);
        let _ = board.grid();
        board.agents.push(Agent {
            start: (0, 0),
            goal: (2, 2),
            position: (0, 0),
            path: vec![(0, 0), (1, 0), (2, 0), (2, 1), (2, 2)],
        });

        let mut calls = 0;
        while !board.display_path_result() {
            calls += 1;
            if calls > 100 {
                panic!("display_path_result did not complete");
            }
        }

        assert!(true);
    }

    #[test]
    fn test_display_path_result_no_path_tiles_left() {
        use crate::pathfinding::Agent;
        let mut board = make_test_board(5, 5);
        let _ = board.grid();
        board.agents.push(Agent {
            start: (0, 0),
            goal: (2, 0),
            position: (0, 0),
            path: vec![(0, 0), (1, 0), (2, 0)],
        });

        while !board.display_path_result() {}
        board.clear_path();

        let grid = board.cached_grid.borrow();
        let path_count = grid
            .as_ref()
            .unwrap()
            .iter()
            .filter(|t| t.tile_type == TileType::Path)
            .count();
        assert_eq!(path_count, 0);
    }

    #[test]
    fn test_display_path_result_multiple_agents() {
        use crate::pathfinding::Agent;
        let mut board = make_test_board(5, 5);
        let _ = board.grid();
        board.agents.push(Agent {
            start: (0, 0),
            goal: (1, 0),
            position: (0, 0),
            path: vec![(1, 0), (0, 0)],
        });
        board.agents.push(Agent {
            start: (2, 2),
            goal: (4, 4),
            position: (2, 2),
            path: vec![(4, 4), (3, 3), (2, 2)],
        });

        let mut calls = 0;
        while !board.display_path_result() {
            calls += 1;
            if calls > 100 {
                panic!("display_path_result did not complete");
            }
        }

        assert_eq!(board.agents[0].position, board.agents[0].goal);
        assert_eq!(board.agents[1].position, board.agents[1].goal);
    }
}
