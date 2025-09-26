use rand::{seq::SliceRandom, thread_rng};

use std::collections::{hash_set::Intersection, HashMap, HashSet};

use crate::board::Tile;

fn distance_from_goal(start: (u32, u32), goal: (u32, u32)) -> u32 {
    return ((goal.0 as i32 - start.0 as i32).abs() + (goal.1 as i32 - start.1 as i32).abs())
        as u32;
}

pub(crate) struct Enemy {
    pub location: (u32, u32),
    pub black_list: HashSet<(u32, u32)>,
}

impl Enemy {
    fn possible_moves(
        &self,
        location: (u32, u32),
        map: &HashMap<(u32, u32), Tile>,
        obstacles: &HashSet<(u32, u32)>,
    ) -> Vec<(u32, u32)> {
        let mut possible_location: Vec<(u32, u32)> =
            vec![(location.0 + 1, location.1), (location.0, location.1 + 1)];
        if location.0 >= 1 {
            possible_location.push((location.0 - 1, location.1))
        }
        if location.1 >= 1 {
            possible_location.push((location.0, location.1 - 1))
        }

        let possible_location: Vec<(u32, u32)> = possible_location
            .into_iter()
            .filter(|a| {
                map.contains_key(a) && !obstacles.contains(a) && !self.black_list.contains(a)
            })
            .collect();
        return possible_location;
    }
    pub(crate) fn greedy_search(
        &mut self,
        location: (u32, u32),
        player_location: (u32, u32),
        map: &HashMap<(u32, u32), Tile>,
        obstacles: &HashSet<(u32, u32)>,
    ) -> (u32, u32) {
        let moves = self.possible_moves(location, map, obstacles);
        let mut good_moves: Vec<(u32, u32)> = Vec::new();
        let mut bad_moves: Vec<(u32, u32)> = Vec::new();
        if moves.len() <= 1 {
            self.black_list.clear()
        }
        for m in moves {
            if distance_from_goal(m, player_location)
                < distance_from_goal(location, player_location)
            {
                good_moves.push(m);
            } else {
                bad_moves.push(m);
            }
        }
        let good_moves: Vec<(u32, u32)> = good_moves.into_iter().collect();
        let mut rng = thread_rng();
        match good_moves.choose(&mut rng) {
            Some(value) => {
                self.black_list.clear();
                return *value;
            }
            _ => match bad_moves.choose(&mut rng) {
                Some(value) => {
                    self.black_list.insert(location);
                    return *value;
                }
                _ => return location,
            },
        }
    }
}
