//! # Color Constants Module
//!
//! This module defines all color constants used throughout the application's UI.
//! Colors are defined as SDL2 `Color` structs for consistent styling.

use sdl2::pixels::Color;

/// Pure red color - used for enemy/goal tiles
pub const RED: Color = Color::RGB(255, 0, 0);

/// Pure green color - used for player/start tiles
pub const GREEN: Color = Color::RGB(0, 255, 0);

/// Pure black color - used for obstacles and outlines
pub const BLACK: Color = Color::RGB(0, 0, 0);

/// Pure white color - used for floor tiles and highlighted buttons
pub const WHITE: Color = Color::RGB(255, 255, 255);

/// Yellow color - available for special highlighting
pub const YELLOW: Color = Color::RGB(255, 255, 0);

/// Primary UI color (dark gray) - used for main button backgrounds
pub const PRIMARY_COLOR: Color = Color::RGB(84, 84, 84);

/// Secondary UI color (medium gray) - used for widget backgrounds
pub const SECONDARY_COLOR: Color = Color::RGB(150, 150, 150);

/// Tertiary UI color (light gray) - used for subtle UI elements
pub const TERTIARY_COLOR: Color = Color::RGB(156, 156, 156);

/// Quaternary UI color (darker gray) - used for file explorer items
pub const QUATERNARY_COLOR: Color = Color::RGB(100, 100, 100);

/// Hover state color (light gray) - used when mouse is over interactive elements
pub const HOVER_COLOR: Color = Color::RGB(200, 200, 200);
