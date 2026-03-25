//! # Button Components Module
//!
//! This module provides various button and interactive UI components:
//!
//! ## Components
//! - **StandardButton**: Basic clickable button with text
//! - **Dropdown**: Expandable menu with selectable options
//! - **OptionButton**: Radio-button style selector with multiple options
//! - **CheckBox**: Toggle button with check mark
//! - **Slider**: Horizontal or vertical value slider
//!
//! All components implement the `Interface` trait for consistent rendering
//! and interaction handling.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

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
use crate::util;

use sdl2::ttf;

/// Extended interface trait for renderable UI components.
///
/// Builds on the Component trait with additional methods for:
/// - Rendering with fonts and textures
/// - Layout and sizing behavior
/// - Draw state management
pub trait Interface: Component {
    /// Get the bounding rectangle at a specific position.
    fn get_rect(&self, point: Point) -> Rect;

    /// Returns true if the component should not be resized by layouts.
    fn is_static(&self) -> bool;

    /// Returns true if the component should have indentation in layouts.
    fn has_indent(&self) -> bool;

    /// Priority for rendering order (higher = rendered later/on top).
    fn draw_priority(&self) -> u8;

    /// Returns true if clicking this component should mark parent as dirty.
    fn dirty_parent(&self) -> bool;

    /// Returns true if this component blocks other components when clicked.
    fn important_component_clicked(&self) -> bool;

    /// Returns true if clicking should deactivate other components.
    fn deactivate_parent(&self) -> bool;

    /// Returns true if click should be processed after mouse release.
    fn after_click(&self) -> bool;

    /// Render the component to the canvas.
    ///
    /// # Arguments
    /// * `canvas` - SDL2 canvas to draw on
    /// * `texture_creator` - For creating text textures
    /// * `mouse_position` - Current mouse position for hover effects
    /// * `font` - Font for text rendering
    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_position: Point,
        font: &mut ttf::Font<'_, 'static>,
    );

    /// Change the component's display text/label.
    fn change_label(&mut self, new_text: String);

    /// Get a mutable reference to the component as Any for downcasting.
    fn as_any(&mut self) -> &mut dyn Any;

    /// Set whether the component has been drawn this frame.
    fn change_drawn(&self, new_val: bool);

    /// Check if the component has already been drawn this frame.
    fn is_drawn(&self) -> bool;
}

/// Trait for components that can be used as dropdown menu options.
///
/// Extends Interface with filtering and layout capabilities.
pub trait ValidDropdownOption: Interface {
    /// Check if this option matches the current filter text.
    fn contains(&self, text: Option<&str>) -> bool;

    /// Position and size the component within a dropdown.
    ///
    /// # Returns
    /// Number of layout slots consumed (for variable-height options)
    fn layout(&mut self, origin: Point, width: u32, height: u32) -> u32;

    /// Set the filter text for this option.
    fn set_filter(&mut self, text: Option<&str>);
}

