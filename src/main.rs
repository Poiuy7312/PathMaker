extern crate sdl2;
use jemalloc_ctl::{epoch, stats};
use sdl2::event::Event;
use sdl2::gfx;
use sdl2::image::LoadSurface;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::surface::Surface;
use sdl2::ttf;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{env, thread};

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

// mod button;
mod benchmarks;
mod colors;

mod components;

mod fileDialog;

mod pathfinding;
mod settings;
mod util;

use crate::benchmarks::sobel_method;
use crate::colors::*;

use crate::components::file_explorer::FileExplorer;
use crate::components::{board::*, button::*, inputbox::*, widget::*, Component};
use crate::settings::GameSettings;

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
    let tiles_x: u32 = settings.tiles_x; // Replace with settings.tiles_x
    let tiles_y: u32 = settings.tiles_y;
    let sdl_context = sdl2::init().unwrap();

    let video_subsystem = sdl_context.video().unwrap();
    video_subsystem.text_input().stop();
    let mut window = video_subsystem
        .window("PathMaker demo", window_width, window_height)
        .position_centered()
        .build()
        .expect("Failed to render Window");
    let window_icon = Surface::from_file("src/assets/Icon.svg").unwrap();
    window.set_icon(window_icon);
    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();
    let directory_tree = fileDialog::get_file_tree();
    let mut select_file: bool = false; // Check if select file widget is active
    let mut save_file: bool = false; // Check if save file widget is active
    let mut gen_menu: bool = false; // Check if controls for board generation are active

    let ttf_context: ttf::Sdl2TtfContext = ttf::init().unwrap();

    let mut font: ttf::Font<'_, 'static> = ttf_context
        .load_font("src/assets/open-sans/OpenSans-Semibold.ttf", 124)
        .expect("Unable to Load Font");

    /*----- File Explorer Components ----- */

    let mut mouse_clicked_on: bool = false;

    let controls_width = window_width * 1 / 5;

    let mut run_game_board = false;
    //= Rect::new(998, 0, 1000, 1000);

    let directories: HashMap<String, (StandardButton, Vec<String>)> =
        util::get_dir_map(&directory_tree, window_width);

    let filtered_directories: HashMap<String, (StandardButton, Vec<String>)> =
        util::get_dir_map(&directory_tree, window_width)
            .extract_if(|k, _| fileDialog::is_directory(k))
            .collect();

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
        text: "Weight_Draw".to_string(),
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

    let DG_Check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Dynamic Generation".to_string(),
        checked: false,
        location: Point::new(40, 40),
        height: 0,
        width: 0,
        id: "DG_Select".to_string(),
        active: true,
        drawn: RefCell::new(false),
    });

    let DE_Check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Doubling Experiment".to_string(),
        checked: false,
        location: Point::new(40, 40),
        height: 0,
        width: 0,
        id: "DE_Select".to_string(),
        active: true,
        drawn: RefCell::new(false),
    });

    let MA_Check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Multiple Agents".to_string(),
        checked: false,
        location: Point::new(40, 40),
        id: "MA_Select".to_string(),
        active: true,
        drawn: RefCell::new(false),
        height: 0,
        width: 0,
    });

    let MG_Check: Box<dyn Interface> = Box::new(CheckBox {
        label: "Multiple Goals".to_string(),
        checked: false,
        location: Point::new(40, 40),
        height: 0,
        width: 0,
        id: "MG_Select".to_string(),
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
    });

    let save_widget_name: Box<dyn Interface> = Box::new(InputBox {
        default_text: "File Name".to_string(),
        text: "test.json".to_string(),
        active: false,
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        height: 0,
        width: 0,
        id: String::from("File_Name"),
        location: Point::new(0, 0),
        drawn: RefCell::new(false),
    });

    let home_dir = directory_tree.path.to_string_lossy().to_string();

    let save_widget_directories: Box<dyn Interface> = Box::new(FileExplorer {
        location: Point::new(0, 0),
        id: String::from("Save_File_Exp"),
        height: 0,
        width: 0,
        directories: RefCell::new(filtered_directories),
        default_dir: home_dir.clone(),
        current_display: home_dir.clone(),
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
            range: 0,
            value: 0,
            slider_offset_axis: 0,
            drawn: RefCell::new(false),
            cached_texture: None,
            is_vertical: true,
            minimal: true,
        }),
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

    let board_control_layout: Vec<Vec<&'static str>> = vec![
        vec!["Upload Map"],
        vec!["Upload Map"],
        vec!["Save Map"],
        vec!["Save Map"],
        vec!["Piece_Select"],
        vec!["Path_Selector"],
        vec!["Weight_Draw"],
        vec!["DG_Select"],
        vec!["DE_Select"],
        vec!["MA_Select"],
        vec!["MG_Select"],
        vec!["START"],
        vec!["START"],
    ];

    let weight_gen_value: Box<dyn Interface> = Box::new(Slider {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: BLACK,
        background_color: SECONDARY_COLOR,
        text: "Weight_Range".to_string(),
        id: "Weight_Range".to_string(),
        active: false,
        range: 255,
        slider_offset_axis: 0,
        drawn: RefCell::new(false),
        cached_texture: None,
        value: 0,
        is_vertical: false,
        minimal: false,
    });

    let obstacle_gen_value: Box<dyn Interface> = Box::new(Slider {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: BLACK,
        background_color: SECONDARY_COLOR,
        text: "Number of Obstacles".to_string(),
        id: "Number of Obstacles".to_string(),
        active: false,
        range: 400,
        slider_offset_axis: 0,
        drawn: RefCell::new(false),
        cached_texture: None,
        value: 0,
        is_vertical: false,
        minimal: false,
    });

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

    let Gen_Ok: Box<dyn Interface> = Box::new(StandardButton {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: BLACK,
        background_color: GREEN,
        hover: RefCell::new(false),
        text: String::from("Ok"),
        id: String::from("Ok_Gen"),
        filter: None,
        active: false,
        drawn: RefCell::new(false),
        cached_texture: None,
    });

    let gen_control_layout: Vec<Vec<&'static str>> = vec![
        vec!["Weight_Range"],
        vec!["Number of Obstacles"],
        vec!["Iterations"],
        vec!["Ok_Gen"],
    ];
    let gen_control_buttons = HashMap::from([
        ("Weight_Range", weight_gen_value),
        ("Number of Obstacles", obstacle_gen_value),
        ("Iterations", iteration_gen_value),
        ("Ok_Gen", Gen_Ok),
    ]);

    let mut gen_control_widget = Widget {
        location: Point::new(window_width as i32 * 1 / 4, 0),
        id: String::from("GEN_WIDGET"),
        result: None,
        height: window_height / 2,
        width: window_width / 2,
        buttons: gen_control_buttons,
        layout: gen_control_layout,
        active: false,
        drawn: false,
        cached_draw_order: None,
        cached_interface_location: None,
        important_component_clicked: false,
    };

    // Tells a widget what buttons to draw.
    let board_control_buttons = HashMap::from([
        ("Upload Map", upload_map_button),
        ("Save Map", save_map_button),
        ("START", start_board_button),
        ("DG_Select", DG_Check),
        ("DE_Select", DE_Check),
        ("MA_Select", MA_Check),
        ("MG_Select", MG_Check),
        ("Path_Selector", path_selector),
        ("Piece_Select", piece_select),
        ("Weight_Draw", weight_draw_value),
    ]);

    let mut board_control_widget: Widget = Widget {
        location: Point::new(window_width as i32 * 4 / 5, 0),
        id: String::from("Board_Control"),
        result: None,
        height: window_height,
        width: controls_width,
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
        result: Some(home_dir.clone()),
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
    });

    let select_file_explorer: Box<dyn Interface> = Box::new(FileExplorer {
        location: Point::new(0, 0),
        id: String::from("Select_File_Exp"),
        height: 0,
        width: 0,
        directories: RefCell::new(directories),
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
            range: 0,
            value: 0,
            slider_offset_axis: 0,
            drawn: RefCell::new(false),
            cached_texture: None,
            is_vertical: true,
            minimal: true,
        }),
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
        /*-------- User UI -------- */
        let current_size = canvas.output_size().expect("Unable to obtain window size");
        if (window_width, window_height) != current_size {
            (window_width, window_height) = current_size;
            canvas.set_draw_color(Color::RGB(87, 87, 81));
            canvas.clear();

            game_board.change_location(Point::new(
                (window_width as i32 - board_width as i32) / 2,
                (window_height as i32 - board_height as i32) / 2,
            ));

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
            gen_control_widget.change_location(Point::new(
                window_width as i32 / 2 - gen_control_widget.get_width() as i32 / 2,
                gen_control_widget.get_location().y(),
            ));
            gen_control_widget.change_drawn(false);
            board_control_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);
        }

        if run_game_board {
            game_board.update_board(
                &mut canvas,
                &settings.selected_algorithm,
                settings.enable_doubling_experiment,
                settings.gen_obstacles,
                settings.iterations,
                settings.gen_weight.max(1),
            );
            run_game_board = false;
        }

        /*-------- Updates User UI Depending on State -------- */
        if save_file {
            board_control_widget.change_active(false);
            board_control_widget.change_drawn(false);
            let result = save_widget.get_result();
            if let Some(save_display) = save_widget.buttons.get_mut("Display") {
                if let Some(display) = save_display.as_any().downcast_mut::<InputBox>() {
                    display.text = match &result {
                        Some(result) => result.to_string(),
                        None => display.text.to_string(),
                    };
                }
            }

            if util::mouse_over(save_widget.get_rect(), mouse_position) {
                save_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);
            }
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
        } else if gen_menu {
            board_control_widget.change_active(false);
            board_control_widget.change_drawn(false);
            gen_control_widget.change_active(true);

            gen_control_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);

        /*------- File Selection Menu -------*/
        } else {
            file_select_widget.change_drawn(false);
            file_select_widget.change_active(false);
            save_widget.change_active(false);
            save_widget.change_active(false);
            /*------ Board Editing Components ------*/

            board_control_widget.draw(&mut canvas, &texture_creator, mouse_position, &mut font);

            board_control_widget.change_active(true);
        }
        /*------ Board Editing Components ------*/

        /*-------- Updates User UI Depending on State --------*/

        /*-------- Handle Component Inputs --------*/
        if mouse_state.left() {
            if game_board.on_click(mouse_position).0 {
                game_board.draw(&mut canvas);
            }
            mouse_clicked_on = true;
            if !select_file || save_file || gen_menu {
                if let Some(slider) = board_control_widget.buttons.get_mut("Weight_Draw") {
                    if let Some(sl) = slider.as_any().downcast_mut::<Slider>() {
                        if sl.on_click(mouse_position).0 {
                            settings.weight = sl.value.max(1) as u8;
                            match game_board.selected_piece_type {
                                TileType::Player => {}
                                TileType::Enemy => {}
                                TileType::Floor => {}
                                TileType::Obstacle => {}
                                _ => {
                                    game_board.selected_piece_type =
                                        TileType::Weighted(settings.weight)
                                }
                            }
                        }
                    }
                }
            }
            if gen_menu {
                let (clicked_button, (_, _)) = gen_control_widget.on_click(mouse_position);
                println!("E: {:#?}", clicked_button);

                match clicked_button {
                    Some(name) => match name.as_str() {
                        "Weight_Range" => {
                            if let Some(slider) = gen_control_widget.buttons.get_mut("Weight_Range")
                            {
                                if let Some(sl) = slider.as_any().downcast_ref::<Slider>() {
                                    settings.gen_weight = sl.value.max(1) as u8;
                                }
                            }
                        }
                        "Number of Obstacles" => {
                            if let Some(slider) =
                                gen_control_widget.buttons.get_mut("Number of Obstacles")
                            {
                                if let Some(sl) = slider.as_any().downcast_ref::<Slider>() {
                                    settings.gen_obstacles = sl.value as u16;
                                }
                            }
                        }
                        "Iterations" => {
                            if let Some(slider) = gen_control_widget.buttons.get_mut("Iterations") {
                                if let Some(sl) = slider.as_any().downcast_ref::<Slider>() {
                                    settings.iterations = sl.value.max(1) as u8;
                                }
                            }
                        }
                        _ => {}
                    },
                    None => {}
                }
            }
        } else if mouse_clicked_on {
            if save_file {
                let (clicked_button, (_, inner_button_clicked)) =
                    save_widget.on_click(mouse_position);
                match clicked_button {
                    Some(id) => {
                        save_widget.change_drawn(false);
                        match id.as_str() {
                            "Display" => {
                                video_subsystem.text_input().start();
                            }
                            "Save_Wid_Exit" => {
                                save_file = false;
                                save_widget.change_active(false);
                                save_widget.change_result(Some(home_dir.clone()));
                                game_board.change_active(true);
                                game_board.draw(&mut canvas);
                            }
                            "Save_Wid_Save" => {
                                game_board
                                    .save_to_file(&save_widget.get_result().expect("No path given"))
                                    .unwrap();
                                save_file = false;
                                save_widget.change_active(false);
                                save_widget.change_result(Some(home_dir.clone()));
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
                    file_select_widget.on_click(mouse_position);
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
                                                game_board = game_board.load_board_file(
                                                    fileDialog::read_file(&new_result),
                                                );
                                                select_file = false;
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
            } else if gen_menu {
                let (clicked_button, (_, inner_button_clicked)) =
                    gen_control_widget.on_click(mouse_position);
                match clicked_button {
                    Some(name) => match name.as_str().trim() {
                        "Weight_Range" => {
                            if let Some(slider) =
                                board_control_widget.buttons.get_mut("Weight_Range")
                            {
                                if let Some(sl) = slider.as_any().downcast_ref::<Slider>() {
                                    settings.gen_weight = sl.value.max(1) as u8;
                                }
                            }
                        }
                        "Number of Obstacles" => {
                            if let Some(slider) =
                                board_control_widget.buttons.get_mut("Number of Obstacles")
                            {
                                if let Some(sl) = slider.as_any().downcast_ref::<Slider>() {
                                    settings.gen_obstacles = sl.value as u16;
                                }
                            }
                        }
                        "Iterations" => {
                            if let Some(slider) = board_control_widget.buttons.get_mut("Iterations")
                            {
                                if let Some(sl) = slider.as_any().downcast_ref::<Slider>() {
                                    settings.iterations = sl.value.max(1) as u8;
                                }
                            }
                        }
                        "Ok_Gen" => {
                            gen_menu = false;
                            gen_control_widget.change_active(false);
                            gen_control_widget.change_drawn(false);
                            game_board.change_active(true);
                            run_game_board = true;
                        }
                        _ => {}
                    },
                    None => {}
                }
            } else {
                let (clicked_button, (_, inner_button_clicked)) =
                    board_control_widget.on_click(mouse_position);
                match clicked_button {
                    Some(name) => match name.as_str() {
                        "START" => {
                            if settings.enable_doubling_experiment
                                || settings.enable_dynamic_generation
                            {
                                game_board.change_active(false);
                                gen_menu = true;
                            } else {
                                run_game_board = true;
                            }
                        }
                        "Upload Map" => {
                            game_board.draw(&mut canvas);
                            game_board.change_active(false);
                            select_file = true;
                        }
                        "Save Map" => {
                            save_file = true;
                            game_board.change_active(false);
                            save_widget.change_active(true);
                            save_widget.draw(
                                &mut canvas,
                                &texture_creator,
                                mouse_position,
                                &mut font,
                            );
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
                        "Weight_Draw" => {
                            if let Some(slider) =
                                board_control_widget.buttons.get_mut("Weight_Draw")
                            {
                                if let Some(sl) = slider.as_any().downcast_ref::<Slider>() {
                                    settings.weight = sl.value.max(1) as u8;
                                    match game_board.selected_piece_type {
                                        TileType::Player => {}
                                        TileType::Enemy => {}
                                        TileType::Floor => {}
                                        _ => {
                                            game_board.selected_piece_type =
                                                TileType::Weighted(settings.weight)
                                        }
                                    }
                                }
                            }
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
                        "Path_Selector" => {
                            if let Some(dropdown) =
                                board_control_widget.buttons.get_mut("Path_Selector")
                            {
                                if let Some(dd) = dropdown.as_any().downcast_ref::<Dropdown>() {
                                    settings.selected_algorithm = dd.text.clone();
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
        canvas.present();
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
                            save_widget.result = match save_widget.result {
                                Some(mut tex) => {
                                    tex.pop();
                                    Some(tex)
                                }
                                None => save_widget.result,
                            };
                            save_widget.change_drawn(false);
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
                            save_widget.result = match save_widget.result {
                                Some(tex) => Some(tex + &text),
                                None => Some(text),
                            };
                            save_widget.change_drawn(false);
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
    }
}
