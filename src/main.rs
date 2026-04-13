//! # PathMaker - Interactive Pathfinding Visualization Application
//!
//! This is the main entry point for the PathMaker application, a graphical tool
//! for visualizing and benchmarking various pathfinding algorithms on customizable grids.
//!
//! ## Features
//! - Interactive grid-based board for placing obstacles, weighted tiles, and agents
//! - Multiple pathfinding algorithms: Greedy Search, BFS, A*, and JPS with Weights (JPSW)
//! - Map generation modes: Random and City-style procedural generation
//! - File save/load functionality for persisting board configurations
//! - Performance benchmarking with memory and timing metrics
//!
//! ## Architecture
//! The application uses SDL2 for rendering and event handling, with a component-based
//! UI system built on top of it. The main loop processes user input, updates the UI,
//! and renders the board and control widgets.

extern crate sdl2;

// Memory allocation and statistics tracking
#[cfg(not(target_os = "windows"))]
use jemalloc_ctl::{epoch, stats};

// Cross-platform memory tracking allocator (used on Windows; jemalloc used on Unix)
use sdl2::image::LoadSurface;
#[cfg(target_os = "windows")]
#[global_allocator]
static ALLOC: cap::Cap<std::alloc::System> = cap::Cap::new(std::alloc::System, usize::MAX);

// SDL2 imports for graphics, events, and text rendering
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::surface::Surface;
use sdl2::ttf;

// Standard library imports for data structures and concurrency
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, fs};

/// Global allocator using jemalloc for improved memory allocation performance
/// and accurate memory usage tracking during pathfinding benchmarks.
#[cfg(not(target_os = "windows"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

// Application modules
/// Benchmarking utilities for measuring pathfinding performance
mod benchmarks;
/// Color constants used throughout the UI
mod colors;
/// UI component system (buttons, widgets, board, etc.)
mod components;
/// File dialog utilities for loading and saving maps
mod fileDialog;
/// Pathfinding algorithms implementation (Greedy, BFS, A*, JPSW)
mod pathfinding;
/// Application settings and configuration persistence
mod settings;
/// Utility functions for UI calculations and file operations
mod util;

use crate::colors::*;

// Embed assets directly into the binary so it works when installed anywhere
const FONT_BYTES: &[u8] = include_bytes!("assets/open-sans/OpenSans-Semibold.ttf");
const ICON_BYTES: &[u8] = include_bytes!("assets/Icon.svg");

/// Returns the application data directory and ensures embedded assets are extracted there.
///
/// Platform-specific locations:
/// - Linux: `~/.local/share/game_ex/`
/// - Windows: `%LOCALAPPDATA%\game_ex\`
fn ensure_assets() -> PathBuf {
    let data_dir = if cfg!(target_os = "windows") {
        let appdata = env::var("LOCALAPPDATA")
            .or_else(|_| env::var("APPDATA"))
            .or_else(|_| env::var("USERPROFILE").map(|h| format!("{}\\AppData\\Local", h)))
            .expect("Could not determine application data directory");
        PathBuf::from(appdata).join("game_ex")
    } else {
        let home = env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home).join(".local/share/game_ex")
    };
    let font_dir = data_dir.join("fonts");

    fs::create_dir_all(&font_dir).expect("Failed to create data directory");

    let font_path = font_dir.join("OpenSans-Semibold.ttf");
    if !font_path.exists() {
        fs::write(&font_path, FONT_BYTES).expect("Failed to write font file");
    }
    let icon_path = data_dir.join("Icon.svg");
    fs::write(&icon_path, ICON_BYTES).expect("Failed to write icon file");

    data_dir
}

use crate::components::displaybox::DisplayBox;
use crate::components::file_explorer::FileExplorer;
use crate::components::{board::*, button::*, inputbox::*, widget::*, Component};
use crate::settings::GameSettings;