/// Style configuration for button components.
///
/// Stores colors that define a button's appearance.
#[derive(Clone)]
pub struct InterfaceStyle {
    /// Color for text rendering
    pub text_color: Color,
    /// Background fill color
    pub background_color: Color,
}
/// A simple clickable button with text.
///
/// Renders as a rectangle with centered text. Supports hover effects
/// and optional filtering for use in dropdowns.
pub struct StandardButton {
    /// Height in pixels
    pub height: u32,
    /// Width in pixels
    pub width: u32,
    /// Screen position
    pub location: Point,
    /// Text color
    pub text_color: Color,
    /// Background color
    pub background_color: Color,
    /// Hover state (RefCell for interior mutability)
    pub hover: RefCell<bool>,
    /// Display text
    pub text: String,
    /// Unique identifier
    pub id: String,
    /// Optional filter text for dropdown matching
    pub filter: Option<String>,
    /// Whether the button is interactive
    pub active: bool,
    /// Draw state flag
    pub drawn: RefCell<bool>,
    /// Cached texture for text (optimization)
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
    fn important_component_clicked(&self) -> bool {
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

    fn change_label(&mut self, new_text: String) {
        self.text = new_text
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
            #[cfg(not(target_os = "macos"))]
            self.change_drawn(false);
            self.change_hover(hovering);
        }
        #[cfg(not(target_os = "macos"))]
        if self.is_drawn() {
            return;
        }

        let button_background: Rect = self.get_rect(self.location);
        let available_width = (self.width as i32 - 10) as u32;
        let text_len = self.text.chars().count() as u32;
        let font_size = util::calculate_scaled_font_size(text_len, available_width);
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

    fn after_click(&self) -> bool {
        true
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

impl fmt::Debug for StandardButton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{:#?},{},{}",
            self.id, self.location, self.height, self.width
        )
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

/// A expandable dropdown menu with selectable options.
///
/// When clicked, expands to show a list of StandardButton options.
/// Selecting an option swaps its text with the dropdown's display text.
pub struct Dropdown {
    /// Height in pixels
    pub height: u32,
    /// Width in pixels
    pub width: u32,
    /// Screen position
    pub location: Point,
    /// Text color
    pub text_color: Color,
    /// Background color
    pub background_color: Color,
    /// Hover state
    pub hover: RefCell<bool>,
    /// Currently displayed text (selected option)
    pub text: String,
    /// Unique identifier
    pub id: String,
    /// Whether the dropdown is interactive
    pub active: bool,
    /// Whether the dropdown is currently expanded
    pub clicked_on: bool,
    /// List of selectable options
    pub options: RefCell<Vec<StandardButton>>,
    /// Optional filter for searching options
    pub filter: Option<String>,
    /// Draw state flag
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

    fn after_click(&self) -> bool {
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
    fn important_component_clicked(&self) -> bool {
        self.clicked_on
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

    fn change_label(&mut self, new_text: String) {
        self.text = new_text
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
        let font_size = util::calculate_scaled_font_size(text_len, available_width);
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

/// A radio-button style selector with multiple exclusive options.
///
/// Displays all options as adjacent buttons. Clicking one deselects others
/// and highlights the selected option.
pub struct OptionButton {
    /// Height in pixels
    pub height: u32,
    /// Width in pixels
    pub width: u32,
    /// Screen position
    pub location: Point,
    /// Unique identifier
    pub id: String,
    /// Whether the component is interactive
    pub active: bool,
    /// Option buttons with their labels
    pub options: RefCell<Vec<(String, StandardButton)>>,
    /// Currently selected option ID
    active_option: Option<String>,
    /// Default styles for each option (for deselected state)
    defaults: HashMap<String, InterfaceStyle>,
    /// Draw state flag
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
        let button_width = self.width as usize / self.options.borrow().len();
        self.options
            .borrow_mut()
            .iter_mut()
            .enumerate()
            .for_each(|(count, button)| {
                button.1.location = Point::new(
                    self.location.x() + (count * button_width) as i32,
                    self.location.y(),
                );
                button.1.change_width(button_width as u32);
            });
    }
    fn change_height(&mut self, new_height: u32) {
        self.height = new_height;
        self.options
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, button)| {
                button.change_height(new_height);
            })
    }
}

impl Interface for OptionButton {
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

    fn change_label(&mut self, _: String) {
        return;
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
        let size = option_values.len();
        let mut options: Vec<(String, StandardButton)> = Vec::with_capacity(size);
        let mut defaults: HashMap<String, InterfaceStyle> = HashMap::with_capacity(size);
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

/// A toggle checkbox with label.
///
/// Renders as a small square checkbox with text label.
/// Clicking toggles the checked state.
pub struct CheckBox {
    /// Display label text
    pub label: String,
    /// Current checked state
    pub checked: bool,
    /// Screen position
    pub location: Point,
    /// Height in pixels
    pub height: u32,
    /// Width in pixels
    pub width: u32,
    /// Unique identifier
    pub id: String,
    /// Whether the checkbox is interactive
    pub active: bool,
    /// Draw state flag
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

    fn important_component_clicked(&self) -> bool {
        false
    }

    fn deactivate_parent(&self) -> bool {
        false
    }

    fn after_click(&self) -> bool {
        true
    }

    fn is_static(&self) -> bool {
        false
    }
    fn has_indent(&self) -> bool {
        false
    }

    fn change_label(&mut self, new_text: String) {
        self.label = new_text
    }

    fn draw<'a>(
        &self,
        canvas: &mut Canvas<Window>,
        texture_creator: &'a TextureCreator<WindowContext>,
        mouse_position: Point,
        font: &mut ttf::Font<'_, 'static>,
    ) {
        #[cfg(not(target_os = "macos"))]
        if self.is_drawn() && !self.mouse_over_component(mouse_position) {
            return; // Skip if already drawn and not hovering
        }

        let button_background: Rect = self.get_rect(self.location);

        let available_width = (self.width as i32 - 30) as u32;
        let text_len = self.label.chars().count() as u32;
        let font_size = util::calculate_scaled_font_size(text_len, available_width);
        let checkbox_button = Rect::new(
            button_background.x() + 5,
            button_background.center().y(),
            10,
            10,
        );
        let checkbox_outline = Rect::from_center(checkbox_button.center(), 15, 15);
        let text_map = Rect::new(
            checkbox_button.right() + 5,
            checkbox_outline.top(),
            font_size,
            checkbox_outline.height(),
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

/// A draggable value slider.
///
/// Can be horizontal or vertical. Supports both full (with label)
/// and minimal (thumb-only) rendering modes.
pub struct Slider {
    /// Height in pixels
    pub height: u32,
    /// Width in pixels
    pub width: u32,
    /// Screen position
    pub location: Point,
    /// Label text color
    pub text_color: Color,
    /// Track background color
    pub background_color: Color,
    /// Label text
    pub text: String,
    /// Unique identifier
    pub id: String,
    /// Whether the slider is interactive
    pub active: bool,
    /// Maximum value of the slider
    pub range: u32,
    /// Current value
    pub value: u32,
    /// Position of the slider thumb along its axis
    pub slider_offset_axis: i32,
    /// Draw state flag
    pub drawn: RefCell<bool>,
    /// Cached texture for text (optimization)
    pub cached_texture: Option<Texture<'static>>,
    /// True for vertical slider, false for horizontal
    pub is_vertical: bool,
    /// True for minimal rendering (thumb only, no label)
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
    fn important_component_clicked(&self) -> bool {
        false
    }

    fn deactivate_parent(&self) -> bool {
        false
    }

    fn after_click(&self) -> bool {
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
        let font_size = util::calculate_scaled_font_size(text_len, available_width);
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

    fn change_label(&mut self, new_text: String) {
        self.text = new_text
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

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::pixels::Color;
    use sdl2::rect::Point;
    use std::cell::RefCell;

    fn make_standard_button(x: i32, y: i32, w: u32, h: u32) -> StandardButton {
        StandardButton {
            height: h,
            width: w,
            location: Point::new(x, y),
            text_color: WHITE,
            background_color: PRIMARY_COLOR,
            hover: RefCell::new(false),
            text: "Test".to_string(),
            id: "test_btn".to_string(),
            filter: None,
            active: true,
            drawn: RefCell::new(false),
            cached_texture: None,
        }
    }

    fn make_checkbox(x: i32, y: i32, w: u32, h: u32) -> CheckBox {
        CheckBox {
            label: "Test Check".to_string(),
            checked: false,
            location: Point::new(x, y),
            height: h,
            width: w,
            id: "test_cb".to_string(),
            active: true,
            drawn: RefCell::new(false),
        }
    }

    fn make_slider(x: i32, y: i32, w: u32, h: u32, range: u32) -> Slider {
        Slider {
            height: h,
            width: w,
            location: Point::new(x, y),
            text_color: BLACK,
            background_color: SECONDARY_COLOR,
            text: "Test Slider".to_string(),
            id: "test_slider".to_string(),
            active: true,
            range,
            slider_offset_axis: x,
            drawn: RefCell::new(false),
            cached_texture: None,
            value: 0,
            is_vertical: false,
            minimal: false,
        }
    }

    fn make_dropdown(x: i32, y: i32, w: u32, h: u32) -> Dropdown {
        Dropdown {
            height: h,
            width: w,
            location: Point::new(x, y),
            text_color: WHITE,
            background_color: PRIMARY_COLOR,
            hover: RefCell::new(false),
            text: "Option A".to_string(),
            id: "test_dd".to_string(),
            active: true,
            clicked_on: false,
            options: RefCell::new(vec![StandardButton {
                height: h,
                width: w,
                location: Point::new(x, y + h as i32),
                text_color: WHITE,
                background_color: PRIMARY_COLOR,
                hover: RefCell::new(false),
                text: "Option B".to_string(),
                id: "Option B".to_string(),
                filter: None,
                active: true,
                drawn: RefCell::new(false),
                cached_texture: None,
            }]),
            filter: None,
            drawn: RefCell::new(false),
        }
    }

    // ==================== StandardButton tests ====================

    #[test]
    fn test_standard_button_get_id() {
        let btn = make_standard_button(0, 0, 100, 50);
        assert_eq!(btn.get_id(), "test_btn");
    }

    #[test]
    fn test_standard_button_mouse_over_inside() {
        let btn = make_standard_button(10, 10, 100, 50);
        assert!(btn.mouse_over_component(Point::new(50, 30)));
    }

    #[test]
    fn test_standard_button_mouse_over_outside() {
        let btn = make_standard_button(10, 10, 100, 50);
        assert!(!btn.mouse_over_component(Point::new(200, 200)));
    }

    #[test]
    fn test_standard_button_mouse_over_inactive() {
        let mut btn = make_standard_button(10, 10, 100, 50);
        btn.active = false;
        assert!(!btn.mouse_over_component(Point::new(50, 30)));
    }

    #[test]
    fn test_standard_button_on_click_inside() {
        let mut btn = make_standard_button(10, 10, 100, 50);
        let (clicked, id) = btn.on_click(Point::new(50, 30));
        assert!(clicked);
        assert_eq!(id, Some("test_btn".to_string()));
    }

    #[test]
    fn test_standard_button_on_click_outside() {
        let mut btn = make_standard_button(10, 10, 100, 50);
        let (clicked, _) = btn.on_click(Point::new(200, 200));
        assert!(!clicked);
    }

    #[test]
    fn test_standard_button_change_location() {
        let mut btn = make_standard_button(0, 0, 100, 50);
        btn.change_location(Point::new(50, 50));
        assert_eq!(btn.get_location(), Point::new(50, 50));
    }

    #[test]
    fn test_standard_button_change_dimensions() {
        let mut btn = make_standard_button(0, 0, 100, 50);
        btn.change_width(200);
        btn.change_height(75);
        assert_eq!(btn.get_width(), 200);
        assert_eq!(btn.get_height(), 75);
    }

    #[test]
    fn test_standard_button_change_active() {
        let mut btn = make_standard_button(0, 0, 100, 50);
        assert!(btn.is_active());
        btn.change_active(false);
        assert!(!btn.is_active());
    }

    #[test]
    fn test_standard_button_change_label() {
        let mut btn = make_standard_button(0, 0, 100, 50);
        btn.change_label("New Label".to_string());
        assert_eq!(btn.text, "New Label");
    }

    #[test]
    fn test_standard_button_drawn_state() {
        let btn = make_standard_button(0, 0, 100, 50);
        assert!(!btn.is_drawn());
        btn.change_drawn(true);
        assert!(btn.is_drawn());
        btn.change_drawn(false);
        assert!(!btn.is_drawn());
    }

    #[test]
    fn test_standard_button_get_rect() {
        let btn = make_standard_button(10, 20, 100, 50);
        let rect = btn.get_rect(Point::new(10, 20));
        assert_eq!(rect.x(), 10);
        assert_eq!(rect.y(), 20);
        assert_eq!(rect.width(), 100);
        assert_eq!(rect.height(), 50);
    }

    #[test]
    fn test_standard_button_contains_no_filter() {
        let btn = make_standard_button(0, 0, 100, 50);
        assert!(btn.contains(None));
    }

    #[test]
    fn test_standard_button_contains_matching_filter() {
        let mut btn = make_standard_button(0, 0, 100, 50);
        btn.id = "Upload Map".to_string();
        assert!(btn.contains(Some("Upload")));
        assert!(btn.contains(Some("Map")));
    }

    #[test]
    fn test_standard_button_contains_non_matching_filter() {
        let mut btn = make_standard_button(0, 0, 100, 50);
        btn.id = "Upload Map".to_string();
        assert!(!btn.contains(Some("Download")));
    }

    #[test]
    fn test_standard_button_layout() {
        let mut btn = make_standard_button(0, 0, 100, 50);
        let used = btn.layout(Point::new(10, 20), 200, 30);
        assert_eq!(used, 1);
        assert_eq!(btn.location, Point::new(10, 20));
        assert_eq!(btn.width, 200);
        assert_eq!(btn.height, 30);
    }

    #[test]
    fn test_standard_button_equality() {
        let btn1 = make_standard_button(10, 20, 100, 50);
        let btn2 = make_standard_button(10, 20, 200, 75);
        assert_eq!(btn1, btn2); // Equality is by location only
    }

    #[test]
    fn test_standard_button_inequality() {
        let btn1 = make_standard_button(10, 20, 100, 50);
        let btn2 = make_standard_button(30, 40, 100, 50);
        assert_ne!(btn1, btn2);
    }

    #[test]
    fn test_standard_button_interface_traits() {
        let btn = make_standard_button(0, 0, 100, 50);
        assert!(!btn.is_static());
        assert!(!btn.has_indent());
        assert_eq!(btn.draw_priority(), 1);
        assert!(!btn.dirty_parent());
        assert!(!btn.important_component_clicked());
        assert!(!btn.deactivate_parent());
        assert!(btn.after_click());
    }

    // ==================== CheckBox tests ====================

    #[test]
    fn test_checkbox_toggle() {
        let mut cb = make_checkbox(0, 0, 100, 30);
        assert!(!cb.checked);
        cb.on_click(Point::new(50, 15));
        assert!(cb.checked);
        cb.on_click(Point::new(50, 15));
        assert!(!cb.checked);
    }

    #[test]
    fn test_checkbox_click_outside() {
        let mut cb = make_checkbox(0, 0, 100, 30);
        let (clicked, _) = cb.on_click(Point::new(200, 200));
        assert!(!clicked);
        assert!(!cb.checked);
    }

    #[test]
    fn test_checkbox_inactive_ignores_click() {
        let mut cb = make_checkbox(0, 0, 100, 30);
        cb.change_active(false);
        let (clicked, _) = cb.on_click(Point::new(50, 15));
        assert!(!clicked);
        assert!(!cb.checked);
    }

    #[test]
    fn test_checkbox_get_id() {
        let cb = make_checkbox(0, 0, 100, 30);
        assert_eq!(cb.get_id(), "test_cb");
    }

    #[test]
    fn test_checkbox_change_label() {
        let mut cb = make_checkbox(0, 0, 100, 30);
        cb.change_label("New Label".to_string());
        assert_eq!(cb.label, "New Label");
    }

    #[test]
    fn test_checkbox_change_location() {
        let mut cb = make_checkbox(0, 0, 100, 30);
        cb.change_location(Point::new(20, 40));
        assert_eq!(cb.get_location(), Point::new(20, 40));
    }

    #[test]
    fn test_checkbox_interface_traits() {
        let cb = make_checkbox(0, 0, 100, 30);
        assert!(!cb.is_static());
        assert!(!cb.has_indent());
        assert_eq!(cb.draw_priority(), 1);
        assert!(!cb.dirty_parent());
        assert!(!cb.important_component_clicked());
        assert!(!cb.deactivate_parent());
        assert!(cb.after_click());
    }

    // ==================== Slider tests ====================

    #[test]
    fn test_slider_initial_value() {
        let slider = make_slider(0, 0, 200, 40, 100);
        assert_eq!(slider.value, 0);
    }

    #[test]
    fn test_slider_horizontal_value_at_start() {
        let mut slider = make_slider(0, 0, 200, 40, 100);
        slider.change_slider_value_horizontal(Point::new(0, 20));
        assert_eq!(slider.value, 0);
    }

    #[test]
    fn test_slider_horizontal_value_at_end() {
        let mut slider = make_slider(0, 0, 200, 40, 100);
        slider.change_slider_value_horizontal(Point::new(200, 20));
        assert_eq!(slider.value, 100);
    }

    #[test]
    fn test_slider_horizontal_value_midpoint() {
        let mut slider = make_slider(0, 0, 200, 40, 100);
        slider.change_slider_value_horizontal(Point::new(100, 20));
        assert!(
            slider.value >= 40 && slider.value <= 60,
            "Midpoint value {} should be near 50",
            slider.value
        );
    }

    #[test]
    fn test_slider_vertical_value_at_start() {
        let mut slider = make_slider(0, 0, 20, 200, 100);
        slider.is_vertical = true;
        slider.change_slider_value_vertical(Point::new(10, 0));
        assert_eq!(slider.value, 0);
    }

    #[test]
    fn test_slider_vertical_value_at_end() {
        let mut slider = make_slider(0, 0, 20, 200, 100);
        slider.is_vertical = true;
        slider.change_slider_value_vertical(Point::new(10, 200));
        assert_eq!(slider.value, 100);
    }

    #[test]
    fn test_slider_on_click_inside() {
        let mut slider = make_slider(0, 0, 200, 40, 100);
        let (clicked, id) = slider.on_click(Point::new(100, 20));
        assert!(clicked);
        assert_eq!(id, Some("test_slider".to_string()));
    }

    #[test]
    fn test_slider_on_click_outside() {
        let mut slider = make_slider(0, 0, 200, 40, 100);
        let (clicked, _) = slider.on_click(Point::new(300, 300));
        assert!(!clicked);
    }

    #[test]
    fn test_slider_on_click_inactive() {
        let mut slider = make_slider(0, 0, 200, 40, 100);
        slider.change_active(false);
        let (clicked, _) = slider.on_click(Point::new(100, 20));
        assert!(!clicked);
    }

    #[test]
    fn test_slider_change_label() {
        let mut slider = make_slider(0, 0, 200, 40, 100);
        slider.change_label("New Label".to_string());
        assert_eq!(slider.text, "New Label");
    }

    #[test]
    fn test_slider_interface_traits() {
        let slider = make_slider(0, 0, 200, 40, 100);
        assert!(!slider.is_static());
        assert!(!slider.has_indent());
        assert_eq!(slider.draw_priority(), 1);
        assert!(!slider.dirty_parent());
        assert!(!slider.important_component_clicked());
        assert!(!slider.deactivate_parent());
        assert!(!slider.after_click()); // Slider processes on mouse-down
    }

    #[test]
    fn test_slider_value_clamped_to_range() {
        let mut slider = make_slider(0, 0, 200, 40, 50);
        slider.change_slider_value_horizontal(Point::new(500, 20));
        assert!(
            slider.value <= 50,
            "Value {} should not exceed range 50",
            slider.value
        );
    }

    // ==================== Dropdown tests ====================

    #[test]
    fn test_dropdown_get_id() {
        let dd = make_dropdown(0, 0, 200, 30);
        assert_eq!(dd.get_id(), "test_dd");
    }

    #[test]
    fn test_dropdown_click_toggles_open() {
        let mut dd = make_dropdown(0, 0, 200, 30);
        assert!(!dd.clicked_on);
        dd.on_click(Point::new(100, 15));
        assert!(dd.clicked_on);
        dd.on_click(Point::new(100, 15));
        assert!(!dd.clicked_on);
    }

    #[test]
    fn test_dropdown_click_outside_when_closed() {
        let mut dd = make_dropdown(0, 0, 200, 30);
        let (clicked, _) = dd.on_click(Point::new(300, 300));
        assert!(!clicked);
        assert!(!dd.clicked_on);
    }

    #[test]
    fn test_dropdown_interface_traits() {
        let dd = make_dropdown(0, 0, 200, 30);
        assert!(dd.is_static());
        assert!(!dd.has_indent());
        assert_eq!(dd.draw_priority(), 2);
        assert!(dd.dirty_parent());
        assert!(dd.deactivate_parent());
        assert!(dd.after_click());
    }

    #[test]
    fn test_dropdown_contains_no_filter() {
        let dd = make_dropdown(0, 0, 200, 30);
        assert!(dd.contains(None));
    }

    #[test]
    fn test_dropdown_contains_matching_self() {
        let mut dd = make_dropdown(0, 0, 200, 30);
        dd.id = "Path_Selector".to_string();
        assert!(dd.contains(Some("Path")));
    }

    #[test]
    fn test_dropdown_contains_matching_child() {
        let dd = make_dropdown(0, 0, 200, 30);
        assert!(dd.contains(Some("Option B")));
    }

    #[test]
    fn test_dropdown_change_active_cascades() {
        let mut dd = make_dropdown(0, 0, 200, 30);
        dd.change_active(false);
        assert!(!dd.is_active());
        for opt in dd.options.borrow().iter() {
            assert!(!opt.is_active());
        }
    }

    #[test]
    fn test_dropdown_change_label() {
        let mut dd = make_dropdown(0, 0, 200, 30);
        dd.change_label("New Selection".to_string());
        assert_eq!(dd.text, "New Selection");
    }

    // ==================== OptionButton tests ====================

    fn make_option_button() -> OptionButton {
        OptionButton::new(
            30,
            200,
            Point::new(0, 0),
            "test_option".to_string(),
            true,
            vec![
                (
                    "Alpha".to_string(),
                    InterfaceStyle {
                        text_color: BLACK,
                        background_color: Color::RGB(255, 0, 0),
                    },
                ),
                (
                    "Beta".to_string(),
                    InterfaceStyle {
                        text_color: BLACK,
                        background_color: Color::RGB(0, 255, 0),
                    },
                ),
                (
                    "Gamma".to_string(),
                    InterfaceStyle {
                        text_color: BLACK,
                        background_color: Color::RGB(0, 0, 255),
                    },
                ),
            ],
            false,
        )
    }

    #[test]
    fn test_option_button_creation() {
        let ob = make_option_button();
        assert_eq!(ob.get_id(), "test_option");
        assert_eq!(ob.options.borrow().len(), 3);
    }

    #[test]
    fn test_option_button_click_selects_option() {
        let mut ob = make_option_button();
        let (clicked, selected) = ob.on_click(Point::new(30, 15));
        assert!(clicked);
        assert_eq!(selected, Some("Alpha".to_string()));
    }

    #[test]
    fn test_option_button_click_second_option() {
        let mut ob = make_option_button();
        let (clicked, selected) = ob.on_click(Point::new(90, 15));
        assert!(clicked);
        assert_eq!(selected, Some("Beta".to_string()));
    }

    #[test]
    fn test_option_button_click_outside() {
        let mut ob = make_option_button();
        let (clicked, _) = ob.on_click(Point::new(300, 300));
        assert!(!clicked);
    }

    #[test]
    fn test_option_button_change_width_redistributes() {
        let mut ob = make_option_button();
        ob.change_width(300);
        assert_eq!(ob.get_width(), 300);
        for (_, btn) in ob.options.borrow().iter() {
            assert_eq!(btn.get_width(), 100);
        }
    }

    #[test]
    fn test_option_button_change_height_cascades() {
        let mut ob = make_option_button();
        ob.change_height(50);
        assert_eq!(ob.get_height(), 50);
        for (_, btn) in ob.options.borrow().iter() {
            assert_eq!(btn.get_height(), 50);
        }
    }

    #[test]
    fn test_option_button_change_location_cascades() {
        let mut ob = make_option_button();
        ob.change_location(Point::new(100, 200));
        assert_eq!(ob.get_location(), Point::new(100, 200));
        let opts = ob.options.borrow();
        let btn_width = opts[0].1.width as i32;
        for (i, (_, btn)) in opts.iter().enumerate() {
            assert_eq!(btn.get_location().x(), 100 + i as i32 * btn_width);
            assert_eq!(btn.get_location().y(), 200);
        }
    }

    #[test]
    fn test_option_button_interface_traits() {
        let ob = make_option_button();
        assert!(!ob.is_static());
        assert!(!ob.has_indent());
        assert_eq!(ob.draw_priority(), 1);
        assert!(!ob.dirty_parent());
        assert!(!ob.important_component_clicked());
        assert!(!ob.deactivate_parent());
        assert!(ob.after_click());
    }

    // Additional slider tests for change_slider_value method

    #[test]
    fn test_slider_change_slider_value_horizontal() {
        let mut slider = make_slider(0, 0, 200, 40, 100);
        slider.change_slider_value(Point::new(50, 20));
        assert!(slider.value > 0);
    }

    #[test]
    fn test_slider_change_slider_value_vertical() {
        let mut slider = make_slider(0, 0, 20, 200, 100);
        slider.is_vertical = true;
        slider.change_slider_value(Point::new(10, 50));
        assert!(slider.value > 0);
    }

    #[test]
    fn test_slider_get_slider_width() {
        let slider = make_slider(0, 0, 200, 40, 100);
        let width = slider.get_slider_width();
        assert!(width > 0);
        assert!(width <= 40);
    }

    #[test]
    fn test_slider_get_rect() {
        let slider = make_slider(10, 20, 200, 40, 100);
        let rect = slider.get_rect(Point::new(10, 20));
        assert_eq!(rect.x(), 10);
        assert_eq!(rect.y(), 20);
        assert_eq!(rect.width(), 200);
        assert_eq!(rect.height(), 40);
    }

    #[test]
    fn test_slider_mouse_over() {
        let slider = make_slider(10, 20, 200, 40, 100);
        assert!(slider.mouse_over_component(Point::new(100, 40)));
        assert!(!slider.mouse_over_component(Point::new(500, 500)));
    }

    #[test]
    fn test_slider_mouse_over_inactive() {
        let mut slider = make_slider(10, 20, 200, 40, 100);
        slider.active = false;
        assert!(!slider.mouse_over_component(Point::new(100, 40)));
    }

    // Additional dropdown tests

    #[test]
    fn test_dropdown_contains_no_matching_filter() {
        let mut dd = make_dropdown(0, 0, 200, 30);
        dd.id = "Path_Selector".to_string();
        assert!(!dd.contains(Some("Download")));
    }

    #[test]
    fn test_dropdown_mouse_over() {
        let dd = make_dropdown(10, 20, 200, 30);
        assert!(dd.mouse_over_component(Point::new(100, 35)));
        assert!(!dd.mouse_over_component(Point::new(500, 500)));
    }

    #[test]
    fn test_dropdown_mouse_over_inactive() {
        let mut dd = make_dropdown(10, 20, 200, 30);
        dd.active = false;
        assert!(!dd.mouse_over_component(Point::new(100, 35)));
    }

    // Additional checkbox tests

    #[test]
    fn test_checkbox_mouse_over() {
        let cb = make_checkbox(10, 20, 100, 30);
        assert!(cb.mouse_over_component(Point::new(50, 35)));
        assert!(!cb.mouse_over_component(Point::new(500, 500)));
    }

    #[test]
    fn test_checkbox_mouse_over_inactive() {
        let mut cb = make_checkbox(10, 20, 100, 30);
        cb.active = false;
        assert!(!cb.mouse_over_component(Point::new(50, 35)));
    }

    #[test]
    fn test_checkbox_drawn_state() {
        let cb = make_checkbox(0, 0, 100, 30);
        assert!(!cb.is_drawn());
        cb.change_drawn(true);
        assert!(cb.is_drawn());
    }

    #[test]
    fn test_checkbox_get_rect() {
        let cb = make_checkbox(10, 20, 100, 30);
        let rect = cb.get_rect(Point::new(10, 20));
        assert_eq!(rect.x(), 10);
        assert_eq!(rect.y(), 20);
        assert_eq!(rect.width(), 100);
        assert_eq!(rect.height(), 30);
    }

    #[test]
    fn test_checkbox_change_dimensions() {
        let mut cb = make_checkbox(0, 0, 100, 30);
        cb.change_width(200);
        cb.change_height(50);
        assert_eq!(cb.get_width(), 200);
        assert_eq!(cb.get_height(), 50);
    }

    #[test]
    fn test_checkbox_change_active() {
        let mut cb = make_checkbox(0, 0, 100, 30);
        assert!(cb.is_active());
        cb.change_active(false);
        assert!(!cb.is_active());
    }

    // Additional standard button tests

    #[test]
    fn test_standard_button_mouse_over_at_boundary() {
        let btn = make_standard_button(10, 10, 100, 50);
        assert!(btn.mouse_over_component(Point::new(10, 10)));
        assert!(btn.mouse_over_component(Point::new(109, 59)));
        assert!(!btn.mouse_over_component(Point::new(110, 60)));
    }

    #[test]
    fn test_standard_button_get_location() {
        let btn = make_standard_button(10, 20, 100, 50);
        assert_eq!(btn.get_location(), Point::new(10, 20));
    }

    #[test]
    fn test_standard_button_get_height() {
        let btn = make_standard_button(0, 0, 100, 50);
        assert_eq!(btn.get_height(), 50);
    }

    #[test]
    fn test_standard_button_change_drawn_cascades() {
        let btn = make_standard_button(0, 0, 100, 50);
        assert!(!btn.is_drawn());
    }

    // Additional option button tests

    #[test]
    fn test_option_button_get_location() {
        let ob = make_option_button();
        assert_eq!(ob.get_location(), Point::new(0, 0));
    }

    #[test]
    fn test_option_button_get_height() {
        let ob = make_option_button();
        assert_eq!(ob.get_height(), 30);
    }

    #[test]
    fn test_option_button_mouse_over() {
        let ob = make_option_button();
        assert!(ob.mouse_over_component(Point::new(30, 15)));
        assert!(!ob.mouse_over_component(Point::new(500, 500)));
    }

    #[test]
    fn test_option_button_mouse_over_inactive() {
        let mut ob = make_option_button();
        ob.active = false;
        assert!(!ob.mouse_over_component(Point::new(30, 15)));
    }

    #[test]
    fn test_option_button_change_active() {
        let mut ob = make_option_button();
        assert!(ob.is_active());
        ob.change_active(false);
        assert!(!ob.is_active());
    }

    #[test]
    fn test_option_button_drawn_state() {
        let ob = make_option_button();
        assert!(!ob.is_drawn());
        ob.change_drawn(true);
        assert!(ob.is_drawn());
    }

    #[test]
    fn test_option_button_get_rect() {
        let ob = make_option_button();
        let rect = ob.get_rect(Point::new(0, 0));
        assert_eq!(rect.x(), 0);
        assert_eq!(rect.y(), 0);
        assert_eq!(rect.width(), 200);
        assert_eq!(rect.height(), 30);
    }
}
