//! # Widget Module
//!
//! This module provides the Widget component, a container for organizing
//! multiple Interface components in a grid layout.
//!
//! ## Layout System
//! Widgets use a 2D grid layout specified as a vector of rows, where each row
//! contains component IDs. Components spanning multiple cells are automatically
//! resized to fill their allocated space.
//!
//! ## Example Layout
//! ```text
//! [["button1", "button2"],
//!  ["dropdown", "dropdown"],  // dropdown spans 2 columns
//!  ["slider", "slider"]]
//! ```

extern crate sdl2;

use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::ttf;
use sdl2::video::{Window, WindowContext};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::time::Duration;

use crate::components::file_explorer::FileExplorer;
use crate::components::inputbox::InputBox;
use crate::components::{button::*, Component};

use crate::colors::*;

/// A container widget that arranges Interface components in a grid layout.
///
/// Widgets manage the positioning, sizing, and rendering of child components.
/// They also handle click event delegation and draw state caching.
pub struct Widget {
    /// Screen position of the widget's top-left corner
    pub location: Point,
    /// Unique identifier
    pub id: String,
    /// Result value (used for file dialogs to store selected path)
    pub result: Option<String>,
    /// Total height in pixels
    pub height: u32,
    /// Total width in pixels
    pub width: u32,
    /// Whether the widget is active and interactive
    pub active: bool,
    /// Map of component IDs to Interface implementations
    pub buttons: HashMap<&'static str, Box<dyn Interface>>,
    /// Grid layout specification (rows of component IDs)
    pub layout: Vec<Vec<&'static str>>,
    /// Whether the widget background has been drawn
    pub drawn: bool,
    /// Flag indicating a modal component is blocking input
    pub important_component_clicked: bool,
    /// Cached mapping of grid positions to component IDs
    pub cached_interface_location: Option<HashMap<(i32, i32), &'static str>>,
    /// Cached rendering order based on draw priority
    pub cached_draw_order: Option<Vec<&'static str>>,
}

impl Widget {
    /// Handle a click event on the widget.
    ///
    /// Delegates the click to the appropriate child component based on
    /// mouse position and cached layout information.
    ///
    /// # Arguments
    /// * `after` - True if this is a mouse-up event, false for mouse-down
    /// * `mouse_state` - Position of the mouse click
    ///
    /// # Returns
    /// Tuple of (clicked component ID, (was clicked, inner result))
    pub fn on_click(
        &mut self,
        after: bool,
        mouse_state: Point,
    ) -> (Option<String>, (bool, Option<String>)) {
        let mut dirty = false;
        let mut result: (Option<String>, (bool, Option<String>)) = (None, (false, None));

        if self.important_component_clicked {
            if after {
                println!("Yellow");
                self.change_drawn(false);
                for (_, button) in self
                    .buttons
                    .iter_mut()
                    .filter(|(_, b)| b.important_component_clicked())
                {
                    let result = button.on_click(mouse_state);
                    if result.0 {
                        self.important_component_clicked = false;
                        let button_id = button.get_id();
                        return (Some(button_id), result);
                    } else {
                        return (None, (false, None));
                    }
                }
            } else {
                return (None, (false, None));
            }
        }
        println!("Yep");
        if let Some(cached_map) = &self.cached_interface_location {
            let rows = self.layout.len() as u32;
            let cols = self.layout[0].len() as u32;
            let cell_width = self.width / cols;
            let cell_height = self.height / rows as u32;

            let relative_x = mouse_state.x() - self.location.x();
            let relative_y = mouse_state.y() - self.location.y();

            if relative_x < 0 || relative_y < 0 {
                return result;
            }

            let cell_x = relative_x / cell_width as i32;
            let cell_y = relative_y / cell_height as i32;

            if cell_x >= cols as i32 || cell_y >= rows as i32 {
                return result;
            }

            let pos: (i32, i32) = (cell_x, cell_y);
            if let Some(button_id) = cached_map.get(&pos) {
                if let Some(button) = self.buttons.get_mut(button_id) {
                    if button.dirty_parent() {
                        if button.deactivate_parent() {
                            if after {
                                self.important_component_clicked =
                                    !self.important_component_clicked;
                            }
                        }
                        dirty = true;
                        println!("Yes");
                    }
                    println!("S: {:#?}", button_id);
                    if after {
                        if button.after_click() {
                            result = (Some(button_id.to_string()), button.on_click(mouse_state));
                        } else {
                            result = (None, (false, None));
                        }
                    } else {
                        if !button.after_click() {
                            result = (Some(button_id.to_string()), button.on_click(mouse_state));
                        } else {
                            result = (None, (false, None));
                        }
                    }
                }
            }
        } else {
            for (_, button) in self.buttons.iter_mut() {
                if button.mouse_over_component(mouse_state) {
                    let button_id = button.get_id();
                    println!("C: {}", button_id);
                    if button.dirty_parent() {
                        if button.deactivate_parent() {
                            if after {
                                self.important_component_clicked =
                                    !self.important_component_clicked;
                            }
                        }
                        dirty = true;
                        println!("Yes");
                    } else if button.is_drawn() {
                        button.change_drawn(false);
                    }
                    if after {
                        if button.after_click() {
                            result = (Some(button_id), button.on_click(mouse_state));
                        } else {
                            result = (None, (false, None));
                        }
                    } else {
                        if !button.after_click() {
                            result = (Some(button_id), button.on_click(mouse_state));
                        } else {
                            result = (None, (false, None));
                        }
                    }
                    break;
                }
            }
        }
        if dirty {
            println!("No");
            self.change_drawn(false);
        }
        println!("Result: {:#?}", result);
        return result;
    }

