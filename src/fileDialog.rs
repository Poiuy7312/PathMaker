use std::{
    collections::HashSet,
    env,
    ffi::{OsStr, OsString},
    fs::{read_dir, read_to_string, write, DirEntry},
    path::{Path, PathBuf},
};

use serde_json::{self, Value};
use std::collections::HashMap;

use crate::benchmarks::PathData;

/// Walks Directory and gets file names for file selection

pub struct DirectoryNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<DirectoryNode>,
}

pub fn is_directory(path: &str) -> bool {
    let path = Path::new(&path);
    return path.is_dir();
}

/// Build a DirectoryNode for `path` recursively.
fn build_tree(path: &Path) -> DirectoryNode {
    let allowed_extension = OsStr::new("json");
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    let mut node = DirectoryNode {
        name,
        path: path.to_path_buf(),
        is_dir: path.is_dir(),
        children: Vec::new(),
    };

    if node.is_dir {
        if let Ok(entries) = read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if is_not_hidden(&entry) {
                    let p = entry.path();
                    if p.is_dir() {
                        node.children.push(build_tree(&p));
                    } else {
                        match p.extension() {
                            Some(ext) => {
                                if ext == allowed_extension {
                                    node.children.push(build_tree(&p));
                                }
                            }
                            None => {}
                        }
                    }
                }
                // Skip hidden entries if desired, or add filtering here.
            }
        }
    }

    node
}

fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with('.'))
        .unwrap_or(false)
}

pub fn get_current_directory() -> PathBuf {
    return env::home_dir().expect("No home directory");
}

/// Returns a tree rooted at the current working directory.
pub fn get_file_tree() -> DirectoryNode {
    let current_dir = get_current_directory();
    build_tree(&current_dir)
}

fn print_tree(node: &DirectoryNode, indent: usize) {
    let indent_str = " ".repeat(indent);
    let kind = if node.is_dir { "[D]" } else { "[F]" };
    println!(
        "{}{} {} ({})",
        indent_str,
        kind,
        node.name,
        node.path.display()
    );
    for child in &node.children {
        print_tree(child, indent + 2);
    }
}

/// Walks Directory and gets file names for file selection
///

/// Parses specified json file and gets the stored tile locations to be implemented on the map.

pub fn parse_map_file(
    file_string: String,
) -> (
    HashSet<(i32, i32)>,
    HashSet<(i32, i32)>,
    HashSet<(i32, i32)>,
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
    let mut obstacle_map: HashSet<(i32, i32)> = HashSet::new();
    let mut player_map: HashSet<(i32, i32)> = HashSet::new();
    let mut enemy_map: HashSet<(i32, i32)> = HashSet::new();
    if let Some(obstacles) = json["obstacles"].as_array() {
        obstacles.iter().for_each(|a| {
            if let Some(cords) = a.as_array() {
                obstacle_map.insert((
                    cords[0].as_i64().expect("Invalid Number") as i32,
                    cords[1].as_i64().expect("Invalid Number") as i32,
                ));
            }
        })
    }
    if let Some(players) = json["player"].as_array() {
        players.iter().for_each(|a| {
            if let Some(cords) = a.as_array() {
                player_map.insert((
                    cords[0].as_i64().expect("Invalid Number") as i32,
                    cords[1].as_i64().expect("Invalid Number") as i32,
                ));
            }
        })
    }
    if let Some(enemies) = json["enemies"].as_array() {
        enemies.iter().for_each(|a| {
            if let Some(cords) = a.as_array() {
                enemy_map.insert((
                    cords[0].as_i64().expect("Invalid Number") as i32,
                    cords[1].as_i64().expect("Invalid Number") as i32,
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

pub fn read_file(path: &str) -> String {
    let path = Path::new(&path);
    println!("Reading File: {:#?}", path);
    match read_to_string(path) {
        Ok(value) => {
            return value;
        }
        Err(e) => return format!("ERROR: {}", e),
    }
}

pub fn save_file(path: String, file_content: String) {
    let path = path + "/test.json";
    println!("{}", path);
    write(path, file_content).expect("bad");
}

pub fn save_data(data: &HashMap<usize, PathData>) {
    let results = serde_json::to_string_pretty(data).expect("N");
    write(
        env::current_dir()
            .expect("No directory")
            .to_str()
            .expect("I")
            .to_string()
            + "/data.json",
        results,
    )
    .unwrap();
}
