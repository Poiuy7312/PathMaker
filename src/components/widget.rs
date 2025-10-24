extern crate sdl2;

use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::ttf;
use sdl2::video::{Window, WindowContext};
use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::components::file_explorer::FileExplorer;
use crate::components::inputbox::InputBox;
use crate::components::{button::*, Component};

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
}

impl Widget {
    pub fn on_click(&mut self, mouse_state: Point) -> (Option<String>, (bool, Option<String>)) {
        match self
            .buttons
            .iter_mut()
            .find(|(_, a)| a.mouse_over_component(mouse_state))
        {
            Some((_, button)) => {
                let button_id = button.get_id();
                return (Some(button_id), button.on_click(mouse_state));
            }
            None => {
                return (None, (false, None));
            }
        };
    }

    fn get_id(&self) -> String {
        self.id.to_string()
    }

    pub fn change_drawn(&mut self, new_val: bool) {
        self.drawn = new_val;
        for b in self.buttons.values_mut() {
            b.change_drawn(new_val);
        }
    }

    pub fn get_mut_button(&mut self, id: &str) -> Option<&mut dyn Any> {
        if let Some(button) = self.buttons.get_mut(id) {
            return Some(button.as_any());
        }
        return None;
    }

    pub fn widget_result(&mut self) {}

    pub fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }

    pub fn change_result(&mut self, new_result: Option<String>) {
        self.result = new_result
    }

    pub fn change_active(&mut self, new_value: bool) {
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
        let cell_width = self.width / cols as u32;
        let cell_height = self.height / rows as u32;
        for row in 0..rows {
            for col in 0..cols {
                if !found_components.contains_key(self.layout[row][col]) {
                    if let Some(component) = self.buttons.get_mut(self.layout[row][col]) {
                        component.change_location(Point::new(
                            self.location.x() + col as i32 * cell_width as i32,
                            self.location.y() + row as i32 * cell_height as i32,
                        ));
                        found_components.insert(self.layout[row][col], (row, col));
                        component.change_height(cell_height);
                        component.change_width(cell_width);
                    }
                } else {
                    if let Some(component) = self.buttons.get_mut(self.layout[row][col]) {
                        if let Some((height, width)) =
                            found_components.get_mut(self.layout[row][col])
                        {
                            if &col > width {
                                component
                                    .change_width((col as u32 - *width as u32 + 1) * cell_width);
                            }
                            if &row > height {
                                component
                                    .change_height((row as u32 - *height as u32 + 1) * cell_height);
                            }
                        }
                    }
                }
            }
        }
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
        }

        self.set_widget_layout();

        self.buttons.iter_mut().for_each(|(_, a)| {
            a.change_active(self.active);
            a.draw(canvas, texture_creator, mouse_state, font);
        });
    }
}
