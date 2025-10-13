use sdl2::rect::Point;

use crate::components::button::*;

pub fn layout_root(
    buttons: &mut Vec<Box<dyn ValidDropdownOption>>,
    origin: Point,
    width: u32,
    height: u32,
    filter: Option<&str>,
) {
    let mut offset: u32 = 0;
    for b in buttons.iter_mut().filter(|a| a.contains(filter)) {
        let y = origin.y + (offset as i32 * height as i32);
        let loc = Point::new(origin.x, y);
        let used = b.layout(loc, width, height);
        println!("{}", used);
        offset += used;
    }
}
