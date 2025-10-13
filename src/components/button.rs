use std::collections::HashMap;

use sdl2::mouse::MouseState;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::sys::False;
use sdl2::video::{Window, WindowContext};

use crate::components::Component;

use sdl2::ttf;

#[derive(Copy, Clone)]
pub enum ButtonType {
    Dropdown,
    Standard,
    Switch,
}

#[derive(Copy, Clone)]

pub enum ValidDropdownType {
    Dropdown,
    Standard,
}

pub trait Button: Component {
    fn mouse_over_component(&self, mouse_position: Point) -> bool;
    fn get_rect(&self, point: Point) -> Rect;
    fn change_active(&mut self, new_value: bool);
    fn is_active(&self) -> bool;
    fn get_location(&self) -> Point;
    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Option<Point>,
        font: &mut ttf::Font<'_, 'static>,
    );
}

pub trait ValidDropdownOption: Button {
    fn contains(&self, text: Option<&str>) -> bool;
    fn layout(&mut self, origin: Point, width: u32, height: u32) -> u32;
    fn set_filter(&mut self, text: Option<&str>);
    fn get_type(&self) -> ValidDropdownType;
}

/// Stores Color values to style buttons
pub struct ButtonStyle {
    pub text_color: Color,
    pub background_color: Color,
    pub hover_color: Color,
}

/// Struct for simple button can only be a rectangle
#[derive(Clone)]
pub struct StandardButton {
    pub height: u32,
    pub width: u32,
    pub location: Point,
    pub text_color: Color,
    pub background_color: Color,
    pub hover_color: Color,
    pub text: String,
    pub id: String,
    pub filter: Option<String>,
    pub active: bool,
}

