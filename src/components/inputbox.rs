use sdl2::mouse::MouseState;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use sdl2::ttf;

pub struct InputBox {
    pub default_text: String,
    pub text: String,
    pub active: bool,
    pub text_color: [u8; 3],
    pub background_color: [u8; 3],
    pub height: u32,
    pub width: u32,
    pub location: Point,
}

impl InputBox {
    fn mouse_over_component(&self, mouse_position: (i32, i32)) -> bool {
        let component: Rect = self.get_rect(self.location);
        let mouse_point = sdl2::rect::Point::new(mouse_position.0, mouse_position.1);
        return component.contains_point(mouse_point);
    }
    pub fn on_click(&mut self, mouse_state: &MouseState) -> bool {
        if self.mouse_over_component((mouse_state.x(), mouse_state.y())) {
            self.text = " ".to_string();
            self.active = true;
            return true;
        }
        return false;
    }
    fn get_rect(&self, point: Point) -> Rect {
        Rect::from_center(point, self.width, self.height)
    }
    pub fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        _: Option<&MouseState>,
    ) {
        let box_outline = Rect::from_center(self.location, self.width + 5, self.height + 5);
        let box_background: Rect = self.get_rect(self.location);
        let text_map_x = box_background.left() + 5;
        let text_map: Rect = Rect::new(
            text_map_x,
            (self.location.y - (self.height as i32 / 4)),
            self.width / 2,
            self.height / 2,
        );
        let ttf_context: ttf::Sdl2TtfContext = ttf::init().unwrap();

        let file = ttf_context.load_font("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 32);

        match file {
            Ok(value) => {
                let font = value;
                canvas.set_draw_color(Color::RGB(0, 0, 0));
                canvas.fill_rect(box_outline).unwrap();
                canvas.set_draw_color(Color::RGB(
                    self.background_color[0],
                    self.background_color[1],
                    self.background_color[2],
                ));
                canvas.fill_rect(box_background).unwrap();
                match self.text.len() > 0 {
                    true => {
                        let font_size = 8 * self.text.chars().count() as u32;
                        let text_map: Rect = Rect::new(
                            text_map_x,
                            self.location.y - (self.height as i32 / 4),
                            font_size,
                            self.height / 2,
                        );
                        let font_surface = font
                            .render(&self.text)
                            .blended_wrapped(
                                Color::RGB(
                                    self.text_color[0],
                                    self.text_color[1],
                                    self.text_color[2],
                                ),
                                (32 * self.text.chars().count()) as u32,
                            )
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
            Err(e) => {
                println!("{}", e);
            }
        }
    }
}
