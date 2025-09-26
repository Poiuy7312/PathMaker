use super::button::Button;

use sdl2::mouse::MouseState;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use sdl2::ttf;
use std::collections::HashMap;

pub struct SelectBox {
    pub height: u32,
    pub width: u32,
    pub location: Point,
    pub text_color: [u8; 3],
    pub background_color: [u8; 3],
    pub hover_color: [u8; 3],
    pub text: String,
    pub hover: bool,
    pub active: bool,
    pub options: Vec<String>,
}

impl SelectBox {
    fn option_buttons(&self) {
        let mut buttons: Vec<Button> = Vec::new();
        for option in self.options.iter() {
            todo!()
        }
    }
}
