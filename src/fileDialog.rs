use std::{collections::HashSet, env, fs, path::Path};
use walkdir::{DirEntry, WalkDir};

use serde_json::{self, Value};
use std::collections::HashMap;

/// Walks Directory and gets file names for file selection

pub fn get_files() -> HashMap<String, String> {
    let current_dir = env::current_dir().unwrap();
    let mut file_map: HashMap<String, String> = HashMap::new();

    let json = std::ffi::OsStr::new("json");

    let file_dir: Vec<DirEntry> = WalkDir::new(current_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().unwrap().is_file() && e.path().extension() == Some(json))
        .collect();

    for file in file_dir {
        file_map.insert(
            file.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            file.path()
                .to_str()
                .expect("Couldn't convert to string")
                .to_string(),
        );
    }
    return file_map;
}

/// Parses specified json file and gets the stored tile locations to be implemented on the map.

pub fn parse_map_file(
    file_string: String,
) -> (
    HashSet<(u32, u32)>,
    HashSet<(u32, u32)>,
    HashSet<(u32, u32)>,
    u32,
    u32,
) {
    let json: Value = serde_json::from_str(&file_string).expect("JSON was incorrectly formatted");
    let tile_amount_x: u32 = json
        .get("tile_amount_x")
        .expect("no tile amount value")
        .as_i64()
        .expect("Invalid Number") as u32;
    let tile_amount_y: u32 = json
        .get("tile_amount_y")
        .expect("no tile amount value")
        .as_i64()
        .expect("Invalid Number") as u32;
    let mut obstacle_map: HashSet<(u32, u32)> = HashSet::new();
    let mut player_map: HashSet<(u32, u32)> = HashSet::new();
    let mut enemy_map: HashSet<(u32, u32)> = HashSet::new();
    if let Some(obstacles) = json["obstacles"].as_array() {
        obstacles.iter().for_each(|a| {
            if let Some(cords) = a.as_array() {
                obstacle_map.insert((
                    cords[0].as_i64().expect("Invalid Number") as u32,
                    cords[1].as_i64().expect("Invalid Number") as u32,
                ));
            }
        })
    }
    if let Some(players) = json["player"].as_array() {
        players.iter().for_each(|a| {
            if let Some(cords) = a.as_array() {
                player_map.insert((
                    cords[0].as_i64().expect("Invalid Number") as u32,
                    cords[1].as_i64().expect("Invalid Number") as u32,
                ));
            }
        })
    }
    if let Some(enemies) = json["enemies"].as_array() {
        enemies.iter().for_each(|a| {
            if let Some(cords) = a.as_array() {
                enemy_map.insert((
                    cords[0].as_i64().expect("Invalid Number") as u32,
                    cords[1].as_i64().expect("Invalid Number") as u32,
                ));
            }
        })
    }

    return (
        obstacle_map,
        player_map,
        enemy_map,
        tile_amount_x,
        tile_amount_y,
    );
}

/// Reads specified file and returns String value

pub fn read_file(path: &String) -> String {
    let path = Path::new(&path);
    match fs::read_to_string(path) {
        Ok(value) => {
            return value;
        }
        Err(e) => return format!("ERROR: {}", e),
    }
}

pub fn save_file(file_content: String) {
    fs::write("test.json", file_content).expect("bad");
}
