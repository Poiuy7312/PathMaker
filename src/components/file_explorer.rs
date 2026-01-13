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
use std::time::Duration;

use crate::components::{board::*, button::*, inputbox::*, Component};
use crate::fileDialog::DirectoryNode;
use crate::{colors::*, layout};

pub struct FileExplorer {
    pub location: Point,
    pub id: String,
    pub height: u32,
    pub width: u32,
    pub default_dir: String,
    pub directories: RefCell<HashMap<String, (StandardButton, Vec<String>)>>,
    pub current_display: String,
    pub filter: Option<String>,
    pub active: bool,
    pub drawn: RefCell<bool>,
}
impl Interface for FileExplorer {
    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn is_static(&self) -> bool {
        false
    }

    fn draw_priority(&self) -> bool {
        true
    }

    fn dirty_parent(&self) -> bool {
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
            println!("{}", display);

            if let Some(buttons) = self.directories.borrow().get(&display) {
                for id in &buttons.1 {
                    button_list.push(id.to_string());
                }
            } else if let Some(buttons) = self.directories.borrow().get(&self.current_display) {
                for id in &buttons.1 {
                    button_list.push(id.to_string());
                }
            }

            let mut offset: u32 = 0;
            button_list
                .iter()
                .filter(|&a| self.directories.borrow().get(a).is_some())
                .for_each(|a| {
                    let mut binding = self.directories.borrow_mut();
                    let a = binding.get_mut(a).unwrap();
                    let col = self.location.y + (offset as i32 * 25 as i32);
                    let loc = Point::new(self.location.x, col);
                    let used = a.0.layout(loc, self.width, 25);
                    offset += used;
                    a.0.change_active(true);
                    a.0.draw(canvas, &texture_creator, mouse_state, font);
                });
        } else {
            self.directories
                .borrow_mut()
                .values_mut()
                .filter(|(b, _)| b.mouse_over_component(mouse_state) || !b.is_drawn())
                .for_each(|(button, _)| {
                    if button.mouse_over_component(mouse_state) {
                        match button.is_drawn() {
                            true => {
                                button.change_drawn(false);
                            }
                            false => {}
                        }
                        button.draw(canvas, &texture_creator, mouse_state, font);
                    } else {
                        match button.is_drawn() {
                            true => {}
                            false => {
                                button.draw(canvas, &texture_creator, mouse_state, font);
                                button.change_drawn(true);
                            }
                        }
                    }
                });
        }
    }
}
impl Component for FileExplorer {
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<String>) {
        self.change_drawn(false);
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
        self.current_display = new_display
    }

    pub fn change_filter(&mut self, new_filter: Option<String>) -> bool {
        if self.filter != new_filter {
            self.filter = new_filter;
            return true;
        }
        false
    }
}
