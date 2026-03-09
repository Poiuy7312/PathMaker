//! # File Dialog Module
//!
//! This module handles file system operations for loading and saving map files:
//!
//! ## Features
//! - Directory tree traversal and caching
//! - JSON map file parsing and writing
//! - Benchmark data persistence
//!
//! ## File Format
//! Maps are stored as JSON files containing:
//! - Tile dimensions and amounts
//! - Obstacle positions
//! - Player and enemy positions
//! - Tile weights

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

/// Represents a node in the directory tree structure.
///
/// Used by the file explorer to navigate the file system.
pub struct DirectoryNode {
    /// Display name of the file or directory
    pub name: String,
    /// Full path to the file or directory
    pub path: PathBuf,
    /// True if this node represents a directory
    pub is_dir: bool,
    /// Child nodes (empty for files)
    pub children: Vec<DirectoryNode>,
}

/// Check if a path points to a directory.
///
/// # Arguments
/// * `path` - String path to check
///
/// # Returns
/// `true` if the path is a directory
pub fn is_directory(path: &str) -> bool {
    let path = Path::new(&path);
    return path.is_dir();
}

/// Recursively build a DirectoryNode tree from a filesystem path.
///
/// Only includes JSON files and directories (excluding hidden entries).
/// This creates the data structure used by the file explorer component.
///
/// # Arguments
/// * `path` - Root path to start building from
///
/// # Returns
/// A DirectoryNode representing the path and all its contents
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

/// Check if a directory entry is not hidden (doesn't start with '.').
///
/// # Arguments
/// * `entry` - Directory entry to check
///
/// # Returns
/// `true` if the entry is visible (not hidden)
fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with('.'))
        .unwrap_or(false)
}

/// Get the user's home directory.
///
/// # Returns
/// PathBuf to the home directory
pub fn get_current_directory() -> PathBuf {
    return env::home_dir().expect("No home directory");
}

/// Build a complete file tree starting from the home directory.
///
/// # Returns
/// DirectoryNode tree rooted at the user's home directory
pub fn get_file_tree() -> DirectoryNode {
    let current_dir = get_current_directory();
    build_tree(&current_dir)
}

/// Debug utility to print the directory tree structure.
///
/// # Arguments
/// * `node` - Node to print
/// * `indent` - Current indentation level
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

/// Parse a map JSON file and extract tile positions.
///
/// Reads a JSON map file and extracts sets of coordinates for:
/// - Obstacles
/// - Player start positions
/// - Enemy/goal positions
///
/// # Arguments
/// * `file_string` - Contents of the JSON file as a string
///
/// # Returns
/// Tuple containing:
/// - Set of obstacle coordinates
/// - Set of player coordinates
/// - Set of enemy coordinates
/// - Tile count in X direction
/// - Tile count in Y direction
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

/// Read a file's contents as a string.
///
/// # Arguments
/// * `path` - Path to the file to read
///
/// # Returns
/// File contents as a string, or an error message
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

/// Save content to a file (appends /test.json to path).
///
/// # Arguments
/// * `path` - Directory path
/// * `file_content` - Content to write
pub fn save_file(path: String, file_content: String) {
    let path = path + "/test.json";
    println!("{}", path);
    write(path, file_content).expect("bad");
}

/// Save benchmark data to a JSON file.
///
/// Writes pathfinding performance data to `data.json` in the current directory.
///
/// # Arguments
/// * `data` - HashMap of agent index to PathData containing benchmark metrics
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

#[cfg(test)]
mod tests {
    use super::*;

    // ------- is_directory -------

    #[test]
    fn test_is_directory_on_tmp() {
        assert!(is_directory("/tmp"));
    }

    #[test]
    fn test_is_directory_on_file() {
        // A well-known file that exists on Linux
        assert!(!is_directory("/etc/hostname"));
    }

    #[test]
    fn test_is_directory_nonexistent() {
        assert!(!is_directory("/nonexistent_path_abc_xyz"));
    }

    // ------- is_not_hidden -------

    #[test]
    fn test_is_not_hidden_on_tmp() {
        // /tmp exists and is not hidden
        if let Ok(entries) = std::fs::read_dir("/tmp") {
            for entry in entries.filter_map(|e| e.ok()).take(1) {
                let name = entry.file_name().to_string_lossy().to_string();
                let result = is_not_hidden(&entry);
                if name.starts_with('.') {
                    assert!(!result);
                } else {
                    assert!(result);
                }
            }
        }
    }

