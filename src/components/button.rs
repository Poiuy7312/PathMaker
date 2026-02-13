use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

use sdl2::mouse::MouseState;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::sys::False;
use sdl2::video::{Window, WindowContext};

use crate::colors::{
    BLACK, HOVER_COLOR, PRIMARY_COLOR, QUATERNARY_COLOR, SECONDARY_COLOR, TERTIARY_COLOR, WHITE,
};
use crate::components::Component;

use sdl2::ttf;

fn calculate_scaled_font_size(text_len: u32, available_width: u32) -> u32 {
    let char_width = 8;
    let estimated_width = text_len * char_width;

    if estimated_width > available_width {
        let scale = available_width as f32 / estimated_width as f32;
        (estimated_width as f32 * scale) as u32
    } else {
        estimated_width
    }
    .max(4) // minimum font size
}

pub trait Interface: Component {
    fn get_rect(&self, point: Point) -> Rect;
    fn is_static(&self) -> bool;
    fn has_indent(&self) -> bool;
    fn draw_priority(&self) -> u8;
    fn dirty_parent(&self) -> bool;
    fn deactivate_parent(&self) -> bool;
    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_position: Point,
        font: &mut ttf::Font<'_, 'static>,
    );
    fn as_any(&mut self) -> &mut dyn Any;
    fn change_drawn(&self, new_val: bool);
    fn is_drawn(&self) -> bool;
}

pub trait ValidDropdownOption: Interface {
    fn contains(&self, text: Option<&str>) -> bool;
    fn layout(&mut self, origin: Point, width: u32, height: u32) -> u32;
    fn set_filter(&mut self, text: Option<&str>);
}

#[derive(Clone)]
/// Stores Color values to style buttons
pub struct InterfaceStyle {
    pub text_color: Color,
    pub background_color: Color,
}

/// Struct for simple button can only be a rectangle
pub struct StandardButton {
    pub height: u32,
    pub width: u32,
    pub location: Point,
    pub text_color: Color,
    pub background_color: Color,
    pub hover: RefCell<bool>,
    pub text: String,
    pub id: String,
    pub filter: Option<String>,
    pub active: bool,
    pub drawn: RefCell<bool>,
    pub cached_texture: Option<Texture<'static>>,
}

