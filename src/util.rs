use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use sdl2::rect::{Point, Rect};

use crate::{components::button::*, fileDialog};

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
