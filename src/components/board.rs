use std::collections::{HashMap, HashSet};

extern crate sdl2;

use sdl2::mouse::MouseState;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use serde_json::{self, json};

#[derive(Copy, Clone)]
pub(crate) enum TileType {
    Obstacle,
    Floor,
    Player,
    Enemy,
}

#[derive(Copy, Clone)]
pub(crate) struct Tile {
    pub position: (u32, u32),
    tile_type: TileType,
    height: u32,
    width: u32,
}

pub(crate) struct Board {
    pub height: u32,
    pub width: u32,
    pub tile_amount_x: u32,
    pub tile_amount_y: u32,
    pub enemy_pos: HashSet<(u32, u32)>,
    pub player_pos: HashSet<(u32, u32)>,
    pub obstacles: HashSet<(u32, u32)>,
    pub selected_piece_type: TileType,
    pub active: bool,
}

impl Board {
    /// Get grid information of board and set TileType
    pub fn grid(&self) -> HashMap<(u32, u32), Tile> {
        let mut grid = HashMap::new();
        let tile_width = self.tile_width();
        let tile_height = self.tile_height();
        for i in 0..self.tile_amount_x {
            for j in 0..self.tile_amount_y {
                let position: (u32, u32) = (i, j);
                let tile_type = if self.obstacles.contains(&position) {
                    TileType::Obstacle
                } else if self.player_pos.contains(&position) {
                    TileType::Player
                } else if self.enemy_pos.contains(&position) {
                    TileType::Enemy
                } else {
                    TileType::Floor
                };

                let tile_coords = (0 + i * tile_width, 0 + j * tile_height);
                grid.insert(
                    position,
                    Tile {
                        position: tile_coords,
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
        grid: &HashMap<(u32, u32), Tile>,
    ) -> Vec<(sdl2::rect::Rect, TileType)> {
        let mut tile_dimensions: Vec<(sdl2::rect::Rect, TileType)> = Vec::new();
        for tile in grid.values() {
            let (x, y) = tile.position;
            tile_dimensions.push((
                sdl2::rect::Rect::new(x as i32, y as i32, tile.width, tile.height),
                tile.tile_type,
            ));
        }

        return tile_dimensions;
    }

    /// Specify functionality for when the board is clicked on
    pub fn on_click(&mut self, mouse_state: &MouseState) -> bool {
        let mouse_point = sdl2::rect::Point::new(mouse_state.x(), mouse_state.y());
        let grid = self.grid();
        let point = self
            .get_tile_information(&grid)
            .into_iter()
            .filter(|(_, tiletype)| matches!(tiletype, TileType::Floor))
            .find(|(rect, _)| rect.contains_point(mouse_point))
            .map(|(rect, _)| {
                (
                    rect.x() as u32 / self.tile_width(),
                    rect.y() as u32 / self.tile_height(),
                )
            });
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
            return true;
        }
        return false;
    }
    /// Draw Function for board
    pub fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        _: &'a TextureCreator<WindowContext>,
        _: Option<&MouseState>,
    ) {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas
            .fill_rect(Rect::new(0, 0, self.height, self.width))
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
