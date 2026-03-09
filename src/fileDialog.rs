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

/// Build a DirectoryNode tree from a filesystem path, scanning only one level deep.
///
/// Only includes JSON files and directories (excluding hidden entries).
/// Child directories are listed but their contents are **not** recursively scanned;
/// they are loaded on demand via `ensure_children_loaded()`.
///
/// # Arguments
/// * `path` - Root path to start building from
///
/// # Returns
/// A DirectoryNode representing the path and its immediate children
fn build_shallow(path: &Path) -> DirectoryNode {
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
                    let child_name = p
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| p.to_string_lossy().into_owned());
                    if p.is_dir() {
                        // Add the directory entry but don't recurse into it
                        node.children.push(DirectoryNode {
                            name: child_name,
                            path: p,
                            is_dir: true,
                            children: Vec::new(),
                        });
                    } else if let Some(ext) = p.extension() {
                        if ext == allowed_extension {
                            node.children.push(DirectoryNode {
                                name: child_name,
                                path: p,
                                is_dir: false,
                                children: Vec::new(),
                            });
                        }
                    }
                }
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

/// Get the user's home directory (cross-platform).
///
/// Uses `USERPROFILE` on Windows and `HOME` on Unix.
///
/// # Returns
/// PathBuf to the home directory
pub fn get_current_directory() -> PathBuf {
    if cfg!(target_os = "windows") {
        env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\"))
    } else {
        env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"))
    }
}

/// Build a shallow file tree starting from the home directory.
///
/// Only the immediate children of the home directory are scanned.
/// Subdirectory contents are loaded lazily via `ensure_children_loaded()`.
///
/// # Returns
/// DirectoryNode tree rooted at the user's home directory
pub fn get_file_tree() -> DirectoryNode {
    let current_dir = get_current_directory();
    build_shallow(&current_dir)
}

/// Lazily load the immediate children of a directory into the shared directory map.
///
/// When the user navigates into a directory that hasn't been scanned yet,
/// this function reads its contents from disk and inserts the entries into
/// the provided `directories` map. If the directory has already been loaded
/// (i.e. its key exists with children), this is a no-op.
///
/// # Arguments
/// * `directories` - Shared directory map to populate
/// * `dir_path` - Path of the directory to scan
pub fn ensure_children_loaded(
    directories: &std::rc::Rc<
        std::cell::RefCell<
            HashMap<String, (crate::components::button::StandardButton, Vec<String>)>,
        >,
    >,
    dir_path: &str,
) {
    use crate::colors::*;
    use crate::components::button::StandardButton;
    use sdl2::rect::Point;

    // Check if this directory already has children loaded
    {
        let map = directories.borrow();
        if let Some((_, children)) = map.get(dir_path) {
            if !children.is_empty() {
                return; // Already loaded
            }
        }
    }

    // Scan the directory from disk
    let node = build_shallow(std::path::Path::new(dir_path));
    if !node.is_dir {
        return;
    }

    let mut child_paths: Vec<String> = Vec::new();
    let mut new_entries: Vec<(String, StandardButton, bool)> = Vec::new();

    for child in &node.children {
        let child_path = child.path.to_string_lossy().to_string();
        child_paths.push(child_path.clone());
        new_entries.push((
            child_path.clone(),
            StandardButton {
                height: 25,
                width: 200,
                location: Point::new(0, 62),
                text_color: WHITE,
                background_color: QUATERNARY_COLOR,
                hover: std::cell::RefCell::new(false),
                text: child.name.clone(),
                id: child_path,
                active: false,
                filter: None,
                drawn: std::cell::RefCell::new(false),
                cached_texture: None,
            },
            child.is_dir,
        ));
    }

    let mut map = directories.borrow_mut();

    // Insert child entries that don't already exist
    for (path, button, is_dir) in new_entries {
        map.entry(path)
            .or_insert_with(|| (button, if is_dir { Vec::new() } else { Vec::new() }));
    }

    // Update the parent's children list
    if let Some((_, children)) = map.get_mut(dir_path) {
        *children = child_paths;
    } else {
        // Parent wasn't in the map yet — insert it
        let parent_name = std::path::Path::new(dir_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| dir_path.to_string());
        map.insert(
            dir_path.to_string(),
            (
                StandardButton {
                    height: 25,
                    width: 200,
                    location: Point::new(0, 62),
                    text_color: WHITE,
                    background_color: QUATERNARY_COLOR,
                    hover: std::cell::RefCell::new(false),
                    text: parent_name,
                    id: dir_path.to_string(),
                    active: false,
                    filter: None,
                    drawn: std::cell::RefCell::new(false),
                    cached_texture: None,
                },
                child_paths,
            ),
        );
    }
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

    // ------- build_shallow -------

    #[test]
    fn test_build_shallow_on_tmp() {
        let tree = build_shallow(Path::new("/tmp"));
        assert_eq!(tree.name, "tmp");
        assert!(tree.is_dir);
    }

    #[test]
    fn test_build_shallow_file_node() {
        let dir = std::env::temp_dir();
        let path = dir.join("pathmaker_tree_test.json");
        std::fs::write(&path, "{}").unwrap();
        let tree = build_shallow(&path);
        assert!(!tree.is_dir);
        assert!(tree.children.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
