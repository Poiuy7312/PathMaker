use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fmt, fs, thread, u8};

extern crate sdl2;

use rand::seq::IteratorRandom;
use rand::Rng;
use sdl2::mouse::MouseState;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::benchmarks::PathData;
use crate::components::Component;
use crate::pathfinding::Agent;
use crate::{colors::*, fileDialog};

#[derive(Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum TileType {
    Obstacle,
    Floor,
    Player,
    Enemy,
    Weighted(u8),
}

#[derive(Copy, Clone)]
pub struct Tile {
    pub position: (i32, i32),
    tile_type: TileType,
    height: u32,
    width: u32,
    pub weight: u8,
    dirty: bool,
    cached_rectangle: Option<Rect>,
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
        state.end()
    }
}

impl Tile {
    fn new(
        position: (i32, i32),
        tile_type: TileType,
        height: u32,
        width: u32,
        weight: u8,
        dirty: bool,
    ) -> Self {
        let position = (position.0 * width as i32, position.1 * height as i32);
        Tile {
            position,
            tile_type,
            height,
            width,
            dirty,
            cached_rectangle: None,
            weight,
        }
    }
    fn get_rect(&self, board_origin: Point) -> Rect {
        Rect::new(
            board_origin.x() + self.position.0,
            board_origin.y() + self.position.1,
            self.width,
            self.height,
        )
    }
    fn draw(&mut self, change_layout: bool, board_origin: Point, canvas: &mut Canvas<Window>) {
        if change_layout {
            self.cached_rectangle = None;
            self.dirty = true;
        }

        if !self.dirty {
            return;
        }
        let tile_rect = match self.cached_rectangle {
            Some(rect) => rect,
            None => self.get_rect(board_origin),
        };
        self.cached_rectangle = Some(tile_rect);
        match self.tile_type {
            TileType::Obstacle => {
                canvas.set_draw_color(BLACK);
                canvas.fill_rect(tile_rect).unwrap();
                self.dirty = false;
            }
            TileType::Floor | TileType::Weighted(_) => {
                if self.weight > 1 {
                    canvas.set_draw_color(Color::RGB(
                        255,
                        230 - (self.weight / 2) as u8,
                        255 - self.weight as u8,
                    ));
                } else {
                    canvas.set_draw_color(WHITE);
                }
                canvas.fill_rect(tile_rect).unwrap();
                self.dirty = false;
            }
            TileType::Player => {
                canvas.set_draw_color(GREEN);
                canvas.fill_rect(tile_rect).unwrap();
                self.dirty = false;
            }
            TileType::Enemy => {
                canvas.set_draw_color(RED);
                canvas.fill_rect(tile_rect).unwrap();
                self.dirty = false;
            }
        }
    }

    pub fn is_traversable(&self) -> bool {
        return self.tile_type != TileType::Obstacle;
    }
    pub fn is_floor(&self) -> bool {
        return self.tile_type == TileType::Floor;
    }

    fn change_tile_type(&mut self, new_type: TileType) {
        if self.tile_type != new_type {
            self.tile_type = new_type;
            self.dirty = true;
        }
    }
}

