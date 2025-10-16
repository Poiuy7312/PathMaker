use sdl2::rect::Point;

pub mod board;
pub mod button;
pub mod file_explorer;
pub mod inputbox;
pub mod widget;
//pub mod selectbox;

pub trait Component {
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<String>);
    fn get_id(&self) -> String;
    fn change_location(&mut self, new_location: Point);
    fn change_active(&mut self, new_value: bool);
    fn is_active(&self) -> bool;
    fn get_location(&self) -> Point;
    fn change_width(&mut self, new_width: u32);
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;
    fn change_height(&mut self, new_height: u32);
    fn mouse_over_component(&self, mouse_position: Point) -> bool;
}
