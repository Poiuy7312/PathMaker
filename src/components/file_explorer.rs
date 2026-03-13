//! # File Explorer Module
//!
//! This module provides a file system browser component for navigating
//! directories and selecting files. Used for save/load dialogs.
//!
//! ## Features
//! - Directory tree navigation
//! - Scrollable list with slider
//! - Optional directory-only filtering
//! - Search/filter support

extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf;
use sdl2::video::{Window, WindowContext};
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::iter;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use crate::components::{board::*, button::*, inputbox::*, Component};
use crate::fileDialog::DirectoryNode;
use crate::{colors::*, fileDialog};

/// A file system browser component with scrollable directory listing.
///
/// Displays entries from a directory tree structure, allowing navigation
/// between directories and selection of files.
pub struct FileExplorer {
    /// Screen position
    pub location: Point,
    /// Unique identifier
    pub id: String,
    /// Height in pixels
    pub height: u32,
    /// Width in pixels
    pub width: u32,
    /// Home/root directory path
    pub default_dir: String,
    /// Shared reference to the directory tree data
    pub directories: Rc<RefCell<HashMap<String, (StandardButton, Vec<String>)>>>,
    /// Currently displayed directory path
    pub current_display: String,
    /// Optional search/filter text
    pub filter: Option<String>,
    /// If true, only show directories (not files)
    pub filter_dir: bool,
    /// Whether the component is interactive
    pub active: bool,
    /// Draw state flag
    pub drawn: RefCell<bool>,
    /// Vertical scroll slider
    pub scroll_slider: RefCell<Slider>,
    /// Cached list of visible button IDs
    pub cached_button_list: RefCell<Option<Vec<String>>>,
}
impl Interface for FileExplorer {
    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn is_static(&self) -> bool {
        false
    }
    fn has_indent(&self) -> bool {
        false
    }

    fn draw_priority(&self) -> u8 {
        0
    }

    fn dirty_parent(&self) -> bool {
        true
    }

    fn important_component_clicked(&self) -> bool {
        false
    }

    fn deactivate_parent(&self) -> bool {
        false
    }