impl Component for StandardButton {
    fn on_click(&mut self, mouse_position: Point) -> (bool, Option<&str>) {
        return (
            self.mouse_over_component(mouse_position),
            Some(self.get_id()),
        );
    }
    fn get_id(&self) -> &str {
        return &self.id;
    }
    fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }
    fn get_width(&self) -> u32 {
        self.width
    }
    fn get_height(&self) -> u32 {
        self.height
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl Button for StandardButton {
    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect(self.location);
        return component.contains_point(mouse_position) && self.active;
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

    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn get_location(&self) -> Point {
        return self.location;
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Option<Point>,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        let button_background: Rect = self.get_rect(self.location);
        let font_size = 8 * self.text.chars().count() as u32;
        let button_outline =
            Rect::from_center(button_background.center(), self.width + 5, self.height + 5);
        let text_map = Rect::from_center(button_background.center(), font_size, 20);
        let mouse_state = mouse_state.expect("No mouse state given");
        font.set_style(sdl2::ttf::FontStyle::BOLD);
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.fill_rect(button_outline).unwrap();

        // render a surface, and convert it to a texture bound to the canvas
        if self.mouse_over_component((mouse_state)) {
            canvas.set_draw_color(self.hover_color);
            canvas.fill_rect(button_background).unwrap();
        } else {
            canvas.set_draw_color(self.background_color);
            canvas.fill_rect(button_background).unwrap();
        }

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

    fn get_type(&self) -> ValidDropdownType {
        return ValidDropdownType::Standard;
    }

    fn layout(&mut self, origin: Point, width: u32, _: u32) -> u32 {
        self.location = origin;
        self.width = width;
        if self.contains(self.filter.as_deref()) {
            println!("Return 1 for layout");
            return 1;
        }
        println!("Return 0 for layout");
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

/// Creates a dropdown menu.
pub struct Dropdown {
    pub height: u32,
    pub width: u32,
    pub location: Point,
    pub text_color: Color,
    pub background_color: Color,
    pub hover_color: Color,
    pub text: String,
    pub id: String,
    pub active: bool,
    pub clicked_on: bool,
    pub options: Vec<Box<dyn ValidDropdownOption>>,
    pub filter: Option<String>,
}

impl Component for Dropdown {
    fn on_click(&mut self, mouse_position: Point) -> (bool, Option<&str>) {
        if self.mouse_over_component(mouse_position) {
            self.clicked_on = !self.clicked_on;
            return (true, None);
        }

        if self.clicked_on {
            self.options.iter_mut().for_each(|a| a.change_active(true));
            let (option_clicked, checked_option) = self.check_options(mouse_position);
            if option_clicked {
                //println!("Found clicked child: {:#?}", checked_option);
                return (true, checked_option);
            }
        } else {
            self.options.iter_mut().for_each(|a| a.change_active(false));
        }

        (false, None)
    }

    fn get_id(&self) -> &str {
        return &self.id;
    }

    fn change_location(&mut self, new_location: Point) {
        self.location = new_location;
    }

    fn get_width(&self) -> u32 {
        self.width
    }
    fn get_height(&self) -> u32 {
        self.height
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl Button for Dropdown {
    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        let component: Rect = self.get_rect(self.location);
        return component.contains_point(mouse_position) && self.active;
    }

    fn change_active(&mut self, new_value: bool) {
        if self.contains(self.filter.as_deref()) {
            self.active = new_value;
        } else {
            self.active = false;
        }
        if !self.active {
            self.clicked_on = false;
            self.options
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

    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn get_location(&self) -> Point {
        return self.location;
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Option<Point>,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        let button_background: Rect = self.get_rect(self.location);
        let font_size = 8 * self.text.chars().count() as u32;
        let button_outline =
            Rect::from_center(button_background.center(), self.width + 5, self.height + 5);
        let text_map = Rect::from_center(button_background.center(), font_size, 20);
        let mouse_state = mouse_state.expect("No mouse state given");
        font.set_style(sdl2::ttf::FontStyle::BOLD);
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.fill_rect(button_outline).unwrap();

        // render a surface, and convert it to a texture bound to the canvas
        if self.mouse_over_component(mouse_state) {
            canvas.set_draw_color(self.hover_color);
            canvas.fill_rect(button_background).unwrap();
        } else {
            canvas.set_draw_color(self.background_color);
            canvas.fill_rect(button_background).unwrap();
        }

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
        canvas.set_draw_color(Color::RGB(0, 0, 0));

        for line in lines {
            canvas.draw_lines(&line[..]).unwrap();
        }
        // Draw the arrow as a filled triangle
        // SDL2's Canvas does not have a fill_polygon, so we can only draw the outline.
        // If you want a filled triangle, you need to use an external crate or draw lines manually.
        // Here, we just draw the triangle outline.

        if self.clicked_on {
            self.options
                .iter()
                .for_each(|a| match a.contains(self.filter.as_deref()) {
                    true => {
                        a.draw(canvas, texture_creator, Some(mouse_state), font);
                    }
                    false => {}
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
                for option in self.options.iter() {
                    if option.contains(text) {
                        return true;
                    }
                }
            }
            return false;
        }
        return true;
    }

    fn set_filter(&mut self, text: Option<&str>) {
        match text {
            Some(value) => {
                self.filter = Some(value.to_string());
                self.options.iter_mut().for_each(|a| a.set_filter(text))
            }
            None => {
                self.filter = None;
            }
        }
    }
    fn get_type(&self) -> ValidDropdownType {
        return ValidDropdownType::Dropdown;
    }

    fn layout(&mut self, origin: Point, width: u32, height: u32) -> u32 {
        self.location = origin;
        self.width = width;

        if self.contains(self.filter.as_deref()) {
            // println!("Hello there");
            let mut consumed: u32 = 1;
            if self.clicked_on {
                let mut offset = 1;
                for child in self.options.iter_mut() {
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
        let x = self.location.x + self.width as i32 * 3 / 4;
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

    fn check_options(&mut self, mouse_position: Point) -> (bool, Option<&str>) {
        for a in self.options.iter_mut() {
            let (result, clicked_button) = a.on_click(mouse_position);
            if result {
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
    pub options: Vec<(String, StandardButton)>,
}

impl Component for OptionButton {
    fn get_id(&self) -> &str {
        return &self.id;
    }
    fn on_click(&mut self, mouse_position: Point) -> (bool, Option<&str>) {
        match self
            .options
            .iter()
            .find(|(_, a)| a.get_rect(a.location).contains_point(mouse_position))
        {
            Some((_, clicked_button)) => {
                return (true, Some(clicked_button.get_id()));
            }
            None => return (false, None),
        }
    }

    fn change_location(&mut self, new_location: Point) {
        let mut count = 0;
        self.location = new_location;
        self.options.iter_mut().for_each(|(_, b)| {
            b.change_location(Point::new(
                new_location.x() + count * b.width as i32,
                new_location.y(),
            ));
            count += 1;
        })
    }

    fn get_width(&self) -> u32 {
        self.width
    }
    fn get_height(&self) -> u32 {
        self.height
    }

    fn change_width(&mut self, new_width: u32) {
        self.width = new_width;
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
    }
}

impl Button for OptionButton {
    fn mouse_over_component(&self, mouse_position: Point) -> bool {
        self.options
            .iter()
            .find(|(_, a)| a.get_rect(a.location).contains_point(mouse_position))
            .is_some()
            && self.active
    }

    fn change_active(&mut self, new_value: bool) {
        self.active = new_value;
        self.options.iter_mut().for_each(|(_, a)| {
            a.change_active(new_value);
        })
    }

    fn is_active(&self) -> bool {
        return self.active;
    }

    fn get_rect(&self, point: Point) -> Rect {
        Rect::new(point.x(), point.y(), self.width, self.height)
    }

    fn get_location(&self) -> Point {
        return self.location;
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_state: Option<Point>,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        // Draw each button in the switch
        self.options.iter().for_each(|(_, b)| {
            b.draw(canvas, texture_creator, mouse_state, font);
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
        option_values: Vec<(String, ButtonStyle)>,
    ) -> Self {
        let mut options: Vec<(String, StandardButton)> = Vec::new();
        let mut count: i32 = 0;
        let button_width = width / option_values.len() as u32;
        option_values.iter().for_each(|(text, style)| {
            options.push((
                text.to_string(),
                StandardButton {
                    height,
                    width: button_width,
                    location: Point::new(location.x() + count * button_width as i32, location.y()),
                    text_color: style.text_color,
                    background_color: style.background_color,
                    hover_color: style.hover_color,
                    text: text.to_string(),
                    id: text.to_string(),
                    filter: None,
                    active,
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
            options,
        }
    }
}