pub struct Board {
    pub location: Point,
    pub height: u32,
    pub width: u32,
    pub tile_amount_x: u32,
    pub tile_amount_y: u32,
    pub selected_piece_type: TileType,
    pub id: String,
    pub starts: Vec<(i32, i32)>,
    pub goals: Vec<(i32, i32)>,
    pub active: bool,
    pub multiple_agents: bool,
    pub multiple_goals: bool,
    pub agents: Vec<Agent>,
    pub cached_background: Option<Rect>,
    pub cached_grid: RefCell<Option<HashMap<(i32, i32), Tile>>>,
}

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
            starts: Vec<(i32, i32)>,
            goals: Vec<(i32, i32)>,
            multiple_agents: bool,
            multiple_goals: bool,
            tiles: Vec<[String; 3]>, // Array of [position, type, weight]
        }

        let data = BoardData::deserialize(deserializer)?;

        // Rebuild grid from tiles
        let mut grid = HashMap::new();
        let tile_width = data.width / data.tile_amount_x;
        let tile_height = data.height / data.tile_amount_y;

        for tile_data in data.tiles {
            //println!("Yes");
            let pos_str = &tile_data[0];
            let type_str = &tile_data[1];
            let weight_str = &tile_data[2];
            let parts: Vec<&str> = pos_str.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                    let tile_type = match type_str.as_str() {
                        "Floor" => TileType::Floor,
                        "Obstacle" => TileType::Obstacle,
                        "Player" => TileType::Player,
                        "Enemy" => TileType::Enemy,
                        _ => TileType::Floor,
                    };
                    let weight = weight_str.parse::<u8>().unwrap();

                    let pos = (x, y);
                    grid.insert(
                        pos,
                        Tile::new(pos, tile_type, tile_height, tile_width, weight, false),
                    );
                }
            }
        }

        Ok(Board {
            location: Point::new(0, 0),
            height: data.height,
            width: data.width,
            tile_amount_x: data.tile_amount_x,
            tile_amount_y: data.tile_amount_y,
            selected_piece_type: TileType::Floor,
            id: String::from("game_board"),
            active: false,
            multiple_agents: data.multiple_agents,
            multiple_goals: data.multiple_goals,
            cached_background: None,
            cached_grid: RefCell::new(Some(grid)),
            agents: vec![],
            goals: data.goals,
            starts: data.starts,
        })
    }
}

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
        let grid = self.grid();
        let tiles: Vec<(String, TileType, String)> = grid
            .iter()
            .map(|(pos, tile)| {
                (
                    format!("{},{}", pos.0, pos.1),
                    tile.tile_type,
                    format!("{}", tile.weight),
                )
            })
            .collect();
        state.serialize_field("tiles", &tiles)?;
        state.end()
    }
}

