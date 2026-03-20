//! # Utility Functions Module
//!
//! This module provides helper functions used throughout the application for:
//! - Mouse position checking
//! - Font size calculations
//! - Directory tree structure management
//! - File browser data handling

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use sdl2::rect::{Point, Rect};

use crate::{components::button::*, fileDialog};

/// Check if the mouse position is within a given rectangle.
///
/// # Arguments
/// * `rect` - The bounding rectangle to check against
/// * `mouse_position` - Current mouse cursor position
///
/// # Returns
/// `true` if the mouse is within the rectangle bounds
pub fn mouse_over(rect: Rect, mouse_position: Point) -> bool {
    return rect.contains_point(mouse_position);
}

use crate::colors::*;

/*
/// Deprecated
///
pub fn walk_tree(
    node: &fileDialog::DirectoryNode,
    width: u32,
) -> Vec<Box<dyn ValidDropdownOption>> {
    let mut buttons: Vec<Box<dyn ValidDropdownOption>> = Vec::new();
    if node.is_dir {
        for child in &node.children {
            if child.is_dir {
                buttons.push(Box::new(Dropdown {
                    height: 25,
                    width: 200,
                    location: Point::new(width as i32 - 200, 62),
                    text_color: WHITE,
                    background_color: QUATERNARY_COLOR,
                    hover_color: SECONDARY_COLOR,
                    text: child.name.to_string(),
                    id: child.path.to_string_lossy().to_string(),
                    active: false,
                    clicked_on: false,
                    filter: None,
                    options: walk_tree(child, width).into_iter().collect(),
                    drawn: false,
                }))
            } else {
                buttons.push(Box::new(StandardButton {
                    height: 25,
                    width: 200,
                    location: Point::new(width as i32 - 200, 62),
                    text_color: WHITE,
                    background_color: SECONDARY_COLOR,
                    hover_color: WHITE,
                    text: child.name.to_string(),
                    id: child.path.to_string_lossy().to_string(),
                    filter: None,
                    active: false,
                    drawn: false,
                }))
            }
        }
    }
    return buttons;
}

*/

/// Calculate an appropriate font size that fits within the available width.
///
/// Uses a simple estimation based on average character width (8 pixels).
/// Scales down the font size if the text would overflow.
///
/// # Arguments
/// * `text_len` - Number of characters in the text
/// * `available_width` - Maximum width available for the text in pixels
///
/// # Returns
/// Calculated font size in pixels (minimum 4)
pub fn calculate_scaled_font_size(text_len: u32, available_width: u32) -> u32 {
    let char_width = 8;
    let estimated_width = text_len * char_width;

    if estimated_width > available_width {
        let scale = available_width as f32 / estimated_width as f32;
        (estimated_width as f32 * scale) as u32
    } else {
        estimated_width
    }
    .max(4) // minimum font size
}

/// Add a new file entry to the directory map data structure.
///
/// This function creates a new StandardButton for the file and adds it
/// to both the flat map and its parent directory's children list.
///
/// # Arguments
/// * `directories` - Shared reference to the directory map
/// * `path` - Parent directory path
/// * `file_name` - Name of the file (without .json extension)
pub fn add_file_to_dir_map(
    directories: Rc<RefCell<HashMap<String, (StandardButton, Vec<String>)>>>,
    path: String,
    file_name: &str,
) {
    let full_path = path.clone() + "/" + file_name + ".json";
    directories.borrow_mut().insert(
        full_path.to_string(),
        (
            StandardButton {
                height: 25,
                width: 200,
                location: Point::new(0, 62),
                text_color: WHITE,
                background_color: QUATERNARY_COLOR,
                hover: RefCell::new(false),
                text: file_name.to_string(),
                id: full_path.to_string(),
                active: false,
                filter: None,
                drawn: RefCell::new(false),
                cached_texture: None,
            },
            Vec::new(),
        ),
    );
    if let Some(directory) = directories.borrow_mut().get_mut(&path) {
        directory.1.push(full_path);
    }
}

#[inline]
pub fn get_coordinate_from_idx(idx: usize, x_size: u32, _y_size: u32) -> (i32, i32) {
    let x = (idx % x_size as usize) as i32;
    let y = (idx / x_size as usize) as i32;
    (x, y)
}

#[inline]
pub fn get_idx_from_coordinate(pos: (i32, i32), width: u32, height: u32) -> Option<usize> {
    if pos.0 < 0 || pos.1 < 0 || pos.0 >= width as i32 || pos.1 >= height as i32 {
        return None;
    }
    Some(pos.1 as usize * width as usize + pos.0 as usize)
}

