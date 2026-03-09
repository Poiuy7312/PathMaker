//! # Game Settings Module
//!
//! This module handles application configuration including:
//! - Display settings (window size, fullscreen)
//! - Gameplay options (algorithms, generation modes)
//! - Board dimensions and parameters
//! - File paths for saving/loading
//!
//! Settings are persisted to JSON files and loaded on startup.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Specifies the method used to generate the game board.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GenerationMode {
    /// Random placement of obstacles and weighted tiles
    Random,
    /// City-style generation with roads and buildings
    City,
}

/// Complete application settings with serialization support.
///
/// All settings can be persisted to a JSON file and loaded on startup.
/// Default values are provided for first-time users.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    // ----- Display Settings -----
    /// Width of the application window in pixels
    pub window_width: u32,
    /// Height of the application window in pixels
    pub window_height: u32,
    /// Whether to run in fullscreen mode
    pub fullscreen: bool,

    // ----- Gameplay Settings -----
    /// Enable dynamic grid regeneration between iterations
    pub enable_dynamic_generation: bool,
    /// Enable doubling experiment (double obstacles each iteration)
    pub enable_doubling_experiment: bool,
    /// Allow multiple agents to pathfind simultaneously
    pub enable_multiple_agents: bool,
    /// Allow multiple goal positions
    pub enable_multiple_goals: bool,

    pub enable_random_agents: bool,
    /// Currently selected pathfinding algorithm ("Greedy", "BFS", "A* search", "JPSW")
    pub selected_algorithm: String,

    // ----- Board Settings -----
    /// Width of the game board in pixels
    pub board_width: u32,
    /// Height of the game board in pixels
    pub board_height: u32,
    /// Number of tiles in the X direction
    pub tiles_x: u32,
    /// Number of tiles in the Y direction
    pub tiles_y: u32,
    /// Maximum weight value for weighted tiles (1-255)
    pub weight: u8,
    /// Percentage/count of obstacles to generate
    pub gen_obstacles: u32,
    /// Percentage/count of weighted tiles to generate
    pub weight_count: u32,
    /// Number of pathfinding iterations to run
    pub iterations: usize,
    /// Default filename for saving maps
    pub save_file: String,
    /// Current grid generation mode
    pub gen_mode: GenerationMode,

    // ----- File Settings -----
    /// Path to the last opened map file
    pub last_opened_file: Option<String>,
    /// Default directory for saving maps
    pub last_save_directory: String,
    /// Enable automatic saving
    pub auto_save_enabled: bool,
}

/// Provides sensible default values for all settings.
///
/// Default configuration creates a reasonable starting point:
/// - 1500x1024 window
/// - 512x512 tile grid
/// - Greedy algorithm selected
/// - Random generation mode
impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            window_width: 1200,
            window_height: 800,
            fullscreen: false,
            enable_dynamic_generation: false,
            enable_doubling_experiment: false,
            enable_multiple_agents: false,
            enable_multiple_goals: false,
            enable_random_agents: false,
            selected_algorithm: String::from("Greedy"),
            board_width: 800,
            board_height: 800,
            tiles_x: 40,
            tiles_y: 40,
            last_opened_file: None,
            last_save_directory: String::from("/home"),
            auto_save_enabled: false,
            save_file: "test".to_string(),
            gen_mode: GenerationMode::Random,
            weight: 1,
            gen_obstacles: 0,
            weight_count: 0,
            iterations: 1,
        }
    }
}