    fn after_click(&self) -> bool {
        true
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn change_drawn(&self, new_val: bool) {
        if self.drawn == new_val.into() {
            return;
        }
        self.drawn.replace(new_val);

        for (b, _) in self.directories.borrow_mut().values_mut() {
            b.change_drawn(new_val);
        }
    }

    fn is_drawn(&self) -> bool {
        let drawn = unsafe { *self.drawn.as_ptr() };
        if drawn {
            return true;
        }
        return false;
    }

    fn change_label(&mut self, _: String) {
        return;
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        let mut display: String;

        match &self.filter {
            Some(filter) => match filter.ends_with(&['/', '\\']) {
                true => {
                    display = filter.trim().to_string();
                    display.pop();
                }
                false => {
                    display = filter
                        .trim()
                        .chars()
                        .rev()
                        .skip_while(|a| a != &'/' || a != &'\\')
                        .collect();
                    display.pop();
                }
            },
            None => {
                display = self.current_display.to_string();
            }
        }

        let mut button_list = Vec::new();

        if self.cached_button_list.borrow().is_none() {
            if let Some(buttons) = self.directories.borrow().get(&display) {
                for id in &buttons.1 {
                    match self.filter_dir {
                        true => {
                            if fileDialog::is_directory(id) {
                                button_list.push(id.to_string());
                            }
                        }
                        false => {
                            button_list.push(id.to_string());
                        }
                    }
                }
            } else if let Some(buttons) = self.directories.borrow().get(&self.current_display) {
                for id in &buttons.1 {
                    match self.filter_dir {
                        true => {
                            if fileDialog::is_directory(id) {
                                button_list.push(id.to_string());
                            }
                        }
                        false => {
                            button_list.push(id.to_string());
                        }
                    }
                }
            }
            self.cached_button_list.replace(Some(button_list.clone()));
        }

        // Update slider range based on number of items
        let cached = self.cached_button_list.borrow();
        let button_list_ref = cached.as_ref().unwrap();

        let max_visible_items = (self.height / 25) as u32;
        let total_items = button_list_ref.len() as u32;

        if total_items > max_visible_items {
            self.scroll_slider.borrow_mut().range = total_items - max_visible_items;
            self.scroll_slider.borrow_mut().active = true;
        } else {
            self.scroll_slider.borrow_mut().active = false;
        }

        // Calculate scroll offset based on slider value
        let slider_value = self.scroll_slider.borrow().value;
        let scroll_offset = slider_value as i32;

        let mut offset: u32 = 0;
        let display_range = self.get_rect(self.location);
        let height = (self.height / 10) as i32;

        for button in button_list_ref {
            if self.directories.borrow().get(button).is_some() {
                let col = self.location.y + offset as i32 * height;
                let loc = Point::new(self.location.x, (col - (scroll_offset * height)).max(0));
                let mut binding = self.directories.borrow_mut();
                let a = binding.get_mut(button).unwrap();
                a.0.change_drawn(false);
                let button_range = a.0.get_rect(loc);
                let used = a.0.layout(loc, self.width - 20, height as u32);
                offset += used;
                if button_range.top() >= display_range.top()
                    && button_range.bottom() <= display_range.bottom()
                {
                    a.0.draw(canvas, &texture_creator, mouse_state, font);
                    a.0.change_active(true);
                }
            }
        }

        // Draw the slider
        if self.scroll_slider.borrow().active {
            let slider_location =
                Point::new(self.location.x + self.width as i32 - 20, self.location.y);
            self.scroll_slider
                .borrow_mut()
                .change_location(slider_location);
            self.scroll_slider.borrow_mut().height = self.height;
            self.scroll_slider.borrow_mut().width = 20;
            self.scroll_slider
                .borrow()
                .draw(canvas, &texture_creator, mouse_state, font);
        }
    }
}
impl Component for FileExplorer {
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<String>) {
        self.change_drawn(false);

        // Check if slider is active and if click is on the slider
        if self.scroll_slider.borrow().active {
            let slider_rect = self
                .scroll_slider
                .borrow()
                .get_rect(self.scroll_slider.borrow().location);
            if slider_rect.contains_point(mouse_state) {
                self.scroll_slider
                    .borrow_mut()
                    .change_slider_value(mouse_state);
                return (true, None);
            }
        }

        // Otherwise check directory buttons
        match self.directories.borrow().get(&self.current_display) {
            Some(value) => {
                for button in &value.1 {
                    match self.directories.borrow().get(button) {
                        Some(but) => {
                            if but.0.mouse_over_component(mouse_state) {
                                return (true, Some(but.0.get_id()));
                            }
                        }
                        None => return (false, None),
                    }
                }
                return (false, None);
            }
            None => {
                return (false, None);
            }
        }
    }

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect(self.location);
        return component.contains_point(mouse_position) && self.active;
    }

    fn get_id(&self) -> String {
        return self.id.to_string();
    }

    fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }

    fn change_active(&mut self, new_value: bool) {
        if self.active != new_value {
            self.active = new_value;
            self.current_display = self.default_dir.clone();
            self.scroll_slider
                .borrow_mut()
                .change_slider_value(Point::new(0, 0));
            self.cached_button_list.replace(None);
        }
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn get_location(&self) -> Point {
        self.location
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }

    fn get_width(&self) -> u32 {
        self.width
    }

    fn get_height(&self) -> u32 {
        self.height
    }

    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl FileExplorer {
    /// Get buttons for the currently displayed directory.
    fn get_cur_buttons(&self) -> Vec<String> {
        if let Some(current_buttons) = self.directories.borrow_mut().get_mut(&self.current_display)
        {
            return current_buttons.1.clone();
        }
        vec![]
    }

    /// Navigate to a different directory.
    ///
    /// Lazily loads the directory's children from disk if they haven't been
    /// scanned yet, then resets scroll position and clears the button cache.
    pub fn change_display(&mut self, new_display: String) {
        if self.current_display != new_display {
            for button in self.get_cur_buttons() {
                if let Some((cur, _)) = self.directories.borrow_mut().get_mut(&button) {
                    cur.change_active(false);
                    cur.change_drawn(false);
                }
            }

            // Lazy-load: scan this directory from disk if not yet loaded
            if fileDialog::is_directory(&new_display) {
                fileDialog::ensure_children_loaded(&self.directories, &new_display);
            }

            self.current_display = new_display;
            self.scroll_slider
                .borrow_mut()
                .change_slider_value(Point::new(0, 0));
            self.cached_button_list.replace(None);
        }
    }

    /// Set a new filter for searching.
    ///
    /// # Returns
    /// True if the filter changed
    pub fn change_filter(&mut self, new_filter: Option<String>) -> bool {
        if self.filter != new_filter {
            self.filter = new_filter;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::*;
    use crate::components::button::*;
    use sdl2::pixels::Color;
    use sdl2::rect::Point;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    fn make_directories() -> Rc<RefCell<HashMap<String, (StandardButton, Vec<String>)>>> {
        let mut map: HashMap<String, (StandardButton, Vec<String>)> = HashMap::new();

        let root_btn = StandardButton {
            height: 25,
            width: 200,
            location: Point::new(0, 0),
            text_color: WHITE,
            background_color: QUATERNARY_COLOR,
            hover: RefCell::new(false),
            text: "root".to_string(),
            id: "/root".to_string(),
            active: false,
            filter: None,
            drawn: RefCell::new(false),
            cached_texture: None,
        };

        let child_btn = StandardButton {
            height: 25,
            width: 200,
            location: Point::new(0, 0),
            text_color: WHITE,
            background_color: QUATERNARY_COLOR,
            hover: RefCell::new(false),
            text: "child".to_string(),
            id: "/root/child".to_string(),
            active: false,
            filter: None,
            drawn: RefCell::new(false),
            cached_texture: None,
        };

        map.insert(
            "/root".to_string(),
            (root_btn, vec!["/root/child".to_string()]),
        );
        map.insert("/root/child".to_string(), (child_btn, vec![]));

        Rc::new(RefCell::new(map))
    }

    fn make_file_explorer() -> FileExplorer {
        let dirs = make_directories();
        FileExplorer {
            location: Point::new(100, 50),
            id: "test_explorer".to_string(),
            height: 300,
            width: 400,
            default_dir: "/root".to_string(),
            directories: dirs,
            current_display: "/root".to_string(),
            filter: None,
            filter_dir: false,
            active: true,
            drawn: RefCell::new(false),
            scroll_slider: RefCell::new(Slider {
                height: 0,
                width: 20,
                location: Point::new(0, 0),
                text_color: BLACK,
                background_color: SECONDARY_COLOR,
                text: String::new(),
                id: "test_slider".to_string(),
                active: false,
                range: 1,
                value: 0,
                slider_offset_axis: 0,
                drawn: RefCell::new(false),
                cached_texture: None,
                is_vertical: true,
                minimal: true,
            }),
            cached_button_list: RefCell::new(None),
        }
    }

    #[test]
    fn test_file_explorer_get_id() {
        let fe = make_file_explorer();
        assert_eq!(fe.get_id(), "test_explorer");
    }

    #[test]
    fn test_file_explorer_get_rect() {
        let fe = make_file_explorer();
        let rect = fe.get_rect(Point::new(100, 50));
        assert_eq!(rect.x(), 100);
        assert_eq!(rect.y(), 50);
        assert_eq!(rect.width(), 400);
        assert_eq!(rect.height(), 300);
    }

    #[test]
    fn test_file_explorer_change_location() {
        let mut fe = make_file_explorer();
        fe.change_location(Point::new(200, 100));
        assert_eq!(fe.get_location(), Point::new(200, 100));
    }

    #[test]
    fn test_file_explorer_change_dimensions() {
        let mut fe = make_file_explorer();
        fe.change_width(500);
        fe.change_height(400);
        assert_eq!(fe.get_width(), 500);
        assert_eq!(fe.get_height(), 400);
    }

    #[test]
    fn test_file_explorer_active_state() {
        let mut fe = make_file_explorer();
        assert!(fe.is_active());
        fe.change_active(false);
        assert!(!fe.is_active());
    }

    #[test]
    fn test_file_explorer_change_active_resets_display() {
        let mut fe = make_file_explorer();
        fe.current_display = "/root/child".to_string();
        fe.change_active(false);
        fe.change_active(true);
        assert_eq!(fe.current_display, "/root");
    }

    #[test]
    fn test_file_explorer_mouse_over_inside() {
        let fe = make_file_explorer();
        assert!(fe.mouse_over_component(Point::new(300, 200)));
    }

    #[test]
    fn test_file_explorer_mouse_over_outside() {
        let fe = make_file_explorer();
        assert!(!fe.mouse_over_component(Point::new(0, 0)));
    }

    #[test]
    fn test_file_explorer_mouse_over_inactive() {
        let mut fe = make_file_explorer();
        fe.active = false;
        assert!(!fe.mouse_over_component(Point::new(300, 200)));
    }

    #[test]
    fn test_file_explorer_drawn_state() {
        let fe = make_file_explorer();
        assert!(!fe.is_drawn());
        fe.change_drawn(true);
        assert!(fe.is_drawn());
        fe.change_drawn(false);
        assert!(!fe.is_drawn());
    }

    #[test]
    fn test_file_explorer_change_filter() {
        let mut fe = make_file_explorer();
        assert!(fe.filter.is_none());
        let changed = fe.change_filter(Some("test".to_string()));
        assert!(changed);
        assert_eq!(fe.filter, Some("test".to_string()));
    }

    #[test]
    fn test_file_explorer_change_filter_same_value() {
        let mut fe = make_file_explorer();
        fe.change_filter(Some("test".to_string()));
        let changed = fe.change_filter(Some("test".to_string()));
        assert!(!changed);
    }

    #[test]
    fn test_file_explorer_change_filter_to_none() {
        let mut fe = make_file_explorer();
        fe.change_filter(Some("test".to_string()));
        let changed = fe.change_filter(None);
        assert!(changed);
        assert!(fe.filter.is_none());
    }

    #[test]
    fn test_file_explorer_get_cur_buttons() {
        let fe = make_file_explorer();
        let buttons = fe.get_cur_buttons();
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0], "/root/child");
    }

    #[test]
    fn test_file_explorer_get_cur_buttons_empty() {
        let mut fe = make_file_explorer();
        fe.current_display = "/nonexistent".to_string();
        let buttons = fe.get_cur_buttons();
        assert!(buttons.is_empty());
    }

    #[test]
    fn test_file_explorer_on_click_outside() {
        let mut fe = make_file_explorer();
        let (clicked, _) = fe.on_click(Point::new(0, 0));
        assert!(!clicked);
    }

    #[test]
    fn test_file_explorer_interface_traits() {
        let fe = make_file_explorer();
        assert!(!fe.is_static());
        assert!(!fe.has_indent());
        assert_eq!(fe.draw_priority(), 0);
        assert!(fe.dirty_parent());
        assert!(!fe.important_component_clicked());
        assert!(!fe.deactivate_parent());
        assert!(fe.after_click());
    }

    #[test]
    fn test_file_explorer_cached_button_list_initially_none() {
        let fe = make_file_explorer();
        assert!(fe.cached_button_list.borrow().is_none());
    }

    #[test]
    fn test_file_explorer_change_active_clears_cache() {
        let fe = make_file_explorer();
        fe.cached_button_list
            .replace(Some(vec!["test".to_string()]));
        // Toggling active should clear the cache
        let mut fe = fe;
        fe.change_active(false);
        assert!(fe.cached_button_list.borrow().is_none());
    }
}