    // ------- get_current_directory -------

    #[test]
    fn test_get_current_directory_exists() {
        let dir = get_current_directory();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    // ------- parse_map_file -------

    #[test]
    fn test_parse_map_file_basic() {
        let json = r#"{
            "tile_amount_x": 10,
            "tile_amount_y": 10,
            "obstacles": [[1, 2], [3, 4]],
            "player": [[0, 0]],
            "enemies": [[9, 9]]
        }"#;
        let (obstacles, players, enemies, tx, ty) = parse_map_file(json.to_string());
        assert_eq!(tx, 10);
        assert_eq!(ty, 10);
        assert_eq!(obstacles.len(), 2);
        assert!(obstacles.contains(&(1, 2)));
        assert!(obstacles.contains(&(3, 4)));
        assert_eq!(players.len(), 1);
        assert!(players.contains(&(0, 0)));
        assert_eq!(enemies.len(), 1);
        assert!(enemies.contains(&(9, 9)));
    }

    #[test]
    fn test_parse_map_file_no_obstacles() {
        let json = r#"{
            "tile_amount_x": 5,
            "tile_amount_y": 5,
            "obstacles": [],
            "player": [],
            "enemies": []
        }"#;
        let (obstacles, players, enemies, tx, ty) = parse_map_file(json.to_string());
        assert_eq!(tx, 5);
        assert_eq!(ty, 5);
        assert!(obstacles.is_empty());
        assert!(players.is_empty());
        assert!(enemies.is_empty());
    }

    #[test]
    fn test_parse_map_file_missing_optional_fields() {
        // Only tile_amount required, obstacles/player/enemies can be absent
        let json = r#"{
            "tile_amount_x": 20,
            "tile_amount_y": 15
        }"#;
        let (obstacles, players, enemies, tx, ty) = parse_map_file(json.to_string());
        assert_eq!(tx, 20);
        assert_eq!(ty, 15);
        assert!(obstacles.is_empty());
        assert!(players.is_empty());
        assert!(enemies.is_empty());
    }

    #[test]
    fn test_parse_map_file_multiple_agents() {
        let json = r#"{
            "tile_amount_x": 10,
            "tile_amount_y": 10,
            "obstacles": [],
            "player": [[0, 0], [1, 1], [2, 2]],
            "enemies": [[9, 9], [8, 8], [7, 7]]
        }"#;
        let (_, players, enemies, _, _) = parse_map_file(json.to_string());
        assert_eq!(players.len(), 3);
        assert_eq!(enemies.len(), 3);
    }

    // ------- read_file -------

    #[test]
    fn test_read_file_nonexistent() {
        let result = read_file("/tmp/this_file_does_not_exist_xyz.json");
        assert!(result.starts_with("ERROR"));
    }

    #[test]
    fn test_read_file_existing() {
        let dir = std::env::temp_dir();
        let path = dir.join("pathmaker_test_read.txt");
        std::fs::write(&path, "hello world").unwrap();
        let result = read_file(path.to_str().unwrap());
        assert_eq!(result, "hello world");
        let _ = std::fs::remove_file(&path);
    }

    // ------- save_data -------

    #[test]
    fn test_save_data_creates_file() {
        let data: HashMap<usize, crate::benchmarks::PathData> = HashMap::new();
        // This writes to data.json in cwd — just verify it doesn't panic
        save_data(&data);
        let cwd = env::current_dir().unwrap();
        let data_path = cwd.join("data.json");
        assert!(data_path.exists());
    }

    // ------- build_tree -------

    #[test]
    fn test_build_tree_on_tmp() {
        let tree = build_tree(Path::new("/tmp"));
        assert_eq!(tree.name, "tmp");
        assert!(tree.is_dir);
    }

    #[test]
    fn test_build_tree_file_node() {
        let dir = std::env::temp_dir();
        let path = dir.join("pathmaker_tree_test.json");
        std::fs::write(&path, "{}").unwrap();
        let tree = build_tree(&path);
        assert!(!tree.is_dir);
        assert!(tree.children.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
