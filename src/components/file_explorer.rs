extern crate sdl2;
use sdl2::event::Event;
use sdl2::gfx;
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

pub struct FileExplorer {
    pub location: Point,
    pub id: String,
    pub height: u32,
    pub width: u32,
    pub default_dir: String,
    pub directories: Rc<RefCell<HashMap<String, (StandardButton, Vec<String>)>>>,
    pub current_display: String,
    pub filter: Option<String>,
    pub filter_dir: bool,
    pub active: bool,
    pub drawn: RefCell<bool>,
    pub scroll_slider: RefCell<Slider>,
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

    fn deactivate_parent(&self) -> bool {
        false
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

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        let mut button_list: Vec<String> = Vec::new();

        let mut display: String;

        if !self.is_drawn() {
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

            // Update slider range based on number of items
            let max_visible_items = (self.height / 25) as u32;
            let total_items = button_list.len() as u32;

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

            for button in button_list {
                if self.directories.borrow().get(&button).is_some() {
                    let col = self.location.y + offset as i32 * height;
                    let loc = Point::new(self.location.x, (col - (scroll_offset * height)).max(0));
                    let mut binding = self.directories.borrow_mut();
                    let a = binding.get_mut(&button).unwrap();
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
        } else {
            self.directories
                .borrow_mut()
                .values_mut()
                .for_each(|(button, _)| button.draw(canvas, texture_creator, mouse_state, font));
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
        self.active = new_value;
        if !new_value {
            self.current_display = self.default_dir.clone();
            self.scroll_slider
                .borrow_mut()
                .change_slider_value(Point::new(0, 0));
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
    pub fn change_display(&mut self, new_display: String) {
        self.current_display = new_display;
        self.scroll_slider
            .borrow_mut()
            .change_slider_value(Point::new(0, 0));
    }

    pub fn change_filter(&mut self, new_filter: Option<String>) -> bool {
        if self.filter != new_filter {
            self.filter = new_filter;
            return true;
        }
        false
    }
}
