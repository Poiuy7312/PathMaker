use std::collections::{HashMap, HashSet};

use sdl2::keyboard::Keycode;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
    None,
}

pub(crate) struct Player {
    pub position: (u32, u32),
    pub direction: Option<Direction>,
    pub speed: u32,
}

impl Player {
    pub fn is_tile_valid(
        grid: &HashMap<(u32, u32), Tile>,
        obstacles: &HashSet<(u32, u32)>,
        tile: (u32, u32),
    ) -> bool {
        let current_tile = grid.get(&tile);
        if current_tile.is_none() {
            return false;
        } else {
            if !obstacles.contains(&tile) {
                return true;
            } else {
                return false;
            }
        }
    }

    pub(crate) fn move_player(
        &self,
        current_position: (u32, u32),
        direction: Option<Direction>,
        speed: u32,
        map: &HashMap<(u32, u32), Tile>,
        obstacles: &HashSet<(u32, u32)>,
    ) -> (u32, u32) {
        let mut new_position: (u32, u32) = current_position;
        match direction.unwrap() {
            Direction::Right => {
                new_position = (current_position.0 + speed, current_position.1);
            }
            Direction::Left => {
                if current_position.0 >= 1 {
                    new_position = (current_position.0 - speed, current_position.1);
                }
            }
            Direction::Up => {
                if current_position.1 >= 1 {
                    new_position = (current_position.0, current_position.1 - speed);
                }
            }
            Direction::Down => {
                new_position = (current_position.0, current_position.1 + speed);
            }
            Direction::None => new_position = current_position,
        }
        if Self::is_tile_valid(map, obstacles, new_position) {
            return new_position;
        }
        return current_position;
    }
}
