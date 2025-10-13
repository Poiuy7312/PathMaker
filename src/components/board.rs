use std::collections::{HashMap, HashSet};

extern crate sdl2;

use sdl2::mouse::MouseState;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use serde_json::{self, json};

use crate::components::Component;

#[derive(Copy, Clone)]
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
}

pub struct Board {
    pub location: Point,
    pub height: u32,
    pub width: u32,
    pub tile_amount_x: u32,
    pub tile_amount_y: u32,
    pub enemy_pos: HashSet<(i32, i32)>,
    pub player_pos: HashSet<(i32, i32)>,
    pub obstacles: HashSet<(i32, i32)>,
    pub selected_piece_type: TileType,
    pub id: String,
    pub active: bool,
}

impl Component for Board {
    /// Specify functionality for when the board is clicked on
    fn on_click(&mut self, mouse_point: Point) -> (bool, Option<&str>) {
        let grid = self.grid();
        let point = self
            .get_tile_information(&grid)
            .into_iter()
            .filter(|(_, tiletype)| matches!(tiletype, TileType::Floor))
            .find(|(rect, _)| rect.contains_point(mouse_point))
            .map(|(rect, _)| (rect.x() - self.location.x(), rect.y() - self.location.y()));
        if point.is_some() {
            match self.selected_piece_type {
                TileType::Obstacle => {
                    self.obstacles
                        .insert(point.expect("Point not provided when should have been"));
                }
                TileType::Enemy => {
                    self.enemy_pos
                        .insert(point.expect("Point not provided when should have been"));
                }
                TileType::Player => {
                    self.player_pos
                        .insert(point.expect("Point not provided when should have been"));
                }
                _ => {}
            }
            return (true, Some(self.get_id()));
        }
        return (false, None);
    }

    fn get_id(&self) -> &str {
        return &self.id;
    }

    fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }

    fn get_width(&self) -> u32 {
        self.width
    }
    fn get_height(&self) -> u32 {
        self.height
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl Board {
    /// Get grid information of board and set TileType
    pub fn grid(&self) -> HashMap<(i32, i32), Tile> {
        let mut grid = HashMap::new();
        let tile_width = self.tile_width();
        let tile_height = self.tile_height();
        for i in 0..self.tile_amount_x {
            for j in 0..self.tile_amount_y {
                let position: (i32, i32) =
                    (i as i32 * tile_width as i32, j as i32 * tile_height as i32);
                let tile_type = if self.obstacles.contains(&position) {
                    TileType::Obstacle
                } else if self.player_pos.contains(&position) {
                    TileType::Player
                } else if self.enemy_pos.contains(&position) {
                    TileType::Enemy
                } else {
                    TileType::Floor
                };

                grid.insert(
                    position,
                    Tile {
                        position: position,
                        tile_type: tile_type,
                        height: tile_height,
                        width: tile_width,
                    },
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

    /// Return json representation of grid information for saving to file
    pub fn map_json(&self) -> String {
        let json_string = json!(
                {
                    "tile_amount_x": self.tile_amount_x,
                    "tile_amount_y": self.tile_amount_y,
                    "player": self.player_pos,
                    "enemies": self.enemy_pos,
                    "obstacles": self.obstacles
                }
        );
        return json_string.to_string();
    }

    /// Get tile type and rectangle to draw the tiles
    pub fn get_tile_information(
        &self,
        grid: &HashMap<(i32, i32), Tile>,
    ) -> Vec<(sdl2::rect::Rect, TileType)> {
        let mut tile_dimensions: Vec<(sdl2::rect::Rect, TileType)> = Vec::new();
        for tile in grid.values() {
            let (x, y) = tile.position;
            tile_dimensions.push((
                sdl2::rect::Rect::new(
                    self.location.x() + x,
                    self.location.y() + y,
                    tile.width,
                    tile.height,
                ),
                tile.tile_type,
            ));
        }

        return tile_dimensions;
    }
    /// Draw Function for board
    pub fn draw<'a>(&self, canvas: &mut Canvas<Window>) {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas
            .fill_rect(Rect::new(
                self.location.x(),
                self.location.y(),
                self.height,
                self.width,
            ))
            .unwrap();
        let grid = self.grid();
        let tiles = self.get_tile_information(&grid);
        for tile in tiles.iter() {
            match tile.1 {
                TileType::Obstacle => {
                    canvas.set_draw_color(Color::RGB(0, 0, 0));
                    canvas.fill_rect(tile.0).unwrap();
                }
                TileType::Floor => {}
                TileType::Player => {
                    canvas.set_draw_color(Color::RGB(0, 255, 0));
                    canvas.fill_rect(tile.0).unwrap();
                }
                TileType::Enemy => {
                    canvas.set_draw_color(Color::RGB(255, 0, 0));
                    canvas.fill_rect(tile.0).unwrap();
                }
            }
        }
    }
}