impl Component for StandardButton {
    fn on_click(&mut self, mouse_position: Point) -> (bool, Option<String>) {
        return (
            self.mouse_over_component(mouse_position),
            Some(self.get_id()),
        );
    }

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect(self.location);
        return component.contains_point(mouse_position) && self.active;
    }

    fn get_id(&self) -> String {
        return self.id.to_string();
    }
    fn change_location(&mut self, new_location: Point) {
        if new_location != self.location {
            self.location = new_location;
        }
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

    fn change_active(&mut self, new_value: bool) {
        if self.contains(self.filter.as_deref()) {
            self.active = new_value;
        } else {
            self.active = false;
        }
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl Interface for StandardButton {
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

    fn deactivate_parent(&self) -> bool {
        false
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn change_drawn(&self, new_val: bool) {
        self.drawn.replace(new_val);
    }

    fn is_drawn(&self) -> bool {
        let drawn = unsafe { *self.drawn.as_ptr() };
        if drawn {
            return true;
        }
        return false;
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_position: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        let hovering = self.mouse_over_component(mouse_position);
        if self.is_hovering() != hovering {
            self.change_drawn(false);
            self.change_hover(hovering);
        }
        if self.is_drawn() {
            return;
        }

        let button_background: Rect = self.get_rect(self.location);
        let available_width = (self.width as i32 - 10) as u32;
        let text_len = self.text.chars().count() as u32;
        let font_size = calculate_scaled_font_size(text_len, available_width);
        let button_outline =
            Rect::from_center(button_background.center(), self.width + 5, self.height + 5);
        let mut text_map = Rect::from_center(button_background.center(), font_size, 20);
        if text_map.width() >= button_background.width() {
            text_map.set_width(button_background.width());
        }
        font.set_style(sdl2::ttf::FontStyle::BOLD);
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.fill_rect(button_outline).unwrap();

        let font_surface: Surface<'_>;

        // render a surface, and convert it to a texture bound to the canvas
        if hovering {
            canvas.set_draw_color(WHITE);
            canvas.fill_rect(button_background).unwrap();
            font_surface = if self.cached_texture.is_none() {
                font.render(&self.text)
                    .blended(BLACK)
                    .map_err(|e| e.to_string())
                    .unwrap()
            } else {
                // Use cached texture
                return;
            }
        } else {
            canvas.set_draw_color(self.background_color);
            canvas.fill_rect(button_background).unwrap();
            font_surface = if self.cached_texture.is_none() {
                font.render(&self.text)
                    .blended(self.text_color)
                    .map_err(|e| e.to_string())
                    .unwrap()
            } else {
                // Use cached texture
                return;
            }
        }

        let font_texture: Texture<'_> = texture_creator
            .create_texture_from_surface(&font_surface)
            .map_err(|e| e.to_string())
            .unwrap();
        canvas
            .copy(&font_texture, None, text_map)
            .expect("Button unable to display text");
        self.change_drawn(true);
    }
}

impl ValidDropdownOption for StandardButton {
    fn set_filter(&mut self, text: Option<&str>) {
        match text {
            Some(value) => self.filter = Some(value.to_string()),
            None => {
                self.filter = None;
            }
        }
    }

    /*fn get_options(self: Box<Self>) -> Option<Vec<StandardButton>> {
        None
    }*/

    fn layout(&mut self, origin: Point, width: u32, height: u32) -> u32 {
        self.location = origin;
        self.width = width;
        self.height = height;
        if self.contains(self.filter.as_deref()) {
            return 1;
        }
        0
    }

    fn contains(&self, text: Option<&str>) -> bool {
        if text.is_some() {
            // println!("Checking if |{}| Contains: |{:#?}|", self.id, text);
            return self.id.contains(text.unwrap().trim());
        }
        return true;
    }
}

impl PartialEq for StandardButton {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location
    }
}

impl StandardButton {
    fn change_hover(&self, new_val: bool) {
        self.hover.replace(new_val);
    }

    fn change_text(&mut self, new_val: &str) {
        self.text = new_val.to_string();
    }
    fn is_hovering(&self) -> bool {
        let hover = unsafe { *self.hover.as_ptr() };
        if hover {
            return true;
        }
        return false;
    }
}

/// Creates a dropdown menu.

pub struct Dropdown {
    pub height: u32,
    pub width: u32,
    pub location: Point,
    pub text_color: Color,
    pub background_color: Color,
    pub hover: RefCell<bool>,
    pub text: String,
    pub id: String,
    pub active: bool,
    pub clicked_on: bool,
    pub options: RefCell<Vec<StandardButton>>,
    pub filter: Option<String>,
    pub drawn: RefCell<bool>,
}

impl Component for Dropdown {
    fn on_click(&mut self, mouse_position: Point) -> (bool, Option<String>) {
        if self.mouse_over_component(mouse_position) {
            if self.clicked_on {
                self.clicked_on = false;
            } else {
                self.clicked_on = true;
            }
            return (true, None);
        }
        if self.clicked_on {
            self.options
                .borrow_mut()
                .iter_mut()
                .for_each(|a| a.change_active(true));
            let (option_clicked, _) = self.check_options(mouse_position);
            if option_clicked {
                self.clicked_on = false;
                println!("{}", self.text);
                return (true, Some(self.get_id()));
            }
        } else {
            self.options
                .borrow_mut()
                .iter_mut()
                .for_each(|a| a.change_active(false));
            return (false, None);
        }

        (false, None)
    }

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect(self.location);
        return component.contains_point(mouse_position) && self.active;
    }

    fn get_id(&self) -> String {
        return self.id.to_string();
    }

    fn change_location(&mut self, new_location: Point) {
        if new_location != self.location {
            self.location = new_location;
        }
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

    fn change_active(&mut self, new_value: bool) {
        if self.contains(self.filter.as_deref()) {
            self.active = new_value;
        } else {
            self.active = false;
        }
        if !self.active {
            self.options
                .borrow_mut()
                .iter_mut()
                .filter(|a| a.contains(self.filter.as_deref()))
                .for_each(|a| {
                    a.change_active(false);
                });
        }
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl Interface for Dropdown {
    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn is_static(&self) -> bool {
        true
    }
    fn has_indent(&self) -> bool {
        false
    }

    fn draw_priority(&self) -> u8 {
        2
    }

    fn dirty_parent(&self) -> bool {
        true
    }

    fn deactivate_parent(&self) -> bool {
        true
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn change_drawn(&self, new_val: bool) {
        self.drawn.replace(new_val);
    }

    fn is_drawn(&self) -> bool {
        let drawn = unsafe { *self.drawn.as_ptr() };
        if drawn {
            return true;
        }
        return false;
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_position: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        let button_background: Rect = self.get_rect(self.location);
        let available_width = ((self.width * 8 / 10) - 40) as u32;
        let text_len = self.text.chars().count() as u32;
        let font_size = calculate_scaled_font_size(text_len, available_width);
        let button_outline =
            Rect::from_center(button_background.center(), self.width + 5, self.height + 5);
        let text_map = Rect::new(
            button_background.left() + 2,
            button_background.top(),
            font_size,
            self.height,
        );
        font.set_style(sdl2::ttf::FontStyle::BOLD);
        canvas.set_draw_color(BLACK);
        canvas.fill_rect(button_outline).unwrap();

        // render a surface, and convert it to a texture bound to the canvas
        if self.mouse_over_component(mouse_position) {
            canvas.set_draw_color(WHITE);
            canvas.fill_rect(button_background).unwrap();
            let font_surface = font
                .render(&self.text)
                .blended(BLACK)
                .map_err(|e| e.to_string())
                .unwrap();

            let font_texture: Texture<'_> = texture_creator
                .create_texture_from_surface(&font_surface)
                .map_err(|e| e.to_string())
                .unwrap();
            canvas
                .copy(&font_texture, None, text_map)
                .expect("Button unable to display text");

            let lines: Vec<[Point; 3]> = self.get_arrow_graphic();
            canvas.set_draw_color(BLACK);

            for line in lines {
                canvas.draw_lines(&line[..]).unwrap();
            }
        } else {
            canvas.set_draw_color(self.background_color);
            canvas.fill_rect(button_background).unwrap();
            let font_surface = font
                .render(&self.text)
                .blended(self.text_color)
                .map_err(|e| e.to_string())
                .unwrap();

            let font_texture: Texture<'_> = texture_creator
                .create_texture_from_surface(&font_surface)
                .map_err(|e| e.to_string())
                .unwrap();
            canvas
                .copy(&font_texture, None, text_map)
                .expect("Button unable to display text");

            let lines = self.get_arrow_graphic();
            canvas.set_draw_color(WHITE);

            for line in lines {
                canvas.draw_lines(&line[..]).unwrap();
            }
        }
        // Draw the arrow as a filled triangle
        // SDL2's Canvas does not have a fill_polygon, so we can only draw the outline.
        // If you want a filled triangle, you need to use an external crate or draw lines manually.
        // Here, we just draw the triangle outline.

        if self.clicked_on {
            self.layout_function();
            self.options.borrow_mut().iter_mut().for_each(|a| {
                a.change_active(true);
                a.draw(canvas, texture_creator, mouse_position, font);
            });
        }
    }
}

impl ValidDropdownOption for Dropdown {
    fn contains(&self, text: Option<&str>) -> bool {
        if text.is_some() {
            if self.id.contains(text.unwrap().trim()) {
                return true;
            } else {
                for option in self.options.borrow().iter() {
                    if option.contains(text) {
                        return true;
                    }
                }
            }
            return false;
        }
        return true;
    }

    /*fn get_options(self: Box<Self>) -> Option<Vec<StandardButton>> {
        return Some(self.options);
    }*/

    fn set_filter(&mut self, text: Option<&str>) {
        match text {
            Some(value) => {
                self.filter = Some(value.to_string());
                self.options
                    .borrow_mut()
                    .iter_mut()
                    .for_each(|a| a.set_filter(text))
            }
            None => {
                self.filter = None;
            }
        }
    }

    fn layout(&mut self, origin: Point, width: u32, height: u32) -> u32 {
        self.location = origin;
        self.width = width;

        if self.contains(self.filter.as_deref()) {
            // println!("Hello there");
            let mut consumed: u32 = 1;
            if self.clicked_on {
                let mut offset = 1;
                for child in self.options.borrow_mut().iter_mut() {
                    let child_x = self.location.x + (width as i32 / 4);
                    let child_y = self.location.y + (offset as i32 * height as i32);
                    let child_origin = Point::new(child_x, child_y);
                    let used = child.layout(child_origin, (width * 3) / 4, height);
                    offset += used;
                    consumed += used;
                }
                //println!("Offset of |{:#?}|: {:#?}", self.id, offset);
            }
            consumed
        } else {
            //println!("Return 0 for layout");
            return 0;
        }
    }
}

impl Dropdown {
    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn get_arrow_graphic(&self) -> Vec<[Point; 3]> {
        let x = self.location.x + self.width as i32 * 4 / 5;
        let y = self.location.y + self.height as i32 / 2;
        let center_points = [Point::new(x, y - 1), Point::new(x, y), Point::new(x, y + 1)];
        let mut lines: Vec<[Point; 3]> = Vec::new();
        match self.clicked_on {
            true => {
                for point in center_points {
                    let left_point = Point::new(point.x - 5, point.y - 5);
                    let right_point = Point::new(point.x + 5, point.y - 5);
                    lines.push([left_point, point, right_point]);
                }
            }
            false => {
                for point in center_points {
                    let left_point = Point::new(point.x - 5, point.y + 5);
                    let right_point = Point::new(point.x + 5, point.y + 5);
                    lines.push([left_point, point, right_point]);
                }
            }
        }
        return lines;
    }

    fn layout_function(&self) {
        let mut offset: u32 = 1;
        for b in self.options.borrow_mut().iter_mut() {
            let col = self.location.y + (offset as i32 * self.height as i32);
            let loc = Point::new(self.location.x, col);
            let used = b.layout(loc, self.width, self.height);
            b.change_drawn(false);
            offset += used;
        }
    }

    fn check_options(&mut self, mouse_position: Point) -> (bool, Option<String>) {
        for a in self.options.borrow_mut().iter_mut() {
            let (result, clicked_button) = a.on_click(mouse_position);
            if result {
                (a.text, self.text) = (self.text.clone(), a.text.clone());
                return (true, clicked_button);
            }
        }
        return (false, None);
    }
}

pub struct OptionButton {
    pub height: u32,
    pub width: u32,
    pub location: Point,
    pub id: String,
    pub active: bool,
    pub options: RefCell<Vec<(String, StandardButton)>>,
    active_option: Option<String>,
    defaults: HashMap<String, InterfaceStyle>,
    pub drawn: RefCell<bool>,
}

impl Component for OptionButton {
    fn get_id(&self) -> String {
        return self.id.to_string();
    }
    fn on_click(&mut self, mouse_position: Point) -> (bool, Option<String>) {
        let mut cur_option: Option<String> = None;
        self.options.borrow_mut().iter_mut().for_each(|(_, a)| {
            a.change_drawn(false);
            if a.get_rect(a.location).contains_point(mouse_position) {
                cur_option = Some(a.get_id());
            }
        });
        if cur_option.is_some() {
            self.active_option = cur_option.clone();
            return (true, cur_option);
        }
        return (false, None);
    }

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        self.options
            .borrow()
            .iter()
            .find(|(_, a)| a.get_rect(a.location).contains_point(mouse_position))
            .is_some()
            && self.active
    }

    fn change_location(&mut self, new_location: Point) {
        if new_location != self.location {
            let mut count = 0;
            self.location = new_location;
            self.options.borrow_mut().iter_mut().for_each(|(_, b)| {
                b.change_location(Point::new(
                    new_location.x() + count * b.width as i32,
                    new_location.y(),
                ));
                count += 1;
            })
        }
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

    fn change_active(&mut self, new_value: bool) {
        self.active = new_value;
        self.options.borrow_mut().iter_mut().for_each(|(_, a)| {
            a.change_active(new_value);
        })
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl Interface for OptionButton {
    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn is_static(&self) -> bool {
        true
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

    fn deactivate_parent(&self) -> bool {
        false
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
    fn change_drawn(&self, new_val: bool) {
        self.drawn.replace(new_val);
        self.options.borrow_mut().iter_mut().for_each(|(_, a)| {
            a.drawn.replace(new_val);
        });
    }

    fn is_drawn(&self) -> bool {
        let drawn = unsafe { *self.drawn.as_ptr() };
        if drawn {
            return true;
        }
        return false;
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_position: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        // Draw each button in the switch
        self.options.borrow_mut().iter_mut().for_each(|(_, b)| {
            if let Some(ac_option) = &self.active_option {
                if b.get_id() == *ac_option {
                    b.background_color = HOVER_COLOR;
                    b.text_color = BLACK;
                } else {
                    if let Some(default) = &self.defaults.get(&b.get_id()) {
                        b.background_color = default.background_color;
                        b.text_color = default.text_color;
                    }
                }
            }
            b.draw(canvas, texture_creator, mouse_position, font);
        })
    }
}

impl OptionButton {
    pub fn new(
        height: u32,
        width: u32,
        location: Point,
        id: String,
        active: bool,
        option_values: Vec<(String, InterfaceStyle)>,
        drawn: bool,
    ) -> Self {
        let mut options: Vec<(String, StandardButton)> = Vec::new();
        let mut defaults: HashMap<String, InterfaceStyle> = HashMap::new();
        let mut count: i32 = 0;
        let button_width = width / option_values.len() as u32;
        option_values.iter().for_each(|(text, style)| {
            defaults.insert(text.to_string(), style.clone());
            options.push((
                text.to_string(),
                StandardButton {
                    height,
                    width: button_width,
                    location: Point::new(location.x() + count * button_width as i32, location.y()),
                    text_color: style.text_color,
                    background_color: style.background_color,
                    hover: RefCell::new(false),
                    text: text.to_string(),
                    id: text.to_string(),
                    filter: None,
                    active,
                    drawn: RefCell::new(drawn),
                    cached_texture: None,
                },
            ));
            count += 1;
        });
        OptionButton {
            height,
            width,
            location,
            id,
            active,
            options: RefCell::new(options),
            active_option: None,
            defaults,
            drawn: RefCell::new(drawn),
        }
    }
}

pub struct CheckBox {
    pub label: String,
    pub checked: bool,
    pub location: Point,
    pub height: u32,
    pub width: u32,
    pub id: String,
    pub active: bool,
    pub drawn: RefCell<bool>,
}

impl Component for CheckBox {
    fn on_click(&mut self, mouse_position: Point) -> (bool, Option<String>) {
        if !self.active {
            return (false, None);
        }
        if self.mouse_over_component(mouse_position) {
            self.checked = !self.checked;
            return (true, Some(self.get_id()));
        } else {
            return (false, None);
        }
    }

    fn get_id(&self) -> String {
        return self.id.to_string();
    }

    fn change_location(&mut self, new_location: Point) {
        if new_location != self.location {
            self.location = new_location;
        }
    }

    fn change_active(&mut self, new_value: bool) {
        self.active = new_value;
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn get_location(&self) -> Point {
        return self.location;
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

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect(self.location);
        return component.contains_point(mouse_position) && self.active;
    }
}

impl Interface for CheckBox {
    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn draw_priority(&self) -> u8 {
        1
    }
    fn dirty_parent(&self) -> bool {
        false
    }

    fn deactivate_parent(&self) -> bool {
        false
    }

    fn is_static(&self) -> bool {
        false
    }
    fn has_indent(&self) -> bool {
        false
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_position: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        if self.is_drawn() && !self.mouse_over_component(mouse_position) {
            return; // Skip if already drawn and not hovering
        }

        let button_background: Rect = self.get_rect(self.location);

        let available_width = (self.width as i32 - 30) as u32;
        let text_len = self.label.chars().count() as u32;
        let font_size = calculate_scaled_font_size(text_len, available_width);
        let checkbox_button = Rect::new(
            button_background.x() + 5,
            button_background.center().y(),
            10,
            10,
        );
        let checkbox_outline = Rect::from_center(checkbox_button.center(), 15, 15);
        let text_map = Rect::new(
            checkbox_button.right() + 5,
            checkbox_button.top(),
            font_size,
            checkbox_button.height(),
        );

        font.set_style(sdl2::ttf::FontStyle::BOLD);
        canvas.set_draw_color(BLACK);
        canvas.fill_rect(checkbox_outline).unwrap();

        let font_surface: Surface<'_>;

        // render a surface, and convert it to a texture bound to the canvas

        canvas.set_draw_color(WHITE);
        canvas.fill_rect(checkbox_button).unwrap();
        font_surface = font
            .render(&self.label)
            .blended(BLACK)
            .map_err(|e| e.to_string())
            .unwrap();

        let font_texture: Texture<'_> = texture_creator
            .create_texture_from_surface(&font_surface)
            .map_err(|e| e.to_string())
            .unwrap();
        canvas
            .copy(&font_texture, None, text_map)
            .expect("Button unable to display text");
        if self.checked {
            let lines = self.get_check_graphic(&checkbox_button);
            canvas.set_draw_color(BLACK);

            canvas.draw_lines(&lines.0[..]).unwrap();
            canvas.draw_lines(&lines.1[..]).unwrap();
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn change_drawn(&self, new_val: bool) {
        self.drawn.replace(new_val);
    }

    fn is_drawn(&self) -> bool {
        let drawn = unsafe { *self.drawn.as_ptr() };
        if drawn {
            return true;
        }
        return false;
    }
}

impl CheckBox {
    fn get_check_graphic(&self, rect: &Rect) -> ([Point; 6], [Point; 6]) {
        (
            [
                Point::new(rect.x(), rect.y() + 1),
                Point::new(rect.right(), rect.bottom() + 1),
                Point::new(rect.x(), rect.y()),
                Point::new(rect.right(), rect.bottom()),
                Point::new(rect.x(), rect.y() - 1),
                Point::new(rect.right(), rect.bottom() - 1),
            ],
            [
                Point::new(rect.right(), rect.y() + 1),
                Point::new(rect.left(), rect.bottom() + 1),
                Point::new(rect.right(), rect.y()),
                Point::new(rect.left(), rect.bottom()),
                Point::new(rect.right(), rect.y() - 1),
                Point::new(rect.left(), rect.bottom() - 1),
            ],
        )
    }
}

pub struct Slider {
    pub height: u32,
    pub width: u32,
    pub location: Point,
    pub text_color: Color,
    pub background_color: Color,
    pub text: String,
    pub id: String,
    pub active: bool,
    pub range: u32,
    pub value: u32,
    pub slider_offset_axis: i32,
    pub drawn: RefCell<bool>,
    pub cached_texture: Option<Texture<'static>>,
    pub is_vertical: bool,
    pub minimal: bool,
}

impl Component for Slider {
    fn on_click(&mut self, mouse_position: Point) -> (bool, Option<String>) {
        if !self.active {
            return (false, None);
        }
        if self.mouse_over_component(mouse_position) {
            self.change_slider_value(mouse_position);
            return (true, Some(self.get_id()));
        } else {
            return (false, None);
        }
    }

    fn get_id(&self) -> String {
        return self.id.to_string();
    }

    fn change_location(&mut self, new_location: Point) {
        if new_location != self.location {
            if self.is_vertical {
                let slider_dif = (self.location.y() - self.slider_offset_axis).abs();
                let new_y = new_location.y();
                self.location = new_location;
                let mut slider_height = 10;
                if self.height / self.range > 10 {
                    slider_height = self.height / self.range;
                }
                self.slider_offset_axis =
                    (new_y + slider_height as i32 / 2).max(new_y + slider_dif);
            } else {
                let slider_dif = (self.location.x() - self.slider_offset_axis).abs();
                let new_x = new_location.x();
                self.location = new_location;
                let mut slider_width = 10;
                if self.width / self.range > 10 {
                    slider_width = self.width / self.range;
                }
                self.slider_offset_axis = (new_x + slider_width as i32 / 2).max(new_x + slider_dif);
            }
        }
    }

    fn change_active(&mut self, new_value: bool) {
        self.active = new_value;
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn get_location(&self) -> Point {
        return self.location;
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

    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect(self.location);
        return component.contains_point(mouse_position) && self.active;
    }
}

impl Interface for Slider {
    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn is_static(&self) -> bool {
        false
    }

    fn draw_priority(&self) -> u8 {
        1
    }

    fn dirty_parent(&self) -> bool {
        false
    }

    fn deactivate_parent(&self) -> bool {
        false
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_position: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        if self.minimal {
            canvas.set_draw_color(SECONDARY_COLOR);
            canvas.fill_rect(self.get_rect(self.location)).unwrap();
            // Draw only the slider thumb
            let slider = if self.is_vertical {
                self.calc_slider_vertical()
            } else {
                self.calc_slider_horizontal()
            };

            canvas.set_draw_color(PRIMARY_COLOR);
            canvas.fill_rect(slider).unwrap();
            return;
        }

        // Original full slider rendering (horizontal only)
        let button_background: Rect = self.get_rect(self.location);
        let available_width = (self.width as i32 - 10) as u32;
        let text_len = self.text.chars().count() as u32;
        let font_size = calculate_scaled_font_size(text_len, available_width);
        let button_outline =
            Rect::from_center(button_background.center(), self.width + 5, self.height + 5);
        let mut text_map = Rect::new(
            button_background.left() + 5,
            button_background.top(),
            font_size,
            self.height / 2,
        );
        let slider_background = Rect::new(
            button_background.x(),
            text_map.bottom(),
            self.width,
            self.height / 2,
        );
        let slider_text = format!("{}: {}", self.text, self.value);
        let font_surface: Surface<'_>;
        let slider = self.calc_slider_horizontal();
        if text_map.width() >= button_background.width() {
            text_map.set_width(button_background.width());
        }
        canvas.set_draw_color(BLACK);
        canvas.fill_rect(button_outline).unwrap();
        canvas.set_draw_color(SECONDARY_COLOR);
        canvas.fill_rect(button_background).unwrap();
        canvas.set_draw_color(HOVER_COLOR);
        canvas.fill_rect(slider_background).unwrap();
        font_surface = font
            .render(&slider_text)
            .blended(self.text_color)
            .map_err(|e| e.to_string())
            .unwrap();
        let font_texture: Texture<'_> = texture_creator
            .create_texture_from_surface(&font_surface)
            .map_err(|e| e.to_string())
            .unwrap();
        canvas
            .copy(&font_texture, None, text_map)
            .expect("Button unable to display text");
        canvas.set_draw_color(PRIMARY_COLOR);
        canvas.fill_rect(slider).unwrap();
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn change_drawn(&self, new_val: bool) {
        self.drawn.replace(new_val);
    }

    fn is_drawn(&self) -> bool {
        let drawn = unsafe { *self.drawn.as_ptr() };
        if drawn {
            return true;
        }
        return false;
    }

    fn has_indent(&self) -> bool {
        false
    }
}

impl Slider {
    fn calc_slider_horizontal(&self) -> Rect {
        let slider_width = self.get_slider_width();
        let slider_background = Rect::new(
            self.location.x(),
            self.location.y() + self.height as i32 / 2,
            self.width,
            self.height / 2,
        );
        let slider_location =
            Point::from((self.slider_offset_axis, slider_background.center().y()));
        Rect::from_center(slider_location, slider_width, slider_background.height())
    }

    fn calc_slider_vertical(&self) -> Rect {
        let slider_height = self.get_slider_width();
        let slider_location = Point::from((
            self.location.x() + self.width as i32 / 2,
            self.slider_offset_axis,
        ));
        Rect::from_center(slider_location, self.width, slider_height)
    }

    fn get_slider_width(&self) -> u32 {
        let dimension = if self.is_vertical {
            self.height
        } else {
            self.width
        };
        if dimension / self.range > 10 {
            return dimension / self.range;
        }
        10
    }

    pub fn change_slider_value(&mut self, mouse_position: Point) {
        if self.is_vertical {
            self.change_slider_value_vertical(mouse_position);
        } else {
            self.change_slider_value_horizontal(mouse_position);
        }
    }

    pub fn change_slider_value_horizontal(&mut self, mouse_position: Point) {
        let new_value = mouse_position.x();
        let slider_width = self.get_slider_width() as i32;
        if new_value != self.slider_offset_axis {
            self.slider_offset_axis = new_value
                .max(self.location.x() + slider_width / 2)
                .min(self.location.x() + self.width as i32 - slider_width / 2);
        }
        let relative_location = self.slider_offset_axis - self.location.x() - slider_width / 2;
        let mut ratio = 1.0;
        if self.range != self.width && self.width > 0 {
            ratio = self.range as f32 / (self.width - slider_width as u32) as f32;
        }
        self.value = ((relative_location as f32 * ratio) as u32)
            .max(0)
            .min(self.range);
    }

    pub fn change_slider_value_vertical(&mut self, mouse_position: Point) {
        let new_value = mouse_position.y();
        let slider_height = self.get_slider_width() as i32;
        if new_value != self.slider_offset_axis {
            self.slider_offset_axis = new_value
                .max(self.location.y() + slider_height / 2)
                .min(self.location.y() + self.height as i32 - slider_height / 2);
        }
        let relative_location = self.slider_offset_axis - self.location.y() - slider_height / 2;
        let mut ratio = 1.0;
        if self.range != self.height && self.height > 0 {
            ratio = self.range as f32 / (self.height - slider_height as u32) as f32;
        }
        self.value = ((relative_location as f32 * ratio) as u32)
            .max(0)
            .min(self.range);
    }
}
