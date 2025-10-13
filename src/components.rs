use sdl2::rect::Point;

pub mod board;
pub mod button;
pub mod inputbox;
//pub mod selectbox;

pub trait Component {
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<&str>);
    fn get_id(&self) -> &str;
    fn change_location(&mut self, new_location: Point);
    fn change_width(&mut self, new_width: u32);
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;
    fn change_height(&mut self, new_width: u32);
}
