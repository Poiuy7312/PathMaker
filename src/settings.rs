use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    // Display settings
    pub window_width: u32,
    pub window_height: u32,
    pub fullscreen: bool,

    // Gameplay settings
    pub enable_dynamic_generation: bool,
    pub enable_doubling_experiment: bool,
    pub enable_multiple_agents: bool,
    pub enable_multiple_goals: bool,
    pub selected_algorithm: String, // "BFS" or "A*"

    // Board settings
    pub board_width: u32,
    pub board_height: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub weight: u8,

    // File settings
    pub last_opened_file: Option<String>,
    pub last_save_directory: String,
    pub auto_save_enabled: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            window_width: 1000,
            window_height: 800,
            fullscreen: false,
            enable_dynamic_generation: false,
            enable_doubling_experiment: false,
            enable_multiple_agents: false,
            enable_multiple_goals: false,
            selected_algorithm: String::from("Greedy"),
            board_width: 800,
            board_height: 800,
            tiles_x: 40,
            tiles_y: 40,
            last_opened_file: None,
            last_save_directory: String::from("/home"),
            auto_save_enabled: false,
            weight: 1,
        }
    }
}

impl GameSettings {
    /// Load settings from JSON file
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if Path::new(path).exists() {
            let data = fs::read_to_string(path)?;
            let settings = serde_json::from_str(&data)?;
            Ok(settings)
        } else {
            Ok(GameSettings::default())
        }
    }

    /// Save settings to JSON file
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Get settings file path (in user config directory)
    pub fn get_default_path() -> String {
        if let Ok(home) = std::env::var("HOME") {
            format!("{}/.config/pathmaker/settings.json", home)
        } else {
            String::from("./settings.json")
        }
    }
}