impl GameSettings {
    /// Load settings from a JSON file.
    ///
    /// If the file doesn't exist, returns default settings.
    ///
    /// # Arguments
    /// * `path` - Path to the settings JSON file
    ///
    /// # Returns
    /// Loaded settings or an error
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if Path::new(path).exists() {
            let data = fs::read_to_string(path)?;
            let settings = serde_json::from_str(&data)?;
            Ok(settings)
        } else {
            Ok(GameSettings::default())
        }
    }

    /// Save current settings to a JSON file.
    ///
    /// Creates a pretty-printed JSON file for human readability.
    ///
    /// # Arguments
    /// * `path` - Path where settings should be saved
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Get the default path for the settings file.
    ///
    /// Platform-specific locations:
    /// - Linux: `~/.config/pathmaker/settings.json`
    /// - Windows: `%APPDATA%\pathmaker\settings.json`
    /// Falls back to `./settings.json` if no home directory is found.
    pub fn get_default_path() -> String {
        if cfg!(target_os = "windows") {
            if let Ok(appdata) = std::env::var("APPDATA") {
                format!("{}\\pathmaker\\settings.json", appdata)
            } else if let Ok(profile) = std::env::var("USERPROFILE") {
                format!("{}\\AppData\\Roaming\\pathmaker\\settings.json", profile)
            } else {
                String::from(".\\settings.json")
            }
        } else if let Ok(home) = std::env::var("HOME") {
            format!("{}/.config/pathmaker/settings.json", home)
        } else {
            String::from("./settings.json")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------- Default -------

    #[test]
    fn test_default_window_dimensions() {
        let s = GameSettings::default();
        assert_eq!(s.window_width, 1200);
        assert_eq!(s.window_height, 800);
    }

    #[test]
    fn test_default_fullscreen_off() {
        let s = GameSettings::default();
        assert!(!s.fullscreen);
    }

    #[test]
    fn test_default_algorithm_is_greedy() {
        let s = GameSettings::default();
        assert_eq!(s.selected_algorithm, "Greedy");
    }

    #[test]
    fn test_default_board_dimensions() {
        let s = GameSettings::default();
        assert_eq!(s.board_width, 800);
        assert_eq!(s.board_height, 800);
        assert_eq!(s.tiles_x, 40);
        assert_eq!(s.tiles_y, 40);
    }

    #[test]
    fn test_default_iterations() {
        let s = GameSettings::default();
        assert_eq!(s.iterations, 1);
    }

    #[test]
    fn test_default_flags_all_false() {
        let s = GameSettings::default();
        assert!(!s.enable_dynamic_generation);
        assert!(!s.enable_doubling_experiment);
        assert!(!s.enable_multiple_agents);
        assert!(!s.enable_multiple_goals);
        assert!(!s.enable_random_agents);
        assert!(!s.auto_save_enabled);
    }

    #[test]
    fn test_default_gen_mode_is_random() {
        let s = GameSettings::default();
        assert!(matches!(s.gen_mode, GenerationMode::Random));
    }

    #[test]
    fn test_default_weight_and_obstacles() {
        let s = GameSettings::default();
        assert_eq!(s.weight, 1);
        assert_eq!(s.gen_obstacles, 0);
        assert_eq!(s.weight_count, 0);
    }

    #[test]
    fn test_default_last_opened_file_is_none() {
        let s = GameSettings::default();
        assert!(s.last_opened_file.is_none());
    }

    // ------- Serialization roundtrip -------

    #[test]
    fn test_serialization_roundtrip() {
        let original = GameSettings::default();
        let json = serde_json::to_string_pretty(&original).unwrap();
        let loaded: GameSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.window_width, original.window_width);
        assert_eq!(loaded.window_height, original.window_height);
        assert_eq!(loaded.fullscreen, original.fullscreen);
        assert_eq!(loaded.selected_algorithm, original.selected_algorithm);
        assert_eq!(loaded.tiles_x, original.tiles_x);
        assert_eq!(loaded.tiles_y, original.tiles_y);
        assert_eq!(loaded.iterations, original.iterations);
        assert_eq!(loaded.weight, original.weight);
    }

    #[test]
    fn test_serialization_with_custom_values() {
        let mut s = GameSettings::default();
        s.window_width = 1920;
        s.window_height = 1080;
        s.fullscreen = true;
        s.selected_algorithm = "A* search".to_string();
        s.tiles_x = 100;
        s.tiles_y = 100;
        s.iterations = 50;
        s.weight = 128;
        s.enable_multiple_agents = true;

        let json = serde_json::to_string(&s).unwrap();
        let loaded: GameSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.window_width, 1920);
        assert_eq!(loaded.fullscreen, true);
        assert_eq!(loaded.selected_algorithm, "A* search");
        assert_eq!(loaded.tiles_x, 100);
        assert_eq!(loaded.iterations, 50);
        assert!(loaded.enable_multiple_agents);
    }

    // ------- Save and Load -------

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("pathmaker_test_settings.json");
        let path_str = path.to_str().unwrap();

        let mut original = GameSettings::default();
        original.window_width = 1600;
        original.selected_algorithm = "BFS".to_string();
        original.save(path_str).unwrap();

        let loaded = GameSettings::load(path_str).unwrap();
        assert_eq!(loaded.window_width, 1600);
        assert_eq!(loaded.selected_algorithm, "BFS");

        // Cleanup
        let _ = std::fs::remove_file(path_str);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let result = GameSettings::load("/tmp/this_file_definitely_does_not_exist_abc.json");
        assert!(result.is_ok());
        let s = result.unwrap();
        assert_eq!(s.window_width, 1200); // default
    }

    // ------- get_default_path -------

    #[test]
    fn test_get_default_path_contains_pathmaker() {
        let path = GameSettings::get_default_path();
        assert!(path.contains("pathmaker"));
        assert!(path.ends_with("settings.json"));
    }
}