impl Component for Board {
    /// Specify functionality for when the board is clicked on
    fn on_click(&mut self, mouse_point: Point) -> (bool, Option<String>) {
        let rect = self.get_rect();
        self.cached_background = Some(rect);
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

        let pos: (i32, i32) = (tile_x, tile_y);
        let mut grid: HashMap<(i32, i32), Tile> = self.grid();

        if let Some(_) = grid.get_mut(&pos) {
            tile_clicked = true;
        }
        if tile_clicked {
            if !self.multiple_agents && self.selected_piece_type == TileType::Player {
                grid.values_mut()
                    .filter(|t| t.tile_type == TileType::Player)
                    .for_each(|t| {
                        if t.position != pos {
                            t.change_tile_type(TileType::Floor);
                        }
                        self.starts.clear();
                    });
            }

            if !self.multiple_goals && self.selected_piece_type == TileType::Enemy {
                grid.values_mut()
                    .filter(|t| t.tile_type == TileType::Enemy)
                    .for_each(|t| {
                        if t.position != pos {
                            t.change_tile_type(TileType::Floor);
                        }
                        self.goals.clear();
                    });
            }

            if let Some(tile) = grid.get_mut(&pos) {
                match self.selected_piece_type {
                    TileType::Obstacle => tile.change_tile_type(TileType::Obstacle),
                    TileType::Enemy => {
                        if tile.tile_type != TileType::Enemy {
                            self.goals.push(pos);
                            tile.change_tile_type(TileType::Enemy);
                        }
                    }
                    TileType::Player => {
                        if tile.tile_type != TileType::Player {
                            self.starts.push(pos);
                            tile.change_tile_type(TileType::Player);
                        }
                    }
                    TileType::Weighted(weight) => {
                        tile.weight = weight;
                        tile.dirty = true;
                    }
                    _ => {}
                }

                self.cached_grid.borrow_mut().replace(grid);
                return (true, Some(self.get_id()));
            }
        }

        self.cached_grid.borrow_mut().replace(grid);
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
    pub fn load_board_file(&mut self, board_json: String) -> Self {
        let result: Board = serde_json::from_str(&board_json).expect("yes");
        return result;
    }
    /// Get grid information of board and set TileType
    pub fn grid(&self) -> HashMap<(i32, i32), Tile> {
        if let Some(map) = self.cached_grid.borrow().as_ref() {
            return map.clone(); // Only clone if needed
        }
        let mut grid = HashMap::new();
        let tile_width = self.tile_width();
        let tile_height = self.tile_height();
        for i in 0..self.tile_amount_x {
            for j in 0..self.tile_amount_y {
                let position: (i32, i32) = (i as i32, j as i32);
                let num: u8 = 1; //rand::rng().random_range(0..=255);
                grid.insert(
                    position,
                    Tile::new(
                        position,
                        TileType::Floor,
                        tile_height,
                        tile_width,
                        num,
                        true,
                    ),
                );
            }
        }
        grid
    }

    /// Get the width of the tiles to be generated
    pub fn tile_width(&self) -> u32 {
        self.width / self.tile_amount_x
    }
    /// Get the height of the tiles to be generated
    pub fn tile_height(&self) -> u32 {
        self.height / self.tile_amount_y
    }

    pub fn generate_random_grid(
        &self,
        weight_range: u8,
        obstacle_number: usize,
        weighted_number: usize,
    ) {
        let mut grid = HashMap::new();
        let tile_width = self.tile_width();
        let tile_height = self.tile_height();
        let mut rng = rand::rng();
        let obstacle_number = obstacle_number.min(
            (self.tile_amount_x * self.tile_amount_y / (2 * self.starts.len().max(1) as u32))
                as usize,
        );
        for i in 0..self.tile_amount_x {
            for j in 0..self.tile_amount_y {
                let position: (i32, i32) = (i as i32, j as i32);
                let num: u8 = 1;
                if self.starts.contains(&position) {
                    grid.insert(
                        position,
                        Tile::new(
                            position,
                            TileType::Player,
                            tile_height,
                            tile_width,
                            num,
                            true,
                        ),
                    );
                } else if self.goals.contains(&position) {
                    grid.insert(
                        position,
                        Tile::new(
                            position,
                            TileType::Enemy,
                            tile_height,
                            tile_width,
                            num,
                            true,
                        ),
                    );
                } else {
                    grid.insert(
                        position,
                        Tile::new(
                            position,
                            TileType::Floor,
                            tile_height,
                            tile_width,
                            num,
                            true,
                        ),
                    );
                }
            }
        }

        let new_grid = grid.clone();
        // Collect all floor tile positions
        let selected: Vec<&(i32, i32)> = new_grid
            .iter()
            .filter(|(_, t)| t.is_floor())
            .map(|(pos, _)| pos)
            .collect();
        let selected_length = selected.len();
        let total_special = (obstacle_number + weighted_number).min(selected_length);

        let selected = selected.iter().choose_multiple(&mut rng, total_special);

        // Choose unique positions for obstacles and weighted tiles in one pass

        // Assign obstacles
        for pos in selected.iter().take(obstacle_number) {
            if let Some(tile) = grid.get_mut(pos) {
                tile.change_tile_type(TileType::Obstacle);
            }
        }

        // Assign weighted tiles (skip those already made obstacles)
        for pos in selected.iter().skip(obstacle_number) {
            if let Some(tile) = grid.get_mut(pos) {
                let weight = rng.random_range(1..=weight_range.min(255));
                tile.weight = weight;
            }
        }

        self.cached_grid.borrow_mut().replace(grid);
    }

    pub fn save_to_file(
        &self,
        filepath: &str,
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        //println!("{}", json);
        fs::write(filepath.to_owned() + "/" + file_name.trim() + ".json", json)?;
        Ok(())
    }

    fn create_agents(&mut self) {
        if self.starts.len() != self.goals.len() {
            println!("Not all agents have a corresponding goal");
            return;
        }
        for i in 0..self.starts.len() {
            self.agents.push(Agent {
                start: self.starts[i],
                goal: self.goals[i],
                position: self.starts[i],
                path: vec![],
            })
        }
    }

    fn create_data_map(&self) -> HashMap<usize, PathData> {
        let mut data_map: HashMap<usize, PathData> = HashMap::new();
        for (i, _) in self.agents.iter().enumerate() {
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

    pub fn update_board(
        &mut self,
        canvas: &mut Canvas<Window>,
        algorithm: &str,
        doubling: bool,
        dyn_gen: bool,
        obstacles: u32,
        weighted_tiles: u32,
        iterations: u8,
        weight_range: u8,
    ) {
        if self.agents.is_empty() {
            self.create_agents();
        }
        let mut obstacles = obstacles as usize;
        let mut data_map: HashMap<usize, PathData> = self.create_data_map();
        for i in 0..iterations {
            let mut valid_iteration = false;
            while !valid_iteration {
                if doubling || dyn_gen {
                    self.generate_random_grid(
                        weight_range.min(255),
                        obstacles as usize,
                        weighted_tiles as usize,
                    );
                }
                if self.agents.is_empty() {
                    self.create_agents();
                }
                let grid: HashMap<(i32, i32), Tile> = self.grid();
                valid_iteration = true;
                let mut agents_completed_count = 0;
                while agents_completed_count != self.starts.len() {
                    let mut handles = vec![];
                    // Spawn threads - each gets its own grid copy
                    for i in 0..self.agents.len() {
                        let grid_clone = grid.clone(); // No Arc<Mutex>, just clone
                        let algorithm_str = algorithm.to_string();

                        let mut agent_clone = self.agents[i].clone();

                        let handle = thread::spawn(move || {
                            if agent_clone.is_path_possible(&grid_clone) {
                                let (_, path, wcf, memory, time, steps, path_cost) =
                                    agent_clone.get_path(&algorithm_str, &grid_clone);
                                (
                                    i,
                                    Some(path),
                                    Some(wcf),
                                    Some(memory),
                                    Some(time),
                                    Some(steps),
                                    Some(path_cost),
                                )
                            } else {
                                (i, None, None, None, None, None, None)
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
                                valid_iteration = false;
                                if !doubling || !dyn_gen {
                                    println!("No possible Path");
                                    return;
                                }
                                break;
                            } else {
                                // Update agent
                                self.agents[index].path = path.unwrap();
                                if let Some(agent_index) = data_map.get_mut(&index) {
                                    agent_index.update_all(
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

                    // Update the cached grid with all changes
                    self.cached_grid.replace(Some(grid.clone()));
                    self.draw(canvas);
                }
            }
            if i == iterations - 1 {
                // Mark all tiles as dirty so they redraw on last iteration
                let mut display_shown = false;
                while !display_shown {
                    let mut grid = self.grid();
                    // Mark all tiles as dirty so they redraw every frame
                    display_shown = true;
                    self.agents.iter_mut().for_each(|agent| {
                        let current = agent.position;
                        if !agent.goal_reached() {
                            display_shown = false;
                        }
                        if agent.path.len() > 0 {
                            let next = agent.path.pop().unwrap();
                            if let Some(tile) = grid.get_mut(&current) {
                                if tile.tile_type != TileType::Enemy {
                                    tile.change_tile_type(TileType::Floor);
                                }
                            }
                            if let Some(new_tile) = grid.get_mut(&next) {
                                if new_tile.tile_type != TileType::Enemy {
                                    new_tile.change_tile_type(TileType::Player);
                                }
                            }
                            agent.position = next;
                        }
                    });
                    self.cached_grid.replace(Some(grid));
                    self.draw(canvas);
                    canvas.present();
                    thread::sleep(Duration::from_millis(16));
                }

                for (_, data) in &data_map {
                    println!("{}", format!("{}", data));
                }
                fileDialog::save_data(&data_map);

                let mut grid = self.grid();
                self.agents.iter_mut().for_each(|agent| {
                    let current = agent.position;
                    let start = agent.start;
                    if let Some(tile) = grid.get_mut(&current) {
                        if tile.tile_type != TileType::Enemy {
                            tile.change_tile_type(TileType::Floor);
                        }
                    }
                    if let Some(new_tile) = grid.get_mut(&start) {
                        if new_tile.tile_type != TileType::Enemy {
                            new_tile.change_tile_type(TileType::Player);
                        }
                    }
                });
                self.cached_grid.replace(Some(grid));
                self.draw(canvas);
                canvas.present();
            }
            if doubling {
                obstacles *= 2;
            }
            self.draw(canvas);
            self.agents.clear();
            println!("{}", i);
        }
    }

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
    /// Draw Function for board
    pub fn draw<'a>(&self, canvas: &mut Canvas<Window>) {
        if self.cached_background.is_none() {
            canvas.set_draw_color(WHITE);
            canvas.fill_rect(self.get_rect()).unwrap();
        }
        let mut grid = self.grid();
        //println!("Starts: {:#?}\n Goals: {:#?}", self.starts, self.goals);
        for tile in grid.values_mut() {
            tile.draw(self.cached_background.is_none(), self.location, canvas);
        }
        self.cached_grid.borrow_mut().replace(grid);
    }
}
