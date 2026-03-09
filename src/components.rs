//! # Component System Module
//!
//! This module defines the core component trait and re-exports all UI component modules.
//! Components are the building blocks of the application's user interface.
//!
//! ## Module Structure
//! - `board`: Game board for pathfinding visualization
//! - `button`: Various button types (Standard, Dropdown, Checkbox, etc.)
//! - `file_explorer`: Directory tree navigation component
//! - `inputbox`: Text input field component
//! - `widget`: Container for grouping and laying out multiple interface components

use sdl2::rect::Point;

/// Board component for the pathfinding grid
pub mod board;
/// Button components including StandardButton, Dropdown, CheckBox, OptionButton, and Slider
pub mod button;
/// File system navigation component for loading/saving maps
pub mod file_explorer;
/// Text input component for user text entry
pub mod inputbox;
/// Widget container for organizing interface components in grid layouts
pub mod widget;
//pub mod selectbox;

/// Core trait that all interactive UI components must implement.
///
/// This trait provides a common interface for handling user interactions,
/// positioning, sizing, and state management of UI components.
pub trait Component {
    /// Handle a mouse click event at the given position.
    ///
    /// # Arguments
    /// * `mouse_state` - The position of the mouse click
    ///
    /// # Returns
    /// A tuple where:
    /// - First element: `true` if the component was clicked
    /// - Second element: Optional string identifier of the clicked element
    fn on_click(&mut self, mouse_state: Point) -> (bool, Option<String>);

    /// Get the unique identifier of this component.
    fn get_id(&self) -> String;

    /// Update the component's position on screen.
    fn change_location(&mut self, new_location: Point);

    /// Set whether the component is active (can receive input).
    fn change_active(&mut self, new_value: bool);

    /// Check if the component is currently active.
    fn is_active(&self) -> bool;

    /// Get the component's current position.
    fn get_location(&self) -> Point;

    /// Update the component's width.
    fn change_width(&mut self, new_width: u32);

    /// Get the component's current width.
    fn get_width(&self) -> u32;

    /// Get the component's current height.
    fn get_height(&self) -> u32;

    /// Update the component's height.
    fn change_height(&mut self, new_height: u32);

    /// Check if the mouse is currently over this component.
    ///
    /// # Arguments
    /// * `mouse_position` - Current position of the mouse cursor
    fn mouse_over_component(&self, mouse_position: Point) -> bool;
}
