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

/// Add static variant so widget doesn't directly change size of components
use crate::colors::*;

/// Struct for combining multiple Interface components and formatting them
pub struct Widget {
    pub location: Point,
    pub id: String,
    pub result: Option<String>,
    pub height: u32,
    pub width: u32,
    pub active: bool,
    pub buttons: HashMap<&'static str, Box<dyn Interface>>,
    pub layout: Vec<Vec<&'static str>>,
    pub drawn: bool,
    pub cached_interface_location: Option<HashMap<(i32, i32), &'static str>>,
    pub cached_draw_order: Option<Vec<&'static str>>,
}

impl Widget {
    pub fn on_click(&mut self, mouse_state: Point) -> (Option<String>, (bool, Option<String>)) {
        let mut dirty = false;
        let mut button_found: (Option<String>, (bool, Option<String>)) = (None, (false, None));

        if let Some(cached_map) = &self.cached_interface_location {
            let rows = self.layout.len() as u32;
            let cols = self.layout[0].len() as u32;
            let cell_width = self.width / cols;
            let cell_height = self.height / rows as u32;

            let relative_x = mouse_state.x() - self.location.x();
            let relative_y = mouse_state.y() - self.location.y();

            if relative_x < 0 || relative_y < 0 {
                return button_found;
            }

            let cell_x = relative_x / cell_width as i32;
            let cell_y = relative_y / cell_height as i32;

            if cell_x >= cols as i32 || cell_y >= rows as i32 {
                return button_found;
            }

            let pos: (i32, i32) = (cell_x, cell_y);
            println!("{:#?}", pos);
            if let Some(button_id) = cached_map.get(&pos) {
                if let Some(button) = self.buttons.get_mut(button_id) {
                    return (Some(button_id.to_string()), button.on_click(mouse_state));
                }
            }
        }
        for (_, button) in self.buttons.iter_mut() {
            if button.mouse_over_component(mouse_state) {
                let button_id = button.get_id();
                // Just set drawn to false directly if it was true
                if button.dirty_parent() {
                    dirty = true;
                } else if button.is_drawn() {
                    button.change_drawn(false);
                }
                button_found = (Some(button_id), button.on_click(mouse_state));
                break;
            }
        }
        if dirty {
            self.change_drawn(false);
        }
        return button_found;
    }

    fn get_id(&self) -> String {
        self.id.to_string()
    }

    pub fn change_drawn(&mut self, new_val: bool) {
        if self.drawn != new_val {
            self.drawn = new_val;
            for b in self.buttons.values_mut() {
                b.change_drawn(new_val);
            }
        }
    }

    pub fn widget_result(&mut self) {}

    pub fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }

    pub fn change_result(&mut self, new_result: Option<String>) {
        self.result = new_result
    }

    pub fn change_active(&mut self, new_value: bool) {
        if self.active == new_value {
            return; // Skip if no change
        }
        self.active = new_value;

        self.buttons
            .iter_mut()
            .for_each(|(_, a)| a.change_active(new_value));
    }

    pub fn get_result(&self) -> Option<String> {
        self.result.clone()
    }

    pub fn is_active(&self) -> bool {
        return self.active;
    }

    pub fn get_location(&self) -> Point {
        return self.location;
    }

    pub fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    fn get_height(&self) -> u32 {
        self.height
    }

    pub fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }

    fn invalidate_draw_cache(&mut self) {
        self.cached_draw_order = None;
    }

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect();
        return component.contains_point(mouse_position) && self.active;
    }

    pub fn get_rect(&self) -> Rect {
        Rect::new(
            self.location.x(),
            self.location.y(),
            self.width,
            self.height,
        )
    }

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
    fn set_widget_layout(&mut self) {
        let rows = self.layout.len();
        let cols = self.layout[0].len();
        let mut found_components: HashMap<&str, (usize, usize)> = HashMap::new();
        let mut components_locations: HashMap<(i32, i32), &'static str> = HashMap::new();
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
                            component
                                .change_width((col as u32 - *start_col as u32 + 1) * cell_width);
                        }
                        if row > *start_row {
                            component
                                .change_height((row as u32 - *start_row as u32 + 1) * cell_height);
                        }
                    }
                } else {
                    // First time seeing this component
                    if let Some(component) = self.buttons.get_mut(key) {
                        let x_offset = if component.is_static() { 5 } else { 0 };
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
    pub fn draw<'a>(
        &mut self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
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

        if let Some(button_ids) = &self.cached_draw_order {
            for id in button_ids {
                if let Some(a) = self.buttons.get_mut(id) {
                    a.change_active(self.active);
                    a.draw(canvas, texture_creator, mouse_state, font);
                }
            }
        } else {
            let mut button_ids: Vec<&str> = self.buttons.keys().copied().collect();
            button_ids.sort_by_key(|id| {
                // high priority = false (drawn last), low priority = true (drawn first)
                // so reverse: !draw_priority() sorts to the end
                !self.buttons[id].draw_priority()
            });
            for id in &button_ids {
                if let Some(a) = self.buttons.get_mut(id) {
                    a.change_active(self.active);
                    a.draw(canvas, texture_creator, mouse_state, font);
                    a.change_drawn(true);
                }
            }
            self.cached_draw_order = Some(button_ids);
        }

        // Single pass through sorted list
    }
}
