use std::any::Any;
use std::cell::RefCell;

use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf;
use sdl2::video::{Window, WindowContext};

use crate::colors::{BLACK, PRIMARY_COLOR, SECONDARY_COLOR, WHITE};
use crate::components::button::Interface;
use crate::components::Component;

pub struct DisplayBox {
    pub current_display: Vec<String>,
    pub location: Point,
    pub width: u32,
    pub height: u32,
    pub id: String,
    pub active: bool,
    pub background_color: Color,
    pub text_color: Color,
    pub drawn: RefCell<bool>,
    pub scroll_offset: RefCell<i32>,
    pub max_lines_visible: usize,
    pub line_height: i32,
    pub cached_texture: RefCell<Option<Texture<'static>>>,
}

impl DisplayBox {
    pub fn new(x: i32, y: i32, width: u32, height: u32, id: &str) -> Self {
        let line_height = 16;
        let max_lines_visible = (height as i32 / line_height) as usize;
        DisplayBox {
            current_display: Vec::new(),
            location: Point::new(x, y),
            width,
            height,
            id: id.to_string(),
            active: true,
            background_color: Color::RGB(30, 30, 30),
            text_color: WHITE,
            drawn: RefCell::new(false),
            scroll_offset: RefCell::new(0),
            max_lines_visible,
            line_height,
            cached_texture: RefCell::new(None),
        }
    }

    pub fn clear(&mut self) {
        self.current_display.clear();
        *self.scroll_offset.borrow_mut() = 0;
    }

    pub fn add_line(&mut self, new_str: &str) {
        self.current_display.push(new_str.to_string());
        if self.current_display.len() > 1000 {
            self.current_display.remove(0);
        }
    }

    pub fn scroll_up(&mut self) {
        let mut offset = self.scroll_offset.borrow_mut();
        if *offset > 0 {
            *offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        let mut offset = self.scroll_offset.borrow_mut();
        let total_lines = self.current_display.len() as i32;
        let visible = self.max_lines_visible as i32;
        if *offset < total_lines - visible {
            *offset += 1;
        }
    }

    pub fn get_rect(&self) -> Rect {
        Rect::new(
            self.location.x(),
            self.location.y(),
            self.width,
            self.height,
        )
    }
}

impl Component for DisplayBox {
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<String>) {
        if self.mouse_over_component(mouse_state) {
            return (true, Some(self.get_id()));
        }
        (false, None)
    }

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        self.get_rect().contains_point(mouse_position)
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
        self.max_lines_visible = (self.height as i32 / self.line_height) as usize;
    }

    fn change_active(&mut self, new_value: bool) {
        self.active = new_value;
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn get_location(&self) -> Point {
        self.location
    }

    fn get_width(&self) -> u32 {
        self.width
    }

    fn get_height(&self) -> u32 {
        self.height
    }

    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
        self.max_lines_visible = (new_height as i32 / self.line_height) as usize;
    }
}

impl Interface for DisplayBox {
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
        1
    }

    fn dirty_parent(&self) -> bool {
        false
    }

    fn important_component_clicked(&self) -> bool {
        false
    }

    fn after_click(&self) -> bool {
        true
    }

    fn deactivate_parent(&self) -> bool {
        false
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn change_label(&mut self, _new_text: String) {}

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        _: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        let rect = self.get_rect();

        canvas.set_draw_color(self.background_color);
        canvas.fill_rect(rect).unwrap();

        canvas.set_draw_color(PRIMARY_COLOR);
        canvas.draw_rect(rect).unwrap();

        let scroll_offset = *self.scroll_offset.borrow();
        let start_y = rect.y() + 5;
        let max_width = rect.width() - 20;

        let end_idx =
            (scroll_offset as usize + self.max_lines_visible).min(self.current_display.len());
        let start_idx = scroll_offset as usize;

        for (i, line) in self
            .current_display
            .iter()
            .enumerate()
            .take(end_idx)
            .skip(start_idx)
        {
            let y = start_y + ((i - start_idx) as i32) * self.line_height;

            let text_width = (line.len() as u32) * 8;
            let text_rect = if text_width > max_width {
                Rect::new(rect.x() + 5, y, max_width, self.line_height as u32)
            } else {
                Rect::new(rect.x() + 5, y, text_width, self.line_height as u32)
            };

            match font.render(line).blended(self.text_color) {
                Ok(font_surface) => {
                    let font_texture = texture_creator.create_texture_from_surface(&font_surface);
                    if font_texture.is_ok() {
                        let _ = canvas.copy(&font_texture.unwrap(), None, text_rect);
                    }
                }
                Err(_) => {}
            }
        }

        if self.current_display.len() > self.max_lines_visible {
            let scrollbar_height =
                rect.height() * self.max_lines_visible as u32 / self.current_display.len() as u32;
            let scrollbar_y = rect.y() as i32
                + ((scroll_offset as u32) * (rect.height() - scrollbar_height)
                    / (self.current_display.len() - self.max_lines_visible) as u32)
                    as i32;
            let scrollbar = Rect::new(
                rect.x() + rect.width() as i32 - 10,
                scrollbar_y,
                8,
                scrollbar_height,
            );
            canvas.set_draw_color(SECONDARY_COLOR);
            canvas.fill_rect(scrollbar).unwrap();
        }
    }
}
