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
    pub directories: RefCell<HashMap<String, Vec<Box<dyn ValidDropdownOption>>>>,
    pub current_display: String,
    pub active: bool,
}
impl Interface for FileExplorer {
    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Option<Point>,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        if let Some(buttons) = self.directories.borrow_mut().get_mut(&self.current_display) {
            layout::layout_root(buttons, self.location, self.width, 25, None);
            buttons.iter_mut().for_each(|a| {
                a.change_active(true);
                a.draw(canvas, &texture_creator, Some(mouse_state.unwrap()), font);
            });
        }
    }
}
impl Component for FileExplorer {
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<String>) {
        match self.directories.borrow().get(&self.current_display) {
            Some(value) => {
                for button in value {
                    if button.mouse_over_component(mouse_state) {
                        return (true, Some(button.get_id()));
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
}