/// Convert a shallow directory tree into a flat HashMap for the initial view.
///
/// Only processes the root node and its immediate children (one level).
/// Deeper directories are loaded on demand via `fileDialog::ensure_children_loaded()`.
///
/// Creates a map where:
/// - Keys are full file/directory paths
/// - Values are tuples of (StandardButton, Vec<child_paths>)
///
/// # Arguments
/// * `node` - Root node of the directory tree to convert
/// * `width` - Window width (used for button positioning)
///
/// # Returns
/// HashMap mapping paths to button/children pairs
pub fn get_dir_map(
    node: &fileDialog::DirectoryNode,
    width: u32,
) -> HashMap<String, (StandardButton, Vec<String>)> {
    let mut buttons: Vec<String> = Vec::new();
    let mut map: HashMap<String, (StandardButton, Vec<String>)> = HashMap::new();

    if node.is_dir {
        let current_button = StandardButton {
            height: 25,
            width: 200,
            location: Point::new(0, 62),
            text_color: WHITE,
            background_color: QUATERNARY_COLOR,
            hover: RefCell::new(false),
            text: node.name.to_string(),
            id: node.path.to_string_lossy().to_string(),
            active: false,
            drawn: RefCell::new(false),
            filter: None,
            cached_texture: None,
        };

        for child in &node.children {
            let child_path = child.path.to_string_lossy().to_string();
            buttons.push(child_path.clone());

            // Insert the child entry (directories get an empty children vec,
            // which signals that their contents haven't been loaded yet)
            map.insert(
                child_path.clone(),
                (
                    StandardButton {
                        height: 25,
                        width: 200,
                        location: Point::new(0, 62),
                        text_color: WHITE,
                        background_color: QUATERNARY_COLOR,
                        hover: RefCell::new(false),
                        text: child.name.to_string(),
                        id: child_path,
                        active: false,
                        filter: None,
                        drawn: RefCell::new(false),
                        cached_texture: None,
                    },
                    Vec::new(),
                ),
            );
        }

        map.insert(
            node.path.to_string_lossy().to_string(),
            (current_button, buttons),
        );
    }
    return map;
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::rect::{Point, Rect};
    use std::cell::RefCell;

    // ------- get_coordinate_from_idx -------

    #[test]
    fn test_get_coordinate_from_idx_basic() {
        let (x, y) = get_coordinate_from_idx(0, 10, 10);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn test_get_coordinate_from_idx_second_row() {
        let (x, y) = get_coordinate_from_idx(10, 10, 10);
        assert_eq!(x, 0);
        assert_eq!(y, 1);
    }

    #[test]
    fn test_get_coordinate_from_idx_middle() {
        let (x, y) = get_coordinate_from_idx(15, 10, 10);
        assert_eq!(x, 5);
        assert_eq!(y, 1);
    }

    #[test]
    fn test_get_coordinate_from_idx_last() {
        let (x, y) = get_coordinate_from_idx(99, 10, 10);
        assert_eq!(x, 9);
        assert_eq!(y, 9);
    }

    // ------- get_idx_from_coordinate -------

    #[test]
    fn test_get_idx_from_coordinate_basic() {
        let idx = get_idx_from_coordinate((0, 0), 10, 10);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn test_get_idx_from_coordinate_second_row() {
        let idx = get_idx_from_coordinate((0, 1), 10, 10);
        assert_eq!(idx, Some(10));
    }

    #[test]
    fn test_get_idx_from_coordinate_middle() {
        let idx = get_idx_from_coordinate((5, 1), 10, 10);
        assert_eq!(idx, Some(15));
    }

    #[test]
    fn test_get_idx_from_coordinate_out_of_bounds_negative() {
        assert_eq!(get_idx_from_coordinate((-1, 0), 10, 10), None);
        assert_eq!(get_idx_from_coordinate((0, -1), 10, 10), None);
    }

    #[test]
    fn test_get_idx_from_coordinate_out_of_bounds_too_large() {
        assert_eq!(get_idx_from_coordinate((10, 0), 10, 10), None);
        assert_eq!(get_idx_from_coordinate((0, 10), 10, 10), None);
    }

    // ------- add_file_to_dir_map -------

    #[test]
    fn test_add_file_to_dir_map_inserts_file() {
        let directories: Rc<RefCell<HashMap<String, (StandardButton, Vec<String>)>>> =
            Rc::new(RefCell::new(HashMap::new()));

        directories.borrow_mut().insert(
            "/home".to_string(),
            (
                StandardButton {
                    height: 25,
                    width: 200,
                    location: Point::new(0, 62),
                    text_color: WHITE,
                    background_color: QUATERNARY_COLOR,
                    hover: RefCell::new(false),
                    text: "home".to_string(),
                    id: "/home".to_string(),
                    active: false,
                    filter: None,
                    drawn: RefCell::new(false),
                    cached_texture: None,
                },
                Vec::new(),
            ),
        );

        add_file_to_dir_map(directories.clone(), "/home".to_string(), "test_map");

        let map = directories.borrow();
        let full_path = "/home/test_map.json";
        assert!(map.contains_key(full_path));
        let (btn, children) = map.get(full_path).unwrap();
        assert_eq!(btn.text, "test_map");
        assert_eq!(children.len(), 0);
    }

    #[test]
    fn test_add_file_to_dir_map_updates_parent_children() {
        let directories: Rc<RefCell<HashMap<String, (StandardButton, Vec<String>)>>> =
            Rc::new(RefCell::new(HashMap::new()));

        directories.borrow_mut().insert(
            "/home".to_string(),
            (
                StandardButton {
                    height: 25,
                    width: 200,
                    location: Point::new(0, 62),
                    text_color: WHITE,
                    background_color: QUATERNARY_COLOR,
                    hover: RefCell::new(false),
                    text: "home".to_string(),
                    id: "/home".to_string(),
                    active: false,
                    filter: None,
                    drawn: RefCell::new(false),
                    cached_texture: None,
                },
                Vec::new(),
            ),
        );

        add_file_to_dir_map(directories.clone(), "/home".to_string(), "test_map");

        let map = directories.borrow();
        let (_, children) = map.get("/home").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], "/home/test_map.json");
    }

    // ------- get_dir_map -------

    #[test]
    fn test_get_dir_map_empty_node() {
        let node = fileDialog::DirectoryNode {
            name: "empty".to_string(),
            path: std::path::PathBuf::from("/empty"),
            is_dir: false,
            children: vec![],
        };

        let map = get_dir_map(&node, 800);
        assert!(map.is_empty());
    }

    #[test]
    fn test_get_dir_map_with_children() {
        let node = fileDialog::DirectoryNode {
            name: "test".to_string(),
            path: std::path::PathBuf::from("/test"),
            is_dir: true,
            children: vec![
                fileDialog::DirectoryNode {
                    name: "child1".to_string(),
                    path: std::path::PathBuf::from("/test/child1"),
                    is_dir: true,
                    children: vec![],
                },
                fileDialog::DirectoryNode {
                    name: "child2.json".to_string(),
                    path: std::path::PathBuf::from("/test/child2.json"),
                    is_dir: false,
                    children: vec![],
                },
            ],
        };

        let map = get_dir_map(&node, 800);
        assert!(!map.is_empty());
        assert!(map.contains_key("/test"));
        assert!(map.contains_key("/test/child1"));
        assert!(map.contains_key("/test/child2.json"));
    }

    // ------- mouse_over -------

    #[test]
    fn test_mouse_over_inside() {
        let rect = Rect::new(10, 10, 100, 50);
        let point = Point::new(50, 30);
        assert!(mouse_over(rect, point));
    }

    #[test]
    fn test_mouse_over_outside_right() {
        let rect = Rect::new(10, 10, 100, 50);
        let point = Point::new(200, 30);
        assert!(!mouse_over(rect, point));
    }

    #[test]
    fn test_mouse_over_outside_below() {
        let rect = Rect::new(10, 10, 100, 50);
        let point = Point::new(50, 100);
        assert!(!mouse_over(rect, point));
    }

    #[test]
    fn test_mouse_over_top_left_corner() {
        let rect = Rect::new(10, 10, 100, 50);
        let point = Point::new(10, 10);
        assert!(mouse_over(rect, point));
    }

    #[test]
    fn test_mouse_over_origin() {
        let rect = Rect::new(0, 0, 100, 100);
        let point = Point::new(0, 0);
        assert!(mouse_over(rect, point));
    }

    #[test]
    fn test_mouse_over_negative_position() {
        let rect = Rect::new(10, 10, 100, 50);
        let point = Point::new(-5, -5);
        assert!(!mouse_over(rect, point));
    }

    // ------- calculate_scaled_font_size -------

    #[test]
    fn test_font_size_fits_in_width() {
        // 10 chars * 8px = 80px, available = 200 → returns 80
        let size = calculate_scaled_font_size(10, 200);
        assert_eq!(size, 80);
    }

    #[test]
    fn test_font_size_overflows_width() {
        // 50 chars * 8px = 400px, available = 200 → scaled down
        let size = calculate_scaled_font_size(50, 200);
        assert!(size <= 200);
        assert!(size >= 4);
    }

    #[test]
    fn test_font_size_minimum() {
        // Very short width → should be at least 4
        let size = calculate_scaled_font_size(100, 1);
        assert_eq!(size, 4);
    }

    #[test]
    fn test_font_size_exact_fit() {
        // 10 chars * 8px = 80px, available = 80 → returns 80
        let size = calculate_scaled_font_size(10, 80);
        assert_eq!(size, 80);
    }

    #[test]
    fn test_font_size_single_char() {
        let size = calculate_scaled_font_size(1, 100);
        assert_eq!(size, 8);
    }

    #[test]
    fn test_font_size_zero_text_len() {
        let size = calculate_scaled_font_size(0, 100);
        assert_eq!(size, 4); // minimum
    }
}
