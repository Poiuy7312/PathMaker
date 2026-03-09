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

/// Convert a directory tree structure into a flat HashMap for efficient lookup.
///
/// Creates a map where:
/// - Keys are full file/directory paths
/// - Values are tuples of (StandardButton, Vec<child_paths>)
///
/// This structure allows O(1) lookup of any node and its children,
/// which is essential for the file explorer's navigation.
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
            if child.is_dir {
                buttons.push(child.path.to_string_lossy().to_string());

                let child_map = get_dir_map(child, width);
                map.extend(child_map);
            } else {
                map.insert(
                    child.path.to_string_lossy().to_string(),
                    (
                        StandardButton {
                            height: 25,
                            width: 200,
                            location: Point::new(0, 62),
                            text_color: WHITE,
                            background_color: QUATERNARY_COLOR,
                            hover: RefCell::new(false),
                            text: child.name.to_string(),
                            id: child.path.to_string_lossy().to_string(),
                            active: false,
                            filter: None,
                            drawn: RefCell::new(false),
                            cached_texture: None,
                        },
                        Vec::new(),
                    ),
                );

                buttons.push(child.path.to_string_lossy().to_string());
            }
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
