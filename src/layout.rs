use std::collections::HashMap;

use sdl2::rect::Point;

use crate::components::{button::*, inputbox::*, Component};

pub fn layout_root(
    buttons: &mut Vec<StandardButton>,
    origin: Point,
    width: u32,
    height: u32,
    filter: Option<&str>,
) {
    let mut offset: u32 = 0;
    for b in buttons.iter_mut().filter(|a| a.contains(filter)) {
        let col = origin.y + (offset as i32 * height as i32);
        let loc = Point::new(origin.x, col);
        let used = b.layout(loc, width, height);
        offset += used;
    }
}