/// Main entry point for the PathMaker application.
///
/// This function initializes the SDL2 context, loads application settings,
/// creates the game board and UI widgets, and runs the main event loop.
///
/// # Event Loop
/// The main loop handles:
/// - Mouse input for interacting with the board and UI components
/// - Keyboard input for text entry (file names, search)
/// - Window resize events for responsive layout
/// - Pathfinding execution and visualization
///
/// # UI Components
/// - **Board Control Widget**: Contains buttons for algorithm selection, generation options, etc.
/// - **File Select Widget**: File browser for loading saved maps
/// - **Save Widget**: File browser with name input for saving maps
pub fn main() {
    // Load settings at startup
    let settings_path = GameSettings::get_default_path();
    let mut settings =
        GameSettings::load(&settings_path).unwrap_or_else(|_| GameSettings::default());

    // Use settings values
    let board_width: u32 = settings.board_width;
    let board_height: u32 = settings.board_height;
    let window_width: u32 = settings.window_width;
    let window_height: u32 = settings.window_height;
    let debug_height = window_height / 4;
    let control_width = window_width - board_width;
    let control_height = window_height;
    let tiles_x: u32 = settings.tiles_x; // Replace with settings.tiles_x
    let tiles_y: u32 = settings.tiles_y;
    let sdl_context = sdl2::init().unwrap();

    let video_subsystem = sdl_context.video().unwrap();
    //let display_mode = video_subsystem.current_display_mode(0).unwrap();
    //let window_width = display_mode.w as u32;
    //let window_height = display_mode.h as u32;
    video_subsystem.text_input().stop();
    let mut window = video_subsystem
        .window("PathMaker demo", window_width, window_height)
        .position_centered()
        .build()
        .expect("Failed to render Window");
    //window
    //  .set_fullscreen(sdl2::video::FullscreenType::True)
    //   .unwrap();
    let data_dir = ensure_assets();
    let icon_path = data_dir.join("Icon.svg");
    let window_icon = Surface::from_file(&icon_path).unwrap();
    window.set_icon(window_icon);
    let font_path = data_dir.join("fonts/OpenSans-Semibold.ttf");
    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .unwrap();

    let texture_creator = canvas.texture_creator();
    let directory_tree = fileDialog::get_file_tree();
    let mut select_file: bool = false; // Check if select file widget is active
    let mut save_file: bool = false; // Check if save file widget is active
    let mut change_gen_sliders = false;
    let mut display_visual_path_result = false;
    let mut results: String = String::new();

    let ttf_context: ttf::Sdl2TtfContext = ttf::init().unwrap();

    let mut font: ttf::Font<'_, 'static> = ttf_context
        .load_font(font_path, 124)
        .expect("Unable to Load Font");

    /*----- File Explorer Components ----- */

    let mut mouse_clicked_on: bool = false;
    let mut replacement_labels: Vec<&str> = Vec::with_capacity(3);

    let controls_width = window_width * 1 / 5;

    let mut run_game_board = false;
    //= Rect::new(998, 0, 1000, 1000);

    let directories: Rc<RefCell<HashMap<String, (StandardButton, Vec<String>)>>> = Rc::new(
        RefCell::new(util::get_dir_map(&directory_tree, window_width)),
    );

    let generation_mode_selector: Box<dyn Interface> = {
        Box::new(Dropdown {
            height: 30,
            width: controls_width,
            location: Point::new(0, 0),
            text_color: WHITE,
            background_color: PRIMARY_COLOR,
            hover: RefCell::new(false),
            text: "Random Generation".to_string(),
            id: "Gen_Mode_Selector".to_string(),
            active: false,
            clicked_on: false,
            options: RefCell::from(vec![StandardButton {
                height: 0,
                width: 0,
                location: Point::new(0, 0),
                text_color: WHITE,
                background_color: PRIMARY_COLOR,
                hover: RefCell::new(false),
                text: "City Generation".to_string(),
                id: "City Generation".to_string(),
                filter: None,
                active: false,
                drawn: RefCell::new(false),
                cached_texture: None,
            }]),
            filter: None,
            drawn: RefCell::new(false),
        })
    };

    let path_selector: Box<dyn Interface> = {
        Box::new(Dropdown {
            height: 30,
            width: controls_width,
            location: Point::new(0, 0),
            text_color: WHITE,
            background_color: PRIMARY_COLOR,
            hover: RefCell::new(false),
            text: "Greedy Search".to_string(),
            id: "Path_Selector".to_string(),
            active: false,
            clicked_on: false,
            options: RefCell::from(vec![
                StandardButton {
                    height: 0,
                    width: 0,
                    location: Point::new(0, 0),
                    text_color: WHITE,
                    background_color: PRIMARY_COLOR,
                    hover: RefCell::new(false),
                    text: "Breadth First Search".to_string(),
                    id: "Breadth First Search".to_string(),
                    filter: None,
                    active: false,
                    drawn: RefCell::new(false),
                    cached_texture: None,
                },
                StandardButton {
                    height: 0,
                    width: 0,
                    location: Point::new(0, 0),
                    text_color: WHITE,
                    background_color: PRIMARY_COLOR,
                    hover: RefCell::new(false),
                    text: "A* search".to_string(),
                    id: "A* search".to_string(),
                    filter: None,
                    active: false,
                    drawn: RefCell::new(false),
                    cached_texture: None,
                },
                StandardButton {
                    height: 0,
                    width: 0,
                    location: Point::new(0, 0),
                    text_color: WHITE,
                    background_color: PRIMARY_COLOR,
                    hover: RefCell::new(false),
                    text: "JPSW".to_string(),
                    id: "JPSW".to_string(),
                    filter: None,
                    active: false,
                    drawn: RefCell::new(false),
                    cached_texture: None,
                },
            ]),
            filter: None,
            drawn: RefCell::new(false),
        })
    };

    let weight_draw_value: Box<dyn Interface> = Box::new(Slider {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: BLACK,
        background_color: SECONDARY_COLOR,
        text: "Weight Value".to_string(),
        id: "Weight_Draw".to_string(),
        active: false,
        range: 255,
        slider_offset_axis: 0,
        drawn: RefCell::new(false),
        cached_texture: None,
        value: 0,
        is_vertical: false,
        minimal: false,
    });

    let obstacle_count: Box<dyn Interface> = Box::new(Slider {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: BLACK,
        background_color: SECONDARY_COLOR,
        text: "Obstacle Percentage".to_string(),
        id: "Obstacle_Count".to_string(),
        active: false,
        range: 100,
        slider_offset_axis: 0,
        drawn: RefCell::new(false),
        cached_texture: None,
        value: 0,
        is_vertical: false,
        minimal: false,
    });

    let weight_count: Box<dyn Interface> = Box::new(Slider {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: BLACK,
        background_color: SECONDARY_COLOR,
        text: "Weighted Percentage".to_string(),
        id: "Weighted_Tile_Count".to_string(),
        active: false,
        range: 100,
        slider_offset_axis: 0,
        drawn: RefCell::new(false),
        cached_texture: None,
        value: 0,
        is_vertical: false,
        minimal: false,
    });

    let dg_check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Dynamic Generation".to_string(),
        checked: false,
        location: Point::new(40, 40),
        height: 0,
        width: 0,
        id: "DG_Select".to_string(),
        active: true,
        drawn: RefCell::new(false),
    });

    let de_check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Doubling Experiment".to_string(),
        checked: false,
        location: Point::new(40, 40),
        height: 0,
        width: 0,
        id: "DE_Select".to_string(),
        active: true,
        drawn: RefCell::new(false),
    });

    let ma_check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Multiple Agents".to_string(),
        checked: false,
        location: Point::new(40, 40),
        id: "MA_Select".to_string(),
        active: true,
        drawn: RefCell::new(false),
        height: 0,
        width: 0,
    });

    let mg_check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Multiple Goals".to_string(),
        checked: false,
        location: Point::new(40, 40),
        height: 0,
        width: 0,
        id: "MG_Select".to_string(),
        active: true,
        drawn: RefCell::new(false),
    });

    let ra_check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Random Agents & Goals".to_string(),
        checked: false,
        location: Point::new(40, 40),
        height: 0,
        width: 0,
        id: "RA_Select".to_string(),
        active: true,
        drawn: RefCell::new(false),
    });

    let save_widget_display: Box<dyn Interface> = Box::new(InputBox {
        default_text: "Chosen_Directory".to_string(),
        text: "".to_string(),
        active: false,
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        height: 0,
        width: 0,
        id: String::from("Display"),
        location: Point::new(0, 0),
        drawn: RefCell::new(false),
        clicked_on: false,
    });

    let save_widget_name: Box<dyn Interface> = Box::new(InputBox {
        default_text: "File Name".to_string(),
        text: "".to_string(),
        active: false,
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        height: 0,
        width: 0,
        id: String::from("File_Name"),
        location: Point::new(0, 0),
        drawn: RefCell::new(false),
        clicked_on: false,
    });

    let home_dir = directory_tree.path.to_string_lossy().to_string();

    let save_widget_directories: Box<dyn Interface> = Box::new(FileExplorer {
        location: Point::new(0, 0),
        id: String::from("Save_File_Exp"),
        height: 0,
        width: 0,
        directories: Rc::clone(&directories),
        default_dir: home_dir.to_string(),
        current_display: home_dir.to_string(),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        scroll_slider: RefCell::new(Slider {
            height: 0,
            width: 20,
            location: Point::new(0, 0),
            text_color: BLACK,
            background_color: SECONDARY_COLOR,
            text: String::new(),
            id: "Save_File_Slider".to_string(),
            active: false,
            range: 1,
            value: 0,
            slider_offset_axis: 0,
            drawn: RefCell::new(false),
            cached_texture: None,
            is_vertical: true,
            minimal: true,
        }),
        filter_dir: true,
        cached_button_list: RefCell::new(None),
    });

    let save_widget_accept: Box<dyn Interface> = Box::new(StandardButton {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover: RefCell::new(false),
        text: String::from("Save"),
        id: String::from("Save_Wid_Save"),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        cached_texture: None,
    });

    let generate_grid: Box<dyn Interface> = Box::new(StandardButton {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover: RefCell::new(false),
        text: String::from("Generate Grid"),
        id: String::from("Gen_Grid"),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        cached_texture: None,
    });

    let save_widget_exit: Box<dyn Interface> = Box::new(StandardButton {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover: RefCell::new(false),
        text: String::from("Exit"),
        id: String::from("Save_Wid_Exit"),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        cached_texture: None,
    });

    /*----- File Explorer Components ----- */

    let start_board_button: Box<dyn Interface> = Box::new(StandardButton {
        height: 100,
        width: 200,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover: RefCell::new(false),
        text: "START".to_string(),
        id: String::from("START"),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        cached_texture: None,
    });

    let piece_select: Box<dyn Interface> = Box::new(OptionButton::new(
        50,
        200,
        Point::new(0, 0),
        String::from("Piece_Select"),
        false,
        vec![
            (
                String::from("Player"),
                InterfaceStyle {
                    text_color: BLACK,
                    background_color: GREEN,
                },
            ),
            (
                String::from("Enemy"),
                InterfaceStyle {
                    text_color: BLACK,
                    background_color: RED,
                },
            ),
            (
                String::from("Obstacle"),
                InterfaceStyle {
                    text_color: WHITE,
                    background_color: BLACK,
                },
            ),
            (
                String::from("Weighted"),
                InterfaceStyle {
                    text_color: BLACK,
                    background_color: Color::RGB(255, 140, 0),
                },
            ),
        ],
        false,
    ));

    let upload_map_button: Box<dyn Interface> = Box::new(StandardButton {
        height: 50,
        width: 200,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover: RefCell::new(false),
        text: "Upload Map".to_string(),
        id: String::from("Upload Map"),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        cached_texture: None,
    });

    let save_map_button: Box<dyn Interface> = Box::new(StandardButton {
        height: 50,
        width: 200,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover: RefCell::new(false),
        text: "Save Map".to_string(),
        id: String::from("Save Map"),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        cached_texture: None,
    });

    let mut debug_window = Box::new(DisplayBox::new(
        (window_width - control_width) as i32,
        (window_height - debug_height) as i32,
        control_width,
        debug_height,
        "Debug_Window",
    ));

    let board_control_layout: Vec<Vec<&'static str>> = vec![
        vec!["Upload Map"],
        vec!["Upload Map"],
        vec!["Save Map"],
        vec!["Save Map"],
        vec!["Piece_Select"],
        vec!["Path_Selector"],
        vec!["Weight_Draw"],
        vec!["Obstacle_Count"],
        vec!["Weighted_Tile_Count"],
        vec!["Iterations"],
        vec!["Gen_Mode_Selector"],
        vec!["Gen_Grid"],
        vec!["DG_Select"],
        vec!["DE_Select"],
        vec!["MA_Select"],
        vec!["MG_Select"],
        vec!["RA_Select"],
        vec!["START"],
        vec!["START"],
        vec!["Debug_Window"],
        vec!["Debug_Window"],
        vec!["Debug_Window"],
        vec!["Debug_Window"],
        vec!["Debug_Window"],
        vec!["Debug_Window"],
    ];

    let iteration_gen_value: Box<dyn Interface> = Box::new(Slider {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: BLACK,
        background_color: SECONDARY_COLOR,
        text: "Iterations".to_string(),
        id: "Iterations".to_string(),
        active: false,
        range: 100,
        slider_offset_axis: 0,
        drawn: RefCell::new(false),
        cached_texture: None,
        value: 0,
        is_vertical: false,
        minimal: false,
    });

    // Tells a widget what buttons to draw.
    let board_control_buttons = HashMap::from([
        ("Upload Map", upload_map_button),
        ("Save Map", save_map_button),
        ("START", start_board_button),
        ("DG_Select", dg_check),
        ("DE_Select", de_check),
        ("MA_Select", ma_check),
        ("MG_Select", mg_check),
        ("RA_Select", ra_check),
        ("Path_Selector", path_selector),
        ("Gen_Mode_Selector", generation_mode_selector),
        ("Piece_Select", piece_select),
        ("Weight_Draw", weight_draw_value),
        ("Obstacle_Count", obstacle_count),
        ("Weighted_Tile_Count", weight_count),
        ("Iterations", iteration_gen_value),
        ("Gen_Grid", generate_grid),
        ("Debug_Window", debug_window),
    ]);

    let mut board_control_widget: Widget = Widget {
        location: Point::new((0 + board_width + 3) as i32, 0),
        id: String::from("Board_Control"),
        result: None,
        height: control_height,
        width: control_width,
        active: false,
        buttons: board_control_buttons,
        layout: board_control_layout,
        drawn: false,
        cached_draw_order: None,
        cached_interface_location: None,
        important_component_clicked: false,
    };

    let save_layout: Vec<Vec<&'static str>> = vec![
        vec!["Display", "Display"],
        vec!["File_Name", "File_Name"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_Wid_Save", "Save_Wid_Exit"],
        vec!["Save_Wid_Save", "Save_Wid_Exit"],
    ];

    let save_widget_buttons: HashMap<&'static str, Box<dyn Interface>> = HashMap::from([
        ("Display", save_widget_display),
        ("File_Name", save_widget_name),
        ("Save_File_Exp", save_widget_directories),
        ("Save_Wid_Save", save_widget_accept),
        ("Save_Wid_Exit", save_widget_exit),
    ]);

    let mut save_widget = Widget {
        location: Point::new(window_width as i32 * 1 / 4, 0),
        id: String::from("SAVE_WIDGET"),
        result: Some(home_dir.to_string()),
        height: window_height / 2,
        width: window_width / 2,
        buttons: save_widget_buttons,
        layout: save_layout,
        active: false,
        drawn: false,
        cached_draw_order: None,
        cached_interface_location: None,
        important_component_clicked: false,
    };

    let search_file: Box<dyn Interface> = Box::new(InputBox {
        default_text: "Search File".to_string(),
        text: home_dir.to_string(),
        active: false,
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        height: 50,
        width: 200,
        id: String::from("Search_File"),
        location: Point::new(window_width as i32 - 200, 1),
        drawn: RefCell::new(false),
        clicked_on: false,
    });

    let select_file_explorer: Box<dyn Interface> = Box::new(FileExplorer {
        location: Point::new(0, 0),
        id: String::from("Select_File_Exp"),
        height: 0,
        width: 0,
        directories: Rc::clone(&directories),
        default_dir: home_dir.to_string(),
        current_display: home_dir.to_string(),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        scroll_slider: RefCell::new(Slider {
            height: 0,
            width: 20,
            location: Point::new(0, 0),
            text_color: BLACK,
            background_color: SECONDARY_COLOR,
            text: String::new(),
            id: "Select_File_Slider".to_string(),
            active: false,
            range: 1,
            value: 0,
            slider_offset_axis: 0,
            drawn: RefCell::new(false),
            cached_texture: None,
            is_vertical: true,
            minimal: true,
        }),
        filter_dir: false,
        cached_button_list: RefCell::new(None),
    });

    let go_back_button: Box<dyn Interface> = Box::new(StandardButton {
        height: 50,
        width: 200,
        location: Point::new(window_width as i32 - 200, window_height as i32 - 25),
        text_color: BLACK,
        background_color: SECONDARY_COLOR,
        hover: RefCell::new(false),
        text: "Back".to_string(),
        id: String::from("Back"),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        cached_texture: None,
    });

    let file_select_layout: Vec<Vec<&'static str>> = vec![
        vec!["Search_File"],
        vec!["Select_File_Exp"],
        vec!["Select_File_Exp"],
        vec!["Select_File_Exp"],
        vec!["Select_File_Exp"],
        vec!["Select_File_Exp"],
        vec!["Back"],
    ];

    let file_select_buttons = HashMap::from([
        ("Search_File", search_file),
        ("Select_File_Exp", select_file_explorer),
        ("Back", go_back_button),
    ]);

    let mut file_select_widget: Widget = Widget {
        location: Point::new(window_width as i32 * 1 / 4, 0),
        id: String::from("Board_Control"),
        result: Some(home_dir.to_string()),
        height: window_height / 2,
        width: window_width / 2,
        active: false,
        buttons: file_select_buttons,
        layout: file_select_layout,
        drawn: false,
        cached_draw_order: None,
        cached_interface_location: None,
        important_component_clicked: false,
    };

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut game_board: Board = Board {
        location: Point::new(0, 0),
        width: board_width,
        height: board_height,
        tile_amount_x: tiles_x,
        tile_amount_y: tiles_y,
        active: true,
        id: String::from("game_board"),
        selected_piece_type: TileType::Obstacle,
        cached_background: None,
        cached_grid: RefCell::new(None),
        multiple_agents: settings.enable_multiple_agents,
        multiple_goals: settings.enable_multiple_agents,
        agents: vec![],
        starts: vec![],
        goals: vec![],
    };

    canvas.set_draw_color(Color::RGB(87, 87, 81));
    canvas.clear();
    game_board.draw(&mut canvas);
    let (mut window_width, mut window_height) =
        canvas.output_size().expect("Unable to obtain window size");
    'running: loop {
        let mouse_state: sdl2::mouse::MouseState = sdl2::mouse::MouseState::new(&event_pump);
        let mouse_position = Point::new(mouse_state.x(), mouse_state.y());
        #[cfg(not(target_os = "windows"))]
        {
            canvas.set_draw_color(Color::RGB(87, 87, 81));
            canvas.clear();
            game_board.draw(&mut canvas);
            board_control_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);
        }
        /*-------- User UI -------- */
        let current_size = canvas.output_size().expect("Unable to obtain window size");
        if game_board.height > current_size.1 || game_board.width > board_width {
            let window = canvas.window_mut();
            window.maximize();
        }
        if (window_width, window_height) != current_size {
            (window_width, window_height) = current_size;
            canvas.set_draw_color(Color::RGB(87, 87, 81));
            #[cfg(target_os = "windows")]
            canvas.clear();

            game_board.change_location(Point::new(0, 0));

            game_board.draw(&mut canvas);
            board_control_widget.change_location(Point::new(
                window_width as i32 - board_control_widget.get_width() as i32,
                board_control_widget.get_location().y(),
            ));
            board_control_widget.change_height(window_height);
            board_control_widget.change_drawn(false);
            file_select_widget.change_location(Point::new(
                window_width as i32 / 2 - file_select_widget.get_width() as i32 / 2,
                file_select_widget.get_location().y(),
            ));
            file_select_widget.change_drawn(false);
            save_widget.change_location(Point::new(
                window_width as i32 / 2 - save_widget.get_width() as i32 / 2,
                save_widget.get_location().y(),
            ));
            save_widget.change_drawn(false);
            #[cfg(target_os = "windows")]
            board_control_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);
        }

        /*-------- Updates User UI Depending on State -------- */
        if save_file {
            board_control_widget.change_active(false);
            board_control_widget.change_drawn(false);
            save_widget.change_active(true);
            let result = save_widget.get_result();
            if let Some(save_display) = save_widget.buttons.get_mut("Display") {
                if let Some(display) = save_display.as_any().downcast_mut::<InputBox>() {
                    display.text = match &result {
                        Some(result) => result.to_string(),
                        None => display.text.to_string(),
                    };
                }
            }

            save_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);
        } else if select_file {
            /*------- File Selection Menu -------*/
            board_control_widget.change_active(false);
            board_control_widget.change_drawn(false);

            file_select_widget.change_active(true);

            let result = file_select_widget.get_result();
            if let Some(text_box) = file_select_widget.buttons.get_mut("Search_File") {
                if let Some(file_text) = text_box.as_any().downcast_mut::<InputBox>() {
                    file_text.text = match &result {
                        Some(result) => result.to_string(),
                        None => file_text.text.to_string(),
                    };
                }
            }

            /*let search_text = match &search_file.text.trim().len() {
                0 => None,
                _ => Some(search_file.text.as_str()),
            };

            layout::layout_root(
                &mut directory_buttons,
                Point::new(window_width as i32 - 200, 62),
                200,
                25,
                search_text,
            );*/

            file_select_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);

        /*------- File Selection Menu -------*/
        } else {
            file_select_widget.change_drawn(false);
            file_select_widget.change_active(false);
            save_widget.change_drawn(false);
            save_widget.change_active(false);
            /*------ Board Editing Components ------*/

            #[cfg(target_os = "windows")]
            board_control_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);

            board_control_widget.change_active(true);
            if change_gen_sliders {
                board_control_widget.change_labels(
                    vec!["Weight_Draw", "Obstacle_Count", "Weighted_Tile_Count"],
                    &replacement_labels,
                );
            }
        }

        if run_game_board {
            match game_board.run_board(
                &mut canvas,
                &settings.selected_algorithm,
                settings.enable_doubling_experiment,
                settings.enable_dynamic_generation,
                settings.enable_random_agents,
                settings.gen_obstacles,
                settings.weight_count,
                settings.iterations,
                settings.weight.max(1),
                settings.gen_mode,
            ) {
                Ok(value) => {
                    display_visual_path_result = true;
                    run_game_board = false;
                    results = value;
                }
                Err(e) => {
                    display_visual_path_result = false;
                    run_game_board = false;
                    if let Some(d_window) = board_control_widget.buttons.get_mut("Debug_Window") {
                        if let Some(d_window) = d_window.as_any().downcast_mut::<DisplayBox>() {
                            d_window.clear();

                            d_window.add_line(e);
                        }
                    }
                }
            }
        }

        if display_visual_path_result {
            match game_board.display_path_result() {
                true => {
                    game_board.reset_board();
                    game_board.draw(&mut canvas);
                    if let Some(d_window) = board_control_widget.buttons.get_mut("Debug_Window") {
                        if let Some(d_window) = d_window.as_any().downcast_mut::<DisplayBox>() {
                            d_window.clear();
                            for line in results.lines() {
                                d_window.add_line(line);
                            }
                        }
                    }
                    display_visual_path_result = false;
                }
                false => {
                    game_board.draw(&mut canvas);
                    canvas.present();
                    continue;
                }
            }
        }
        /*------ Board Editing Components ------*/

        /*-------- Updates User UI Depending on State --------*/

        /*-------- Handle Component Inputs --------*/
        if mouse_state.left() {
            if game_board.on_click(mouse_position).0 {
                game_board.draw(&mut canvas);
            }
            mouse_clicked_on = true;
            if !select_file || save_file {
                let (clicked_button, (_, _)) = board_control_widget.on_click(false, mouse_position);
                match clicked_button {
                    Some(name) => match name.as_str() {
                        "Weight_Draw" => {
                            if let Some(slider) =
                                board_control_widget.buttons.get_mut("Weight_Draw")
                            {
                                if let Some(sl) = slider.as_any().downcast_mut::<Slider>() {
                                    settings.weight = sl.value.max(1) as u8;
                                    match game_board.selected_piece_type {
                                        TileType::Player => {}
                                        TileType::Enemy => {}
                                        TileType::Floor => {}
                                        TileType::Obstacle => {}
                                        TileType::Path => {}
                                        _ => {
                                            game_board.selected_piece_type =
                                                TileType::Weighted(settings.weight)
                                        }
                                    }
                                }
                            };
                        }
                        "Obstacle_Count" => {
                            if let Some(slider) =
                                board_control_widget.buttons.get_mut("Obstacle_Count")
                            {
                                if let Some(sl) = slider.as_any().downcast_mut::<Slider>() {
                                    settings.gen_obstacles = sl.value.max(0);
                                }
                            };
                        }
                        "Weighted_Tile_Count" => {
                            if let Some(slider) =
                                board_control_widget.buttons.get_mut("Weighted_Tile_Count")
                            {
                                if let Some(sl) = slider.as_any().downcast_mut::<Slider>() {
                                    settings.weight_count = sl.value.max(0);
                                }
                            };
                        }
                        "Iterations" => {
                            if let Some(slider) = board_control_widget.buttons.get_mut("Iterations")
                            {
                                if let Some(sl) = slider.as_any().downcast_mut::<Slider>() {
                                    settings.iterations = sl.value.max(1) as usize;
                                }
                            };
                        }
                        _ => {}
                    },
                    None => {}
                }
            }
        } else if mouse_clicked_on {
            if save_file {
                let (clicked_button, (_, inner_button_clicked)) =
                    save_widget.on_click(true, mouse_position);
                match clicked_button {
                    Some(id) => {
                        save_widget.change_drawn(false);
                        match id.as_str() {
                            "Display" => {
                                video_subsystem.text_input().start();
                            }
                            "File_Name" => {
                                video_subsystem.text_input().start();
                            }
                            "Save_Wid_Exit" => {
                                save_file = false;
                                save_widget.change_active(false);
                                save_widget.change_result(Some(home_dir.clone()));
                                game_board.change_active(true);
                                canvas.set_draw_color(Color::RGB(87, 87, 81));
                                #[cfg(target_os = "windows")]
                                canvas.clear();
                                game_board.draw(&mut canvas);
                            }
                            "Save_Wid_Save" => {
                                let save_path = &save_widget.get_result().expect("No path given");
                                util::add_file_to_dir_map(
                                    Rc::clone(&directories),
                                    save_path.to_string(),
                                    &settings.save_file,
                                );
                                game_board
                                    .save_to_file(&save_path, &settings.save_file)
                                    .unwrap();
                                save_file = false;
                                save_widget.change_active(false);
                                save_widget.change_result(Some(home_dir.clone()));
                                canvas.set_draw_color(Color::RGB(87, 87, 81));
                                #[cfg(target_os = "windows")]
                                canvas.clear();
                                game_board.change_active(true);
                                game_board.draw(&mut canvas);
                            }
                            "Save_File_Exp" => {
                                if inner_button_clicked.is_some() {
                                    if let Some(file_exp) =
                                        save_widget.buttons.get_mut("Save_File_Exp")
                                    {
                                        if let Some(button) =
                                            file_exp.as_any().downcast_mut::<FileExplorer>()
                                        {
                                            let new_result = inner_button_clicked.expect("Nope");

                                            button.change_display(new_result.clone());
                                            save_widget.change_result(Some(new_result));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    None => {}
                };
            } else if select_file {
                let (clicked_button, (_, inner_button_clicked)) =
                    file_select_widget.on_click(true, mouse_position);
                match clicked_button {
                    Some(button) => {
                        //file_select_widget.change_drawn(false);
                        match button.as_str() {
                            "Search_File" => {
                                video_subsystem.text_input().start();
                            }
                            "Select_File_Exp" => {
                                if inner_button_clicked.is_some() {
                                    if let Some(file_exp) =
                                        file_select_widget.buttons.get_mut("Select_File_Exp")
                                    {
                                        if let Some(button) =
                                            file_exp.as_any().downcast_mut::<FileExplorer>()
                                        {
                                            let new_result = inner_button_clicked.expect("Nope");

                                            button.change_display(new_result.clone());
                                            file_select_widget
                                                .change_result(Some(new_result.clone()));
                                            if !fileDialog::is_directory(&new_result) {
                                                file_select_widget
                                                    .change_result(Some(home_dir.clone()));
                                                match scanner::board_from(
                                                    &new_result,
                                                    game_board.height,
                                                    game_board.tile_amount_x,
                                                ) {
                                                    Ok(board) => {
                                                        game_board = board;
                                                    }
                                                    Err(_) => {}
                                                }
                                                select_file = false;
                                                canvas.set_draw_color(Color::RGB(87, 87, 81));
                                                #[cfg(target_os = "windows")]
                                                canvas.clear();
                                                game_board.active = true;
                                                game_board.draw(&mut canvas);
                                            }
                                        }
                                    }
                                }
                            }
                            "Back" => {
                                file_select_widget.change_active(false);
                                select_file = false;
                                canvas.set_draw_color(Color::RGB(87, 87, 81));
                                #[cfg(target_os = "windows")]
                                canvas.clear();
                                file_select_widget.change_result(Some(home_dir.clone()));
                                game_board.change_active(true);
                                game_board.draw(&mut canvas);
                            }
                            _ => {}
                        }
                    }
                    None => {}
                }
            } else {
                let (clicked_button, (_, inner_button_clicked)) =
                    board_control_widget.on_click(true, mouse_position);
                match clicked_button {
                    Some(name) => match name.as_str() {
                        "START" => {
                            run_game_board = true;
                            game_board.clear_path();

                            if let Some(d_window) =
                                board_control_widget.buttons.get_mut("Debug_Window")
                            {
                                if let Some(d_window) =
                                    d_window.as_any().downcast_mut::<DisplayBox>()
                                {
                                    d_window.clear();
                                }
                            }

                            /*let cwd = env::current_dir().unwrap();
                            let data_path = cwd.join("testing.csv");
                            benchmarks::run_overall_benchmark(
                                &benchmarks::default_benchmark_configs(),
                                &["A* search", "Breadth First Search", "JPSW", "Greedy"],
                                15,
                                &data_path,
                            );*/
                        }
                        "Upload Map" => {
                            game_board.draw(&mut canvas);
                            game_board.change_active(false);
                            select_file = true;
                            game_board.draw(&mut canvas);
                        }
                        "Save Map" => {
                            save_file = true;
                            game_board.change_active(false);
                            save_widget.change_active(true);
                        }
                        "Piece_Select" => match inner_button_clicked {
                            Some(value) => match value.as_str() {
                                "Player" => {
                                    game_board.selected_piece_type = TileType::Player;
                                }
                                "Enemy" => {
                                    game_board.selected_piece_type = TileType::Enemy;
                                }
                                "Obstacle" => {
                                    game_board.selected_piece_type = TileType::Obstacle;
                                }
                                "Weighted" => {
                                    game_board.selected_piece_type =
                                        TileType::Weighted(settings.weight);
                                }
                                _ => {
                                    game_board.selected_piece_type = TileType::Floor;
                                }
                            },
                            None => {}
                        },
                        "Gen_Grid" => {
                            println!("{}", settings.enable_random_agents);
                            match settings.gen_mode {
                                settings::GenerationMode::Random => {
                                    game_board.generate_random_grid(
                                        settings.weight,
                                        settings.gen_obstacles as usize,
                                        settings.weight_count as usize,
                                        settings.enable_random_agents,
                                    );
                                }
                                settings::GenerationMode::City => {
                                    game_board.generate_organic_city(
                                        0,
                                        2,
                                        settings.weight.max(2).into(),
                                        settings.gen_obstacles as f32,
                                        2,
                                        settings.weight_count.max(2),
                                        settings.enable_random_agents,
                                    );
                                }
                            }
                            game_board.draw(&mut canvas);
                        }
                        "DG_Select" => {
                            if let Some(checkbox) =
                                board_control_widget.buttons.get_mut("DG_Select")
                            {
                                if let Some(cb) = checkbox.as_any().downcast_ref::<CheckBox>() {
                                    settings.enable_dynamic_generation = cb.checked;
                                }
                            }
                        }
                        "DE_Select" => {
                            if let Some(checkbox) =
                                board_control_widget.buttons.get_mut("DE_Select")
                            {
                                if let Some(cb) = checkbox.as_any().downcast_ref::<CheckBox>() {
                                    settings.enable_doubling_experiment = cb.checked;
                                }
                            }
                        }
                        "MA_Select" => {
                            if let Some(checkbox) =
                                board_control_widget.buttons.get_mut("MA_Select")
                            {
                                if let Some(cb) = checkbox.as_any().downcast_ref::<CheckBox>() {
                                    settings.enable_multiple_agents = cb.checked;
                                    game_board.multiple_agents = settings.enable_multiple_agents;
                                }
                            }
                        }
                        "MG_Select" => {
                            if let Some(checkbox) =
                                board_control_widget.buttons.get_mut("MG_Select")
                            {
                                if let Some(cb) = checkbox.as_any().downcast_ref::<CheckBox>() {
                                    settings.enable_multiple_goals = cb.checked;
                                    game_board.multiple_goals = settings.enable_multiple_goals
                                }
                            }
                        }
                        "RA_Select" => {
                            if let Some(checkbox) =
                                board_control_widget.buttons.get_mut("RA_Select")
                            {
                                if let Some(cb) = checkbox.as_any().downcast_ref::<CheckBox>() {
                                    settings.enable_random_agents = cb.checked;
                                }
                            }
                        }
                        "Path_Selector" => {
                            if let Some(dropdown) =
                                board_control_widget.buttons.get_mut("Path_Selector")
                            {
                                if let Some(dd) = dropdown.as_any().downcast_ref::<Dropdown>() {
                                    settings.selected_algorithm = dd.text.clone();
                                }
                            }
                        }
                        "Gen_Mode_Selector" => {
                            if let Some(dropdown) =
                                board_control_widget.buttons.get_mut("Gen_Mode_Selector")
                            {
                                if let Some(dd) = dropdown.as_any().downcast_ref::<Dropdown>() {
                                    match dd.text.as_str() {
                                        "City Generation" => {
                                            settings.gen_mode = settings::GenerationMode::City;
                                            change_gen_sliders = true;
                                            replacement_labels = vec![
                                                "Road Spacing Range",
                                                "Building Percentage",
                                                "Building Size Range",
                                            ];
                                        }
                                        "Random Generation" => {
                                            settings.gen_mode = settings::GenerationMode::Random;
                                            change_gen_sliders = true;
                                            replacement_labels = vec![
                                                "Weight Value",
                                                "Obstacle Percentage",
                                                "Weighted Percentage",
                                            ];
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    None => {}
                }
            }
            mouse_clicked_on = false;
        }
        /*-------- Handle Component Inputs -------- */

        /*-------- User UI --------- */

        /*--------  Key Controls --------*/
        for event in event_pump.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    ..
                } => {
                    if video_subsystem.text_input().is_active() {
                        if file_select_widget.is_active() {
                            if let Some(file_exp) =
                                file_select_widget.buttons.get_mut("Select_File_Exp")
                            {
                                if let Some(button) =
                                    file_exp.as_any().downcast_mut::<FileExplorer>()
                                {
                                    button.change_filter(file_select_widget.result.clone());
                                }
                            }
                            file_select_widget.change_drawn(false);
                        } else if save_widget.is_active() {
                            if let Some(file_exp) = save_widget.buttons.get_mut("Save_File_Exp") {
                                if let Some(button) =
                                    file_exp.as_any().downcast_mut::<FileExplorer>()
                                {
                                    button.change_filter(save_widget.result.clone());
                                }
                            }
                            if let Some(save_display) = save_widget.buttons.get_mut("File_Name") {
                                if let Some(display) =
                                    save_display.as_any().downcast_mut::<InputBox>()
                                {
                                    if display.clicked_on() {
                                        display.clicked_on = false;
                                        settings.save_file = display.text.clone();
                                    }
                                }
                            }
                            if let Some(save_display) = save_widget.buttons.get_mut("Display") {
                                if let Some(display) =
                                    save_display.as_any().downcast_mut::<InputBox>()
                                {
                                    display.clicked_on = false;
                                }
                            }
                            save_widget.change_drawn(false);
                        }

                        video_subsystem.text_input().stop()
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Backspace),
                    ..
                } => {
                    if video_subsystem.text_input().is_active() {
                        if file_select_widget.is_active() {
                            file_select_widget.result = match file_select_widget.result {
                                Some(mut tex) => {
                                    tex.pop();
                                    Some(tex)
                                }
                                None => file_select_widget.result,
                            };
                            file_select_widget.change_drawn(false);
                        } else if save_widget.is_active() {
                            save_widget.change_drawn(false);
                            if let Some(save_display) = save_widget.buttons.get_mut("Display") {
                                if let Some(display) =
                                    save_display.as_any().downcast_mut::<InputBox>()
                                {
                                    if display.clicked_on() {
                                        save_widget.result = match save_widget.result {
                                            Some(mut tex) => {
                                                tex.pop();
                                                Some(tex)
                                            }
                                            None => save_widget.result,
                                        };
                                    }
                                }
                            }

                            if let Some(save_display) = save_widget.buttons.get_mut("File_Name") {
                                if let Some(display) =
                                    save_display.as_any().downcast_mut::<InputBox>()
                                {
                                    if display.clicked_on() {
                                        match display.text.trim().is_empty() {
                                            true => {}
                                            false => {
                                                display.text.pop();
                                                settings.save_file = display.text.clone();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Event::TextInput { text, .. } => {
                    if video_subsystem.text_input().is_active() {
                        if file_select_widget.is_active() {
                            file_select_widget.result = match file_select_widget.result {
                                Some(tex) => Some(tex + &text),
                                None => Some(text),
                            };
                            file_select_widget.change_drawn(false);
                        } else if save_widget.is_active() {
                            if let Some(save_display) = save_widget.buttons.get_mut("Display") {
                                if let Some(display) =
                                    save_display.as_any().downcast_mut::<InputBox>()
                                {
                                    if display.clicked_on() {
                                        save_widget.result = match save_widget.result {
                                            Some(tex) => Some(tex + &text),
                                            None => Some(text.clone()),
                                        };
                                    }
                                }
                            }

                            if let Some(save_display) = save_widget.buttons.get_mut("File_Name") {
                                if let Some(display) =
                                    save_display.as_any().downcast_mut::<InputBox>()
                                {
                                    if display.clicked_on() {
                                        match display.text.trim().is_empty() {
                                            true => {
                                                display.text = text;
                                            }
                                            false => {
                                                display.text += &text;
                                            }
                                        }
                                    }
                                }
                            }
                            save_widget.change_drawn(false);
                        }
                    }
                }
                Event::MouseWheel { y, .. } => {
                    if let Some(d_window) = board_control_widget.buttons.get_mut("Debug_Window") {
                        if d_window.is_active() {
                            if let Some(d_window) = d_window.as_any().downcast_mut::<DisplayBox>() {
                                if y > 0 {
                                    d_window.scroll_up();
                                } else if y < 0 {
                                    d_window.scroll_down();
                                }
                            }
                        }
                    }
                }
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        /*--------  Key Controls --------*/

        /*-------- Updates values for board Generation -------- */

        //let obs_y: u32 = rand::thread_rng().gen_range(0..tiles_y);
        //let obs_x: u32 = rand::thread_rng().gen_range(0..tiles_x);
        /*-------- Updates values for board Generation -------- */

        // Cap at ~60 FPS
        canvas.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::{BenchmarkConfig, PathData};
    use crate::components::board::{Board, Tile, TileType};
    use crate::pathfinding::{get_algorithm, get_possible_moves, Agent};
    use crate::settings::{GameSettings, GenerationMode};
    use crate::util;
    use sdl2::pixels::Color;
    use sdl2::rect::{Point, Rect};
    use std::cell::RefCell;
    use std::time::Duration;

    // ==================== ensure_assets tests ====================

    #[test]
    fn test_ensure_assets_creates_data_directory() {
        let data_dir = ensure_assets();
        assert!(
            data_dir.exists(),
            "Data directory should exist after ensure_assets"
        );
    }

    #[test]
    fn test_ensure_assets_creates_font_file() {
        let data_dir = ensure_assets();
        let font_path = data_dir.join("fonts/OpenSans-Semibold.ttf");
        assert!(font_path.exists(), "Font file should exist");
        assert!(
            font_path.metadata().unwrap().len() > 0,
            "Font file should not be empty"
        );
    }

    #[test]
    fn test_ensure_assets_idempotent() {
        let dir1 = ensure_assets();
        let dir2 = ensure_assets();
        assert_eq!(
            dir1, dir2,
            "Calling ensure_assets twice should return the same path"
        );
    }

    #[test]
    fn test_embedded_font_bytes_not_empty() {
        assert!(
            !FONT_BYTES.is_empty(),
            "Embedded font bytes must not be empty"
        );
    }

    #[test]
    fn test_embedded_icon_bytes_not_empty() {
        assert!(
            !ICON_BYTES.is_empty(),
            "Embedded icon bytes must not be empty"
        );
    }

    // ==================== Settings tests ====================

    #[test]
    fn test_settings_default_values() {
        let settings = GameSettings::default();
        assert_eq!(settings.window_width, 1200);
        assert_eq!(settings.window_height, 800);
        assert!(!settings.fullscreen);
        assert!(!settings.enable_dynamic_generation);
        assert!(!settings.enable_doubling_experiment);
        assert!(!settings.enable_multiple_agents);
        assert!(!settings.enable_multiple_goals);
        assert!(!settings.enable_random_agents);
        assert_eq!(settings.selected_algorithm, "Greedy");
        assert_eq!(settings.tiles_x, 40);
        assert_eq!(settings.tiles_y, 40);
        assert_eq!(settings.weight, 1);
        assert_eq!(settings.iterations, 1);
    }

    #[test]
    fn test_settings_save_and_load_roundtrip() {
        let tmp = std::env::temp_dir().join("test_settings_roundtrip.json");
        let path = tmp.to_str().unwrap();

        let mut settings = GameSettings::default();
        settings.window_width = 1920;
        settings.tiles_x = 100;
        settings.selected_algorithm = "A* search".to_string();
        settings.weight = 42;
        settings.save(path).unwrap();

        let loaded = GameSettings::load(path).unwrap();
        assert_eq!(loaded.window_width, 1920);
        assert_eq!(loaded.tiles_x, 100);
        assert_eq!(loaded.selected_algorithm, "A* search");
        assert_eq!(loaded.weight, 42);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_settings_load_nonexistent_returns_default() {
        let loaded = GameSettings::load("/tmp/nonexistent_settings_12345.json").unwrap();
        let default = GameSettings::default();
        assert_eq!(loaded.window_width, default.window_width);
        assert_eq!(loaded.tiles_x, default.tiles_x);
    }

    #[test]
    fn test_settings_get_default_path_not_empty() {
        let path = GameSettings::get_default_path();
        assert!(!path.is_empty());
        assert!(path.contains("settings.json"));
    }

    #[test]
    fn test_settings_serialization_includes_gen_mode() {
        let settings = GameSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("gen_mode"));
        assert!(json.contains("Random"));
    }

    // ==================== Utility function tests ====================

    #[test]
    fn test_mouse_over_inside() {
        let rect = Rect::new(10, 10, 100, 100);
        assert!(util::mouse_over(rect, Point::new(50, 50)));
    }

    #[test]
    fn test_mouse_over_outside() {
        let rect = Rect::new(10, 10, 100, 100);
        assert!(!util::mouse_over(rect, Point::new(0, 0)));
        assert!(!util::mouse_over(rect, Point::new(200, 200)));
    }

    #[test]
    fn test_mouse_over_on_edge() {
        let rect = Rect::new(10, 10, 100, 100);
        assert!(util::mouse_over(rect, Point::new(10, 10)));
    }

    #[test]
    fn test_calculate_scaled_font_size_short_text() {
        let size = util::calculate_scaled_font_size(5, 200);
        assert!(size >= 4, "Font size should be at least 4");
        assert_eq!(size, 40); // 5 * 8 = 40 fits within 200
    }

    #[test]
    fn test_calculate_scaled_font_size_overflow() {
        let size = util::calculate_scaled_font_size(50, 100);
        assert!(size <= 100, "Scaled size should not exceed available width");
        assert!(size >= 4, "Font size should be at least minimum");
    }

    #[test]
    fn test_calculate_scaled_font_size_minimum() {
        let size = util::calculate_scaled_font_size(0, 100);
        assert_eq!(size, 4, "Should return minimum font size for empty text");
    }

    #[test]
    fn test_get_coordinate_from_idx_first() {
        let (x, y) = util::get_coordinate_from_idx(0, 10, 10);
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn test_get_coordinate_from_idx_mid() {
        let (x, y) = util::get_coordinate_from_idx(15, 10, 10);
        assert_eq!((x, y), (5, 1));
    }

    #[test]
    fn test_get_coordinate_from_idx_last() {
        let (x, y) = util::get_coordinate_from_idx(99, 10, 10);
        assert_eq!((x, y), (9, 9));
    }

    #[test]
    fn test_get_idx_from_coordinate_valid() {
        assert_eq!(util::get_idx_from_coordinate((0, 0), 10, 10), Some(0));
        assert_eq!(util::get_idx_from_coordinate((5, 1), 10, 10), Some(15));
        assert_eq!(util::get_idx_from_coordinate((9, 9), 10, 10), Some(99));
    }

    #[test]
    fn test_get_idx_from_coordinate_out_of_bounds() {
        assert_eq!(util::get_idx_from_coordinate((-1, 0), 10, 10), None);
        assert_eq!(util::get_idx_from_coordinate((0, -1), 10, 10), None);
        assert_eq!(util::get_idx_from_coordinate((10, 0), 10, 10), None);
        assert_eq!(util::get_idx_from_coordinate((0, 10), 10, 10), None);
    }

    #[test]
    fn test_coordinate_idx_roundtrip() {
        for idx in 0..100 {
            let (x, y) = util::get_coordinate_from_idx(idx, 10, 10);
            let recovered = util::get_idx_from_coordinate((x, y), 10, 10);
            assert_eq!(recovered, Some(idx));
        }
    }

    // ==================== Color constant tests ====================

    #[test]
    fn test_color_constants() {
        assert_eq!(RED, Color::RGB(255, 0, 0));
        assert_eq!(GREEN, Color::RGB(0, 255, 0));
        assert_eq!(BLACK, Color::RGB(0, 0, 0));
        assert_eq!(WHITE, Color::RGB(255, 255, 255));
        assert_eq!(YELLOW, Color::RGB(255, 255, 0));
    }

    // ==================== TileType tests ====================

    #[test]
    fn test_tile_type_equality() {
        assert_eq!(TileType::Obstacle, TileType::Obstacle);
        assert_eq!(TileType::Floor, TileType::Floor);
        assert_eq!(TileType::Player, TileType::Player);
        assert_eq!(TileType::Enemy, TileType::Enemy);
        assert_eq!(TileType::Weighted(10), TileType::Weighted(10));
        assert_ne!(TileType::Weighted(10), TileType::Weighted(20));
        assert_ne!(TileType::Obstacle, TileType::Floor);
    }

    #[test]
    fn test_tile_type_serialization() {
        let types = vec![
            TileType::Obstacle,
            TileType::Floor,
            TileType::Player,
            TileType::Enemy,
            TileType::Weighted(128),
        ];
        for tt in types {
            let json = serde_json::to_string(&tt).unwrap();
            let deserialized: TileType = serde_json::from_str(&json).unwrap();
            assert_eq!(tt, deserialized);
        }
    }

    // ==================== Tile tests ====================

    fn make_floor_tile(pos: (i32, i32)) -> Tile {
        Tile::new(
            pos,
            TileType::Floor,
            20,
            20,
            0,
            false,
            Color::RGB(255, 255, 255),
        )
    }

    fn make_obstacle_tile(pos: (i32, i32)) -> Tile {
        Tile::new(
            pos,
            TileType::Obstacle,
            20,
            20,
            0,
            false,
            Color::RGB(0, 0, 0),
        )
    }

    fn make_weighted_tile(pos: (i32, i32), weight: u8) -> Tile {
        Tile::new(
            pos,
            TileType::Weighted(weight),
            20,
            20,
            weight,
            false,
            Color::RGB(255, 140, 0),
        )
    }

    #[test]
    fn test_tile_is_traversable() {
        assert!(make_floor_tile((0, 0)).is_traversable());
        assert!(!make_obstacle_tile((0, 0)).is_traversable());
        assert!(make_weighted_tile((0, 0), 10).is_traversable());
    }

    #[test]
    fn test_tile_is_floor() {
        assert!(make_floor_tile((0, 0)).is_floor());
        assert!(!make_obstacle_tile((0, 0)).is_floor());
        assert!(!make_weighted_tile((0, 0), 10).is_floor());
    }

    // ==================== Board tests ====================

    fn make_test_board(width: u32, height: u32, tiles_x: u32, tiles_y: u32) -> Board {
        Board {
            location: Point::new(0, 0),
            width,
            height,
            tile_amount_x: tiles_x,
            tile_amount_y: tiles_y,
            active: true,
            id: String::from("test_board"),
            selected_piece_type: TileType::Obstacle,
            cached_background: None,
            cached_grid: RefCell::new(None),
            multiple_agents: false,
            multiple_goals: false,
            agents: vec![],
            starts: vec![],
            goals: vec![],
        }
    }

    #[test]
    fn test_board_tile_dimensions() {
        let board = make_test_board(200, 100, 10, 5);
        assert_eq!(board.tile_width(), 20);
        assert_eq!(board.tile_height(), 20);
    }

    #[test]
    fn test_board_grid_initialization() {
        let board = make_test_board(100, 100, 10, 10);
        let grid = board.grid();
        assert_eq!(grid.len(), 100);
    }

    #[test]
    fn test_board_grid_all_tiles_traversable_by_default() {
        let board = make_test_board(100, 100, 10, 10);
        let grid = board.grid();
        for tile in &grid {
            assert!(
                tile.is_traversable(),
                "All default tiles should be traversable"
            );
        }
    }

    #[test]
    fn test_board_generate_random_grid() {
        let mut board = make_test_board(200, 200, 20, 20);
        board.generate_random_grid(5, 30, 10, false);
        let grid = board.grid();
        assert_eq!(grid.len(), 400);

        let obstacles = grid.iter().filter(|t| !t.is_traversable()).count();
        assert!(obstacles > 0, "Random grid should have some obstacles");
    }

    #[test]
    fn test_board_generate_random_grid_with_random_agents() {
        let mut board = make_test_board(200, 200, 20, 20);
        board.generate_random_grid(5, 10, 10, true);
        assert!(!board.starts.is_empty(), "Should place at least one start");
        assert!(!board.goals.is_empty(), "Should place at least one goal");
    }

    #[test]
    fn test_board_generate_city_grid() {
        let mut board = make_test_board(200, 200, 20, 20);
        board.generate_organic_city(0, 2, 5, 30.0, 2, 4, false);
        let grid = board.grid();
        assert_eq!(grid.len(), 400);
    }

    #[test]
    fn test_board_on_click_outside_returns_false() {
        let mut board = make_test_board(100, 100, 10, 10);
        let (clicked, _) = board.on_click(Point::new(500, 500));
        assert!(!clicked);
    }

    #[test]
    fn test_board_on_click_inside_returns_true() {
        let mut board = make_test_board(100, 100, 10, 10);
        board.active = true;
        let (clicked, _) = board.on_click(Point::new(5, 5));
        assert!(clicked);
    }

    #[test]
    fn test_board_mouse_over_component() {
        let board = make_test_board(100, 100, 10, 10);
        assert!(board.mouse_over_component(Point::new(50, 50)));
        assert!(!board.mouse_over_component(Point::new(200, 200)));
    }

    #[test]
    fn test_board_change_location() {
        let mut board = make_test_board(100, 100, 10, 10);
        board.change_location(Point::new(50, 50));
        assert_eq!(board.get_location(), Point::new(50, 50));
    }

    #[test]
    fn test_board_change_active() {
        let mut board = make_test_board(100, 100, 10, 10);
        assert!(board.is_active());
        board.change_active(false);
        assert!(!board.is_active());
    }

    #[test]
    fn test_board_save_and_load_roundtrip() {
        let tmp_dir = std::env::temp_dir();
        let mut board = make_test_board(100, 100, 10, 10);
        board.generate_random_grid(5, 20, 10, true);

        let save_path = tmp_dir.to_str().unwrap();
        board
            .save_to_file(save_path, "test_board_roundtrip")
            .unwrap();

        let file_path = tmp_dir.join("test_board_roundtrip.json");
        let json = std::fs::read_to_string(&file_path).unwrap();
        let loaded = board.load_board_file(json).unwrap();

        assert_eq!(loaded.tile_amount_x, board.tile_amount_x);
        assert_eq!(loaded.tile_amount_y, board.tile_amount_y);
        assert_eq!(loaded.grid().len(), board.grid().len());

        std::fs::remove_file(&file_path).ok();
    }

    // ==================== Pathfinding tests ====================

    fn make_open_grid(w: u32, h: u32) -> Vec<Tile> {
        (0..(w * h))
            .map(|i| {
                let x = (i % w) as i32;
                let y = (i / w) as i32;
                make_floor_tile((x, y))
            })
            .collect()
    }

    fn make_grid_with_wall(w: u32, h: u32) -> Vec<Tile> {
        let mut grid = make_open_grid(w, h);
        // Vertical wall at x=5, leaving a gap at y=0
        for y in 1..h as i32 {
            let idx = (y as u32 * w + 5) as usize;
            grid[idx] = make_obstacle_tile((5, y));
        }
        grid
    }

    #[test]
    fn test_get_possible_moves_center() {
        let grid = make_open_grid(10, 10);
        let moves = get_possible_moves((5, 5), &grid, 10, 10);
        assert_eq!(moves.len(), 8, "Center tile should have 8 neighbors");
    }

    #[test]
    fn test_get_possible_moves_corner() {
        let grid = make_open_grid(10, 10);
        let moves = get_possible_moves((0, 0), &grid, 10, 10);
        assert_eq!(moves.len(), 3, "Top-left corner should have 3 neighbors");
    }

    #[test]
    fn test_get_possible_moves_edge() {
        let grid = make_open_grid(10, 10);
        let moves = get_possible_moves((0, 5), &grid, 10, 10);
        assert_eq!(moves.len(), 5, "Left edge tile should have 5 neighbors");
    }

    #[test]
    fn test_get_possible_moves_blocked() {
        let mut grid = make_open_grid(3, 3);
        // Surround center with obstacles
        for i in [0, 1, 2, 3, 5, 6, 7, 8] {
            let x = (i % 3) as i32;
            let y = (i / 3) as i32;
            grid[i] = make_obstacle_tile((x, y));
        }
        let moves = get_possible_moves((1, 1), &grid, 3, 3);
        assert_eq!(
            moves.len(),
            0,
            "Fully surrounded tile should have 0 neighbors"
        );
    }

    #[test]
    fn test_greedy_finds_path_open_grid() {
        let grid = make_open_grid(10, 10);
        let algo = get_algorithm("Greedy");
        let (path, _cost) = algo.find_path((0, 0), (9, 9), &grid, 10, 10);
        assert!(!path.is_empty(), "Greedy should find a path on open grid");
        assert_eq!(*path.last().unwrap(), (0, 0));
        assert_eq!(path[0], (9, 9));
    }

    #[test]
    fn test_jpsw_find_path_open_grid() {
        let grid = make_open_grid(10, 10);
        let algo = get_algorithm("JPSW");
        let (path, _cost) = algo.find_path((0, 0), (9, 9), &grid, 10, 10);
        assert!(!path.is_empty(), "JPSW should find a path on open grid");
        assert_eq!(*path.last().unwrap(), (0, 0));
        assert_eq!(path[0], (9, 9));
    }

    #[test]
    fn test_bfs_finds_path_open_grid() {
        let grid = make_open_grid(10, 10);
        let algo = get_algorithm("Breadth First Search");
        let (path, _cost) = algo.find_path((0, 0), (9, 9), &grid, 10, 10);
        assert!(!path.is_empty(), "BFS should find a path on open grid");
        assert_eq!(path[0], (9, 9));
    }

    #[test]
    fn test_astar_finds_path_open_grid() {
        let grid = make_open_grid(10, 10);
        let algo = get_algorithm("A* search");
        let (path, _cost) = algo.find_path((0, 0), (9, 9), &grid, 10, 10);
        assert!(!path.is_empty(), "A* should find a path on open grid");
        assert_eq!(path[0], (9, 9));
    }

    #[test]
    fn test_jpsw_finds_path_open_grid() {
        let grid = make_open_grid(10, 10);
        let algo = get_algorithm("JPSW");
        let (path, _cost) = algo.find_path((0, 0), (9, 9), &grid, 10, 10);
        assert!(!path.is_empty(), "JPSW should find a path on open grid");
        assert_eq!(path[0], (9, 9));
    }

    #[test]
    fn test_algorithms_find_path_around_wall() {
        let grid = make_grid_with_wall(10, 10);
        // Greedy is excluded: it uses random fallback moves and may not reliably
        // navigate around walls within the step limit.
        for name in &["Breadth First Search", "A* search", "JPSW"] {
            let algo = get_algorithm(name);
            let (path, _cost) = algo.find_path((0, 5), (9, 5), &grid, 10, 10);
            assert!(
                !path.is_empty(),
                "{} should find a path around the wall",
                name
            );
            assert_eq!(path[0], (9, 5));
        }
    }

    #[test]
    fn test_algorithms_no_path_when_blocked() {
        let mut grid = make_open_grid(5, 5);
        // Complete vertical wall at x=2
        for y in 0..5i32 {
            let idx = (y * 5 + 2) as usize;
            grid[idx] = make_obstacle_tile((2, y));
        }
        for name in &["Breadth First Search", "A* search"] {
            let algo = get_algorithm(name);
            let (path, _cost) = algo.find_path((0, 0), (4, 4), &grid, 5, 5);
            assert!(
                path.is_empty(),
                "{} should return empty path when goal is unreachable",
                name
            );
        }
    }

    #[test]
    fn test_agent_goal_reached() {
        let mut agent = Agent {
            start: (0, 0),
            goal: (5, 5),
            position: (0, 0),
            path: vec![],
        };
        assert!(!agent.goal_reached());
        agent.position = (5, 5);
        assert!(agent.goal_reached());
    }

    #[test]
    fn test_agent_is_path_possible() {
        let grid = make_open_grid(10, 10);
        let agent = Agent {
            start: (0, 0),
            goal: (9, 9),
            position: (0, 0),
            path: vec![],
        };
        assert!(agent.is_path_possible(&grid, 10, 10));
    }

    #[test]
    fn test_agent_path_impossible_when_blocked() {
        let mut grid = make_open_grid(5, 5);
        for y in 0..5i32 {
            let idx = (y * 5 + 2) as usize;
            grid[idx] = make_obstacle_tile((2, y));
        }
        let agent = Agent {
            start: (0, 0),
            goal: (4, 4),
            position: (0, 0),
            path: vec![],
        };
        assert!(!agent.is_path_possible(&grid, 5, 5));
    }

    // ==================== PathData / Benchmarks tests ====================

    #[test]
    fn test_path_data_update_and_averages() {
        let mut pd = PathData {
            wcf: vec![],
            memory: vec![],
            time: vec![],
            steps: vec![],
            path_cost: vec![],
        };
        pd.update_all(1.0, 100, Duration::from_millis(10), 50, 20);
        pd.update_all(3.0, 300, Duration::from_millis(30), 150, 60);

        assert!((pd.avg_wcf() - 2.0).abs() < f64::EPSILON);
        assert_eq!(pd.avg_memory(), 200);
        assert_eq!(pd.avg_steps(), 100);
        assert_eq!(pd.avg_path_cost(), 40);
        assert_eq!(pd.avg_time(), Duration::from_millis(20));
    }

    #[test]
    fn test_default_benchmark_configs() {
        let configs = benchmarks::default_benchmark_configs();
        assert!(!configs.is_empty());
        for cfg in &configs {
            assert!(cfg.grid_size > 0);
            assert!(cfg.obstacle_pct <= 100);
            assert!(cfg.weighted_pct <= 100);
        }
    }

    #[test]
    fn test_sobel_method_uniform_grid() {
        let grid = make_open_grid(10, 10);
        let wcf = benchmarks::sobel_method(&grid, 10, 10);
        // A uniform grid should have a low WCF (no weight variation)
        assert!(wcf >= 0.0, "WCF should be non-negative");
    }

    #[test]
    fn test_sobel_method_weighted_grid() {
        let mut grid = make_open_grid(10, 10);
        // Add some weighted tiles to create variation
        for i in (0..100).step_by(3) {
            let x = (i % 10) as i32;
            let y = (i / 10) as i32;
            grid[i] = make_weighted_tile((x, y), 200);
        }
        let wcf = benchmarks::sobel_method(&grid, 10, 10);
        assert!(wcf >= 0.0, "WCF should be non-negative for weighted grid");
    }

    // ==================== fileDialog tests ====================

    #[test]
    fn test_is_directory() {
        assert!(fileDialog::is_directory("/tmp"));
        assert!(!fileDialog::is_directory(
            "/tmp/nonexistent_file_abc123.txt"
        ));
    }

    #[test]
    fn test_get_current_directory() {
        let cwd = fileDialog::get_current_directory();
        assert!(cwd.exists());
        assert!(cwd.is_dir());
    }

    #[test]
    fn test_file_save_and_read_roundtrip() {
        let tmp_dir = std::env::temp_dir().join("test_file_dialog_roundtrip_dir");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let content = "hello, pathmaker!".to_string();
        fileDialog::save_file(tmp_dir.to_str().unwrap().to_string(), content.clone());
        let actual_file = tmp_dir.join("test.json");
        let read_back = fileDialog::read_file(actual_file.to_str().unwrap()).unwrap();
        assert_eq!(read_back, content);
        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    // ==================== Integration-style tests ====================

    #[test]
    fn test_board_generate_then_pathfind() {
        let mut board = make_test_board(200, 200, 20, 20);
        board.generate_random_grid(5, 10, 5, true);

        let grid = board.grid();
        let starts = &board.starts;
        let goals = &board.goals;

        if !starts.is_empty() && !goals.is_empty() {
            let start_pos = util::get_coordinate_from_idx(starts[0], 20, 20);
            let goal_pos = util::get_coordinate_from_idx(goals[0], 20, 20);
            let algo = get_algorithm("A* search");
            let (path, _) = algo.find_path(start_pos, goal_pos, &grid, 20, 20);
            // Path may or may not exist depending on random generation; just verify no panic
            let _ = path;
        }
    }

    #[test]
    fn test_board_multiple_agents_mode() {
        let mut board = make_test_board(200, 200, 20, 20);
        board.multiple_agents = true;
        board.generate_random_grid(5, 5, 5, true);
        // With multiple agents enabled and random agents, should place multiple starts
        // (depends on random generation, but should not panic)
        let _ = board.grid();
    }

    #[test]
    fn test_all_algorithms_consistent_on_simple_path() {
        let grid = make_open_grid(10, 10);
        let start = (0, 0);
        let goal = (9, 0); // Straight horizontal path

        let mut paths = Vec::new();
        for name in &["Greedy", "Breadth First Search", "A* search", "JPSW"] {
            let algo = get_algorithm(name);
            let (path, _cost) = algo.find_path(start, goal, &grid, 10, 10);
            assert!(!path.is_empty(), "{} should find the path", name);
            assert_eq!(*path.last().unwrap(), start);
            assert_eq!(path[0], goal);
            paths.push((name, path));
        }
    }

    #[test]
    fn test_weighted_tiles_affect_path_cost() {
        let mut grid = make_open_grid(10, 10);
        // Add heavy weights on the direct diagonal path
        for i in 1..9 {
            let idx = i * 10 + i;
            grid[idx] = make_weighted_tile((i as i32, i as i32), 200);
        }
        let algo = get_algorithm("A* search");
        let (path_weighted, cost_weighted) = algo.find_path((0, 0), (9, 9), &grid, 10, 10);
        assert!(!path_weighted.is_empty());

        let grid_open = make_open_grid(10, 10);
        let (path_open, cost_open) = algo.find_path((0, 0), (9, 9), &grid_open, 10, 10);
        assert!(!path_open.is_empty());

        // The weighted grid path should cost more or take a detour
        assert!(
            cost_weighted >= cost_open,
            "Path through weighted tiles should cost at least as much"
        );
    }
}
