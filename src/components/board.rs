use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::{fmt, fs};

extern crate sdl2;

use sdl2::mouse::MouseState;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::colors::*;
use crate::components::Component;
use crate::pathfinding::Agent;

#[derive(Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum TileType {
    Obstacle,
    Floor,
    Player,
    Enemy,
}

#[derive(Copy, Clone)]
pub struct Tile {
    pub position: (i32, i32),
    tile_type: TileType,
    height: u32,
    width: u32,
    dirty: bool,
    cached_rectangle: Option<Rect>,
}
impl Serialize for Tile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Tile", 5)?;
        state.serialize_field("position", &self.position)?;
        state.serialize_field("tile_type", &self.tile_type)?;
        state.serialize_field("height", &self.height)?;
        state.serialize_field("width", &self.width)?;
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
            TileType::Floor => {
                canvas.set_draw_color(WHITE);
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

    fn change_tile_type(&mut self, new_type: TileType) {
        if self.tile_type != new_type {
            self.tile_type = new_type;
        }
        self.dirty = true;
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
            multiple_agents: bool,
            multiple_goals: bool,
            tiles: Vec<[String; 2]>, // Array of [position, type]
        }

        let data = BoardData::deserialize(deserializer)?;

        // Rebuild grid from tiles
        let mut grid = HashMap::new();
        let tile_width = data.width / data.tile_amount_x;
        let tile_height = data.height / data.tile_amount_y;

        for tile_data in data.tiles {
            println!("Yes");
            let pos_str = &tile_data[0];
            let type_str = &tile_data[1];

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

                    let pos = (x, y);
                    grid.insert(
                        pos,
                        Tile::new(pos, tile_type, tile_height, tile_width, false),
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
            goals: vec![],
            starts: vec![],
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
        state.serialize_field("multiple_agents", &self.multiple_agents)?;
        state.serialize_field("multiple_goals", &self.multiple_goals)?;
        let grid = self.grid();
        let tiles: Vec<(String, TileType)> = grid
            .iter()
            .map(|(pos, tile)| (format!("{},{}", pos.0, pos.1), tile.tile_type))
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
                grid.insert(
                    position,
                    Tile::new(position, TileType::Floor, tile_height, tile_width, true),
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

    pub fn save_to_file(&self, filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        println!("{}", json);
        fs::write(filepath.to_owned() + "/test.json", json)?;
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

    pub fn update_board(&mut self, algorithm: &str) {
        if self.agents.is_empty() {
            self.create_agents();
        }
        let mut grid = self.grid();
        self.agents.iter_mut().for_each(|a| {
            let (cur_loc, new_loc) = a.get_next_move(algorithm, &grid);
            if let Some(tile) = grid.get_mut(&cur_loc) {
                tile.change_tile_type(TileType::Floor);
                if let Some(new_tile) = grid.get_mut(&new_loc) {
                    new_tile.change_tile_type(TileType::Player);
                }
            }
        });
        self.cached_grid.borrow_mut().replace(grid);
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
        println!("Starts: {:#?}\n Goals: {:#?}", self.starts, self.goals);
        for tile in grid.values_mut() {
            tile.draw(self.cached_background.is_none(), self.location, canvas);
        }
        self.cached_grid.borrow_mut().replace(grid);
    }
}