    /// Get the widget's unique identifier.
    fn get_id(&self) -> String {
        self.id.to_string()
    }

    /// Update the draw state of the widget and all children.
    pub fn change_drawn(&mut self, new_val: bool) {
        if self.drawn != new_val {
            self.drawn = new_val;
            for b in self.buttons.values_mut() {
                b.change_drawn(new_val);
            }
        }
    }

    /// Placeholder for widget result computation.
    pub fn widget_result(&mut self) {}

    /// Update the widget's screen position.
    pub fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }

    /// Set a new result value.
    pub fn change_result(&mut self, new_result: Option<String>) {
        self.result = new_result
    }

    /// Activate or deactivate the widget and all children.
    pub fn change_active(&mut self, new_value: bool) {
        if self.active == new_value {
            return;
        }
        self.active = new_value;

        self.buttons
            .iter_mut()
            .for_each(|(_, a)| a.change_active(new_value));
    }

    /// Get the current result value.
    pub fn get_result(&self) -> Option<String> {
        self.result.clone()
    }

    /// Check if the widget is active.
    pub fn is_active(&self) -> bool {
        return self.active;
    }

    /// Get the current screen position.
    pub fn get_location(&self) -> Point {
        return self.location;
    }

    /// Set the widget width.
    pub fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }

    /// Get the current width.
    pub fn get_width(&self) -> u32 {
        self.width
    }

    /// Get the current height.
    fn get_height(&self) -> u32 {
        self.height
    }

    /// Set the widget height.
    pub fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }

    /// Clear the cached draw order (forces recalculation).
    fn invalidate_draw_cache(&mut self) {
        self.cached_draw_order = None;
    }

    /// Check if mouse is over the widget.
    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect();
        return component.contains_point(mouse_position) && self.active;
    }

    /// Update labels of multiple components.
    ///
    /// # Arguments
    /// * `components` - IDs of components to update
    /// * `replacement_labels` - New labels (parallel array)
    pub fn change_labels(&mut self, components: Vec<&str>, replacement_labels: &Vec<&str>) {
        for (i, component) in components.into_iter().enumerate() {
            if let Some(button) = self.buttons.get_mut(component) {
                button.change_label(replacement_labels[i].to_string());
            }
        }
    }

    /// Get the bounding rectangle of the widget.
    pub fn get_rect(&self) -> Rect {
        Rect::new(
            self.location.x(),
            self.location.y(),
            self.width,
            self.height,
        )
    }

    /// Delegate a click to a specific component by ID.
    fn get_options_on_click(
        &mut self,
        id: String,
        mouse_position: Point,
    ) -> (bool, Option<String>) {
        if let Some(component) = self.buttons.get_mut(id.as_str()) {
            return component.on_click(mouse_position);
        }
        return (false, None);
    }

    /// Calculate positions and sizes for all child components.
    ///
    /// Uses the layout grid to determine component placement.
    /// Components spanning multiple cells are sized accordingly.
    fn set_widget_layout(&mut self) {
        let rows = self.layout.len();
        let cols = self.layout[0].len();
        let size = self.buttons.len();
        let mut found_components: HashMap<&str, (usize, usize)> = HashMap::with_capacity(size);
        let mut components_locations: HashMap<(i32, i32), &'static str> =
            HashMap::with_capacity(size);
        let cell_width = self.width / cols as u32;
        let cell_height = self.height / rows as u32;

        for row in 0..rows {
            for col in 0..cols {
                let key = self.layout[row][col];
                let loc: (i32, i32) = (
                    self.location.x() + col as i32 * cell_width as i32,
                    self.location.y() + row as i32 * cell_height as i32,
                );
                components_locations.insert((col as i32, row as i32), key);

                if let Some((start_row, start_col)) = found_components.get(key) {
                    // Component already placed, just extend dimensions
                    if let Some(component) = self.buttons.get_mut(key) {
                        if col > *start_col {
                            if !component.is_static() {
                                component.change_width(
                                    (col as u32 - *start_col as u32 + 1) * cell_width,
                                );
                            }
                        }
                        if row > *start_row {
                            if !component.is_static() {
                                component.change_height(
                                    (row as u32 - *start_row as u32 + 1) * cell_height,
                                );
                            }
                        }
                    }
                } else {
                    // First time seeing this component
                    if let Some(component) = self.buttons.get_mut(key) {
                        let x_offset = if component.has_indent() { 5 } else { 0 };

                        component.change_location(Point::new(loc.0 + x_offset, loc.1));

                        found_components.insert(key, (row, col));

                        if !component.is_static() {
                            component.change_height(cell_height);
                            component.change_width(cell_width);
                        }
                    }
                }
            }
        }
        self.cached_interface_location = Some(components_locations);
    }
    /// Draw the widget and all child components.
    ///
    /// First draws the widget background, then iterates through children
    /// in priority order. Uses caching for efficient redrawing.
    pub fn draw<'a>(
        &mut self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        #[cfg(not(target_os = "macos"))]
        if !self.drawn {
            let rectangle = self.get_rect();
            let outline = Rect::from_center(rectangle.center(), self.width + 5, self.height + 5);
            canvas.set_draw_color(BLACK);
            canvas.fill_rect(outline).unwrap();
            canvas.set_draw_color(SECONDARY_COLOR);
            canvas.fill_rect(rectangle).unwrap();
            self.drawn = true;
            self.set_widget_layout();
        }

        #[cfg(target_os = "macos")]
        {
            let rectangle = self.get_rect();
            let outline = Rect::from_center(rectangle.center(), self.width + 5, self.height + 5);
            canvas.set_draw_color(BLACK);
            canvas.fill_rect(outline).unwrap();
            canvas.set_draw_color(SECONDARY_COLOR);
            canvas.fill_rect(rectangle).unwrap();
            self.drawn = true;
            self.set_widget_layout();
        }

        if let Some(button_ids) = &self.cached_draw_order {
            for id in button_ids {
                if let Some(a) = self.buttons.get_mut(id) {
                    if self.important_component_clicked {
                        if a.important_component_clicked() {
                            a.change_active(true);
                        } else {
                            a.change_active(false);
                        }
                    } else {
                        a.change_active(self.active);
                    }
                    a.draw(canvas, texture_creator, mouse_state, font);
                }
            }
        } else {
            let mut button_ids: Vec<&str> = self.buttons.keys().copied().collect();
            button_ids.sort_by_key(|id| self.buttons[id].draw_priority());
            for id in &button_ids {
                if let Some(a) = self.buttons.get_mut(id) {
                    if self.important_component_clicked {
                        if a.important_component_clicked() {
                            a.change_active(true);
                        } else {
                            a.change_active(false);
                        }
                    } else {
                        a.change_active(self.active);
                    }
                    a.draw(canvas, texture_creator, mouse_state, font);
                    a.change_drawn(true);
                }
            }
            self.cached_draw_order = Some(button_ids);
        }

        // Single pass through sorted list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::*;
    use crate::components::button::*;
    use crate::components::inputbox::InputBox;
    use sdl2::pixels::Color;
    use sdl2::rect::Point;
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn make_button(id: &'static str) -> Box<dyn Interface> {
        Box::new(StandardButton {
            height: 30,
            width: 100,
            location: Point::new(0, 0),
            text_color: WHITE,
            background_color: PRIMARY_COLOR,
            hover: RefCell::new(false),
            text: id.to_string(),
            id: id.to_string(),
            filter: None,
            active: false,
            drawn: RefCell::new(false),
            cached_texture: None,
        })
    }

    fn make_test_widget() -> Widget {
        let buttons: HashMap<&'static str, Box<dyn Interface>> = HashMap::from([
            ("btn_a", make_button("btn_a")),
            ("btn_b", make_button("btn_b")),
        ]);

        let layout: Vec<Vec<&'static str>> = vec![vec!["btn_a", "btn_b"], vec!["btn_a", "btn_b"]];

        Widget {
            location: Point::new(100, 50),
            id: "test_widget".to_string(),
            result: None,
            height: 200,
            width: 400,
            active: true,
            buttons,
            layout,
            drawn: false,
            cached_draw_order: None,
            cached_interface_location: None,
            important_component_clicked: false,
        }
    }

    #[test]
    fn test_widget_get_id() {
        let widget = make_test_widget();
        assert_eq!(widget.get_id(), "test_widget");
    }

    #[test]
    fn test_widget_get_rect() {
        let widget = make_test_widget();
        let rect = widget.get_rect();
        assert_eq!(rect.x(), 100);
        assert_eq!(rect.y(), 50);
        assert_eq!(rect.width(), 400);
        assert_eq!(rect.height(), 200);
    }

    #[test]
    fn test_widget_change_location() {
        let mut widget = make_test_widget();
        widget.change_location(Point::new(200, 300));
        assert_eq!(widget.get_location(), Point::new(200, 300));
    }

    #[test]
    fn test_widget_change_dimensions() {
        let mut widget = make_test_widget();
        widget.change_width(600);
        widget.change_height(400);
        assert_eq!(widget.get_width(), 600);
        assert_eq!(widget.get_height(), 400);
    }

    #[test]
    fn test_widget_active_state() {
        let mut widget = make_test_widget();
        assert!(widget.is_active());
        widget.change_active(false);
        assert!(!widget.is_active());
    }

    #[test]
    fn test_widget_active_cascades_to_children() {
        let mut widget = make_test_widget();
        widget.change_active(false);
        for (_, btn) in widget.buttons.iter() {
            assert!(!btn.is_active());
        }
        widget.change_active(true);
        for (_, btn) in widget.buttons.iter() {
            assert!(btn.is_active());
        }
    }

    #[test]
    fn test_widget_result() {
        let mut widget = make_test_widget();
        assert!(widget.get_result().is_none());
        widget.change_result(Some("test_path".to_string()));
        assert_eq!(widget.get_result(), Some("test_path".to_string()));
    }

    #[test]
    fn test_widget_change_result_to_none() {
        let mut widget = make_test_widget();
        widget.change_result(Some("path".to_string()));
        widget.change_result(None);
        assert!(widget.get_result().is_none());
    }

    #[test]
    fn test_widget_drawn_state() {
        let mut widget = make_test_widget();
        assert!(!widget.drawn);
        widget.change_drawn(true);
        assert!(widget.drawn);
        widget.change_drawn(false);
        assert!(!widget.drawn);
    }

    #[test]
    fn test_widget_drawn_cascades_to_children() {
        let mut widget = make_test_widget();
        widget.change_drawn(true);
        for (_, btn) in widget.buttons.iter() {
            assert!(btn.is_drawn());
        }
        widget.change_drawn(false);
        for (_, btn) in widget.buttons.iter() {
            assert!(!btn.is_drawn());
        }
    }

    #[test]
    fn test_widget_mouse_over_inside() {
        let widget = make_test_widget();
        assert!(widget.mouse_over_component(Point::new(300, 150)));
    }

    #[test]
    fn test_widget_mouse_over_outside() {
        let widget = make_test_widget();
        assert!(!widget.mouse_over_component(Point::new(0, 0)));
        assert!(!widget.mouse_over_component(Point::new(600, 600)));
    }

    #[test]
    fn test_widget_mouse_over_inactive() {
        let mut widget = make_test_widget();
        widget.active = false;
        assert!(!widget.mouse_over_component(Point::new(300, 150)));
    }

    #[test]
    fn test_widget_change_labels() {
        let mut widget = make_test_widget();
        widget.change_labels(vec!["btn_a"], &vec!["New A"]);
        let btn = widget.buttons.get("btn_a").unwrap();
        // StandardButton stores label in .text via change_label
        // We can verify by downcasting
        // For now just verify no panic
    }

    #[test]
    fn test_widget_on_click_outside() {
        let mut widget = make_test_widget();
        let (clicked, _) = widget.on_click(true, Point::new(0, 0));
        assert!(clicked.is_none());
    }

    #[test]
    fn test_widget_invalidate_draw_cache() {
        let mut widget = make_test_widget();
        widget.cached_draw_order = Some(vec!["btn_a"]);
        widget.invalidate_draw_cache();
        assert!(widget.cached_draw_order.is_none());
    }

    #[test]
    fn test_widget_with_single_button() {
        let buttons: HashMap<&'static str, Box<dyn Interface>> =
            HashMap::from([("only", make_button("only"))]);
        let layout = vec![vec!["only"]];
        let widget = Widget {
            location: Point::new(0, 0),
            id: "single".to_string(),
            result: None,
            height: 100,
            width: 100,
            active: true,
            buttons,
            layout,
            drawn: false,
            cached_draw_order: None,
            cached_interface_location: None,
            important_component_clicked: false,
        };
        assert_eq!(widget.buttons.len(), 1);
        assert_eq!(widget.layout.len(), 1);
    }

    #[test]
    fn test_widget_change_active_idempotent() {
        let mut widget = make_test_widget();
        widget.change_active(true);
        widget.change_active(true); // Should not panic or change state
        assert!(widget.is_active());
    }

    // ------- set_widget_layout -------

    #[test]
    fn test_set_widget_layout_populates_cache() {
        let mut widget = make_test_widget();
        widget.change_drawn(true);
        assert!(widget.cached_interface_location.is_none());
        let _ = widget.buttons.get_mut("btn_a");
        widget.set_widget_layout();
        assert!(widget.cached_interface_location.is_some());
    }

    #[test]
    fn test_set_widget_layout_with_single_component() {
        let buttons: HashMap<&'static str, Box<dyn Interface>> =
            HashMap::from([("only", make_button("only"))]);
        let layout = vec![vec!["only"]];
        let mut widget = Widget {
            location: Point::new(0, 0),
            id: "single".to_string(),
            result: None,
            height: 100,
            width: 100,
            active: true,
            buttons,
            layout,
            drawn: false,
            cached_draw_order: None,
            cached_interface_location: None,
            important_component_clicked: false,
        };

        widget.set_widget_layout();
        assert!(widget.cached_interface_location.is_some());
        let cache = widget.cached_interface_location.unwrap();
        assert!(cache.contains_key(&(0, 0)));
        assert_eq!(cache.get(&(0, 0)), Some(&"only"));
    }

    #[test]
    fn test_set_widget_layout_with_multiple_rows() {
        let buttons: HashMap<&'static str, Box<dyn Interface>> = HashMap::from([
            ("top", make_button("top")),
            ("bottom", make_button("bottom")),
        ]);
        let layout = vec![vec!["top"], vec!["bottom"]];
        let mut widget = Widget {
            location: Point::new(0, 0),
            id: "multirow".to_string(),
            result: None,
            height: 200,
            width: 100,
            active: true,
            buttons,
            layout,
            drawn: false,
            cached_draw_order: None,
            cached_interface_location: None,
            important_component_clicked: false,
        };

        widget.set_widget_layout();
        let cache = widget.cached_interface_location.unwrap();
        assert!(cache.contains_key(&(0, 0)));
        assert!(cache.contains_key(&(0, 1)));
    }

    #[test]
    fn test_set_widget_layout_updates_existing_cache() {
        let mut widget = make_test_widget();
        widget.cached_interface_location = Some(HashMap::new());
        widget.set_widget_layout();
        let cache = widget.cached_interface_location.as_ref().unwrap();
        assert!(!cache.is_empty());
    }

    // ------- get_options_on_click -------

    #[test]
    fn test_get_options_on_click_nonexistent() {
        let mut widget = make_test_widget();
        let result = widget.get_options_on_click("nonexistent".to_string(), Point::new(0, 0));
        assert_eq!(result, (false, None));
    }

    #[test]
    fn test_invalidate_draw_cache_resets_draw_order() {
        let mut widget = make_test_widget();
        widget.cached_draw_order = Some(vec!["btn_a", "btn_b"]);
        widget.invalidate_draw_cache();
        assert!(widget.cached_draw_order.is_none());
    }
}
