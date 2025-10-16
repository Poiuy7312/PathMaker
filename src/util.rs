use std::collections::HashMap;

use sdl2::rect::{Point, Rect};

use crate::{components::button::*, fileDialog};

pub fn mouse_over(rect: Rect, mouse_position: Point) -> bool {
    return rect.contains_point(mouse_position);
}

use crate::colors::*;

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
                }))
            }
        }
    }
    return buttons;
}

pub fn get_dir_map(
    node: &fileDialog::DirectoryNode,
    width: u32,
) -> HashMap<String, Vec<Box<dyn ValidDropdownOption>>> {
    let mut buttons: Vec<Box<dyn ValidDropdownOption>> = Vec::new();
    let mut map: HashMap<String, Vec<Box<dyn ValidDropdownOption>>> = HashMap::new();
    if node.is_dir {
        for child in &node.children {
            if child.is_dir {
                buttons.push(Box::new(StandardButton {
                    height: 25,
                    width: 200,
                    location: Point::new(width as i32 - 200, 62),
                    text_color: WHITE,
                    background_color: QUATERNARY_COLOR,
                    hover_color: SECONDARY_COLOR,
                    text: child.name.to_string(),
                    id: child.path.to_string_lossy().to_string(),
                    active: false,
                    filter: None,
                }));
                map.extend(get_dir_map(child, width));
            }
        }
        map.insert(node.path.to_string_lossy().to_string(), buttons);
    }
    return map;
}
