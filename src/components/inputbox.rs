//! # Input Box Module
//!
//! This module provides a text input field component for user text entry.
//! Supports placeholder text and visual feedback when focused.

use std::any::Any;
use std::cell::RefCell;

use sdl2::mouse::MouseState;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::components::button::Interface;
use crate::components::Component;
use crate::util;
use sdl2::ttf;

/// A text input field with placeholder support.
///
/// Displays either the entered text or placeholder text (dimmed).
/// Shows a cursor when clicked/focused.
pub struct InputBox {
    /// Placeholder text shown when empty
    pub default_text: String,
    /// Currently entered text
    pub text: String,
    /// Whether the component is interactive
    pub active: bool,
    /// Text color
    pub text_color: Color,
    /// Background color
    pub background_color: Color,
    /// Whether the input is currently focused
    pub clicked_on: bool,
    /// Height in pixels
    pub height: u32,
    /// Width in pixels
    pub width: u32,
    /// Unique identifier
    pub id: String,
    /// Screen position
    pub location: Point,
    /// Draw state flag
    pub drawn: RefCell<bool>,
}

impl Component for InputBox {
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<String>) {
        if self.mouse_over_component(mouse_state) {
            if self.text.len() == 0 {
                self.text = " ".to_string();
            }
            self.clicked_on = true;
            return (true, Some(self.get_id()));
        }
        return (false, None);
    }

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect(self.location);
        return component.contains_point(mouse_position);
    }

    fn get_id(&self) -> String {
        return self.id.to_string();
    }

    fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }

    fn change_active(&mut self, new_value: bool) {
        self.active = new_value;
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn get_width(&self) -> u32 {
        self.width
    }

    fn get_height(&self) -> u32 {
        self.height
    }
    fn get_location(&self) -> Point {
        self.location
    }

    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl Interface for InputBox {
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

    fn change_drawn(&self, new_val: bool) {
        if self.drawn == new_val.into() {
            return;
        }
        self.drawn.replace(new_val);
    }

    fn is_drawn(&self) -> bool {
        let drawn = unsafe { *self.drawn.as_ptr() };
        if drawn {
            return true;
        }
        return false;
    }

    fn change_label(&mut self, new_text: String) {
        self.default_text = new_text
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        _: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        let rectangle = self.get_rect(self.location);
        let box_outline = Rect::from_center(rectangle.center(), self.width + 5, self.height + 5);
        let available_width = (self.width as i32 - 10) as u32;
        let text_len = if self.text.len() > 0 {
            self.text.chars().count() as u32
        } else {
            self.default_text.chars().count() as u32
        };
        let box_background: Rect = rectangle;
        let font_size = util::calculate_scaled_font_size(text_len, available_width);
        let text_map_x = box_background.left() + 5;
        let text_map: Rect = Rect::new(
            text_map_x,
            rectangle.center().y(),
            font_size,
            self.height / 2,
        );

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.fill_rect(box_outline).unwrap();
        canvas.set_draw_color(self.background_color);
        canvas.fill_rect(box_background).unwrap();
        match self.text.len() > 0 {
            true => {
                let font_size = 8 * self.text.chars().count() as u32;
                let mut text_map: Rect = Rect::new(
                    text_map_x,
                    rectangle.center().y() - 5,
                    font_size,
                    self.height / 2,
                );
                if text_map.width() >= box_background.width() {
                    text_map.set_width(box_background.width() * 5 / 6);
                }
                if self.clicked_on() {
                    let text = self.text.clone() + "|";
                    let font_surface = font
                        .render(&text)
                        .blended(self.text_color)
                        .map_err(|e| e.to_string())
                        .unwrap();
                    let font_texture: Texture<'_> = texture_creator
                        .create_texture_from_surface(&font_surface)
                        .map_err(|e| e.to_string())
                        .unwrap();
                    canvas.copy(&font_texture, None, text_map).unwrap();
                } else {
                    let font_surface = font
                        .render(&self.text)
                        .blended(self.text_color)
                        .map_err(|e| e.to_string())
                        .unwrap();
                    let font_texture: Texture<'_> = texture_creator
                        .create_texture_from_surface(&font_surface)
                        .map_err(|e| e.to_string())
                        .unwrap();
                    canvas.copy(&font_texture, None, text_map).unwrap();
                }
            }
            false => {
                let font_surface = font
                    .render(&self.default_text)
                    .blended(Color::RGB(158, 158, 158))
                    .map_err(|e| e.to_string())
                    .unwrap();
                let font_texture: Texture<'_> = texture_creator
                    .create_texture_from_surface(&font_surface)
                    .map_err(|e| e.to_string())
                    .unwrap();
                canvas.copy(&font_texture, None, text_map).unwrap();
            }
        }
    }
}

impl InputBox {
    /// Update the current text content.
    pub fn change_text(&mut self, new_text: String) {
        self.text = new_text;
    }

    /// Check if the input box is currently focused.
    pub fn clicked_on(&self) -> bool {
        self.clicked_on
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::pixels::Color;
    use sdl2::rect::Point;
    use std::cell::RefCell;

    fn make_inputbox(x: i32, y: i32, w: u32, h: u32) -> InputBox {
        InputBox {
            default_text: "Placeholder".to_string(),
            text: "".to_string(),
            active: true,
            text_color: Color::RGB(255, 255, 255),
            background_color: Color::RGB(84, 84, 84),
            clicked_on: false,
            height: h,
            width: w,
            id: "test_input".to_string(),
            location: Point::new(x, y),
            drawn: RefCell::new(false),
        }
    }

    #[test]
    fn test_inputbox_get_id() {
        let ib = make_inputbox(0, 0, 200, 30);
        assert_eq!(ib.get_id(), "test_input");
    }

    #[test]
    fn test_inputbox_click_inside_sets_focus() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        assert!(!ib.clicked_on());
        ib.on_click(Point::new(100, 15));
        assert!(ib.clicked_on());
    }

    #[test]
    fn test_inputbox_click_inside_sets_space_when_empty() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        assert_eq!(ib.text, "");
        ib.on_click(Point::new(100, 15));
        assert_eq!(ib.text, " ");
    }

    #[test]
    fn test_inputbox_click_outside_no_focus() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        let (clicked, _) = ib.on_click(Point::new(300, 300));
        assert!(!clicked);
        assert!(!ib.clicked_on());
    }

    #[test]
    fn test_inputbox_mouse_over_inside() {
        let ib = make_inputbox(10, 10, 200, 30);
        assert!(ib.mouse_over_component(Point::new(50, 20)));
    }

    #[test]
    fn test_inputbox_mouse_over_outside() {
        let ib = make_inputbox(10, 10, 200, 30);
        assert!(!ib.mouse_over_component(Point::new(300, 300)));
    }

    #[test]
    fn test_inputbox_change_text() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        ib.change_text("Hello".to_string());
        assert_eq!(ib.text, "Hello");
    }

    #[test]
    fn test_inputbox_change_label() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        ib.change_label("New Placeholder".to_string());
        assert_eq!(ib.default_text, "New Placeholder");
    }

    #[test]
    fn test_inputbox_change_location() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        ib.change_location(Point::new(50, 100));
        assert_eq!(ib.get_location(), Point::new(50, 100));
    }

    #[test]
    fn test_inputbox_change_dimensions() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        ib.change_width(300);
        ib.change_height(50);
        assert_eq!(ib.get_width(), 300);
        assert_eq!(ib.get_height(), 50);
    }

    #[test]
    fn test_inputbox_active_state() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        assert!(ib.is_active());
        ib.change_active(false);
        assert!(!ib.is_active());
    }

    #[test]
    fn test_inputbox_drawn_state() {
        let ib = make_inputbox(0, 0, 200, 30);
        assert!(!ib.is_drawn());
        ib.change_drawn(true);
        assert!(ib.is_drawn());
        ib.change_drawn(false);
        assert!(!ib.is_drawn());
    }

    #[test]
    fn test_inputbox_get_rect() {
        let ib = make_inputbox(10, 20, 200, 30);
        let rect = ib.get_rect(Point::new(10, 20));
        assert_eq!(rect.x(), 10);
        assert_eq!(rect.y(), 20);
        assert_eq!(rect.width(), 200);
        assert_eq!(rect.height(), 30);
    }

    #[test]
    fn test_inputbox_interface_traits() {
        let ib = make_inputbox(0, 0, 200, 30);
        assert!(!ib.is_static());
        assert!(!ib.has_indent());
        assert_eq!(ib.draw_priority(), 1);
        assert!(!ib.dirty_parent());
        assert!(!ib.important_component_clicked());
        assert!(!ib.deactivate_parent());
        assert!(ib.after_click());
    }

    #[test]
    fn test_inputbox_click_preserves_existing_text() {
        let mut ib = make_inputbox(0, 0, 200, 30);
        ib.text = "existing".to_string();
        ib.on_click(Point::new(100, 15));
        assert_eq!(ib.text, "existing");
        assert!(ib.clicked_on());
    }
}
