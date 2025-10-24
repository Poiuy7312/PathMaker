use std::any::Any;

use sdl2::mouse::MouseState;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::components::button::Interface;
use crate::components::Component;
use sdl2::ttf;

pub struct InputBox {
    pub default_text: String,
    pub text: String,
    pub active: bool,
    pub text_color: Color,
    pub background_color: Color,
    pub height: u32,
    pub width: u32,
    pub id: String,
    pub location: Point,
    pub drawn: bool,
}

impl Component for InputBox {
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<String>) {
        if self.mouse_over_component(mouse_state) {
            if self.text.len() == 0 {
                self.text = " ".to_string();
            }
            self.active = true;
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

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
    fn change_drawn(&mut self, new_val: bool) {
        self.drawn = new_val;
    }

    fn is_drawn(&self) -> bool {
        self.drawn
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
        let box_background: Rect = rectangle;
        let text_map_x = box_background.left() + 5;
        let text_map: Rect = Rect::new(
            text_map_x,
            rectangle.center().y(),
            self.width / 2,
            self.height / 2,
        );

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.fill_rect(box_outline).unwrap();
        canvas.set_draw_color(self.background_color);
        canvas.fill_rect(box_background).unwrap();
        match self.text.len() > 0 {
            true => {
                let font_size = 8 * self.text.chars().count() as u32;
                let text_map: Rect = Rect::new(
                    text_map_x,
                    rectangle.center().y() - 5,
                    font_size,
                    self.height / 2,
                );
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
    pub fn change_text(&mut self, new_text: String) {
        self.text = new_text;
    }
}
