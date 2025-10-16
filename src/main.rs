extern crate sdl2;
use sdl2::event::Event;
use sdl2::gfx;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::ttf;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

// mod button;
mod colors;

mod components;

mod fileDialog;

mod layout;

mod util;

use crate::colors::*;
use crate::components::file_explorer::FileExplorer;
use crate::components::{board::*, button::*, inputbox::*, widget::*, Component};

pub fn main() {
    const BOARD_WIDTH: u32 = 800;
    const BOARD_HEIGHT: u32 = 800;
    let window_width: u32 = 1200;
    let window_height: u32 = 800;
    const TILES_X: u32 = 40;
    const TILES_Y: u32 = 40;
    let sdl_context = sdl2::init().unwrap();

    let video_subsystem = sdl_context.video().unwrap();
    video_subsystem.text_input().stop();
    let window = video_subsystem
        .window("PathMaker demo", window_width, window_height)
        .position_centered()
        .build()
        .expect("Failed to render Window");
    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();
    let directory_tree = fileDialog::get_file_tree();
    let mut select_file: bool = false;
    let mut components_drawn: bool = false;
    let mut save_file: bool = false;

    let ttf_context: ttf::Sdl2TtfContext = ttf::init().unwrap();

    let mut font: ttf::Font<'_, 'static> = ttf_context
        .load_font("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 124)
        .expect("Unable to Load Font");

    /*----- File Explorer Components ----- */

    let mut controls_width = window_width * 1 / 6;
    //= Rect::new(998, 0, 1000, 1000);

    let mut search_file = InputBox {
        default_text: "Search File".to_string(),
        text: "".to_string(),
        active: false,
        text_color: TERTIARY_COLOR,
        background_color: PRIMARY_COLOR,
        height: 50,
        width: 200,
        id: String::from("Search_File"),
        location: Point::new(window_width as i32 - 200, 1),
    };

    let mut directory_buttons: Vec<Box<dyn ValidDropdownOption>> =
        util::walk_tree(&directory_tree, window_width);

    let directories: HashMap<String, Vec<Box<dyn ValidDropdownOption>>> =
        util::get_dir_map(&directory_tree, window_width);

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
    });

    let home_dir = directory_tree.path.to_string_lossy().to_string();

    let save_widget_directories: Box<dyn Interface> = Box::new(FileExplorer {
        location: Point::new(0, 0),
        id: String::from("Save_File_Exp"),
        height: 0,
        width: 0,
        directories: RefCell::new(directories),
        current_display: home_dir.clone(),
        active: false,
    });

    let save_widget_accept: Box<dyn Interface> = Box::new(StandardButton {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover_color: QUATERNARY_COLOR,
        text: String::from("Save"),
        id: String::from("Save_Wid_Save"),
        filter: None,
        active: false,
    });

    let save_widget_exit: Box<dyn Interface> = Box::new(StandardButton {
        height: 0,
        width: 0,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover_color: QUATERNARY_COLOR,
        text: String::from("Exit"),
        id: String::from("Save_Wid_Exit"),
        filter: None,
        active: false,
    });

    /*----- File Explorer Components ----- */

    let start_board_button: Box<dyn Interface> = Box::new(StandardButton {
        height: 100,
        width: 200,
        location: Point::new(0, 0),
        text_color: BLACK,
        background_color: GREEN,
        hover_color: WHITE,
        text: "START".to_string(),
        id: String::from("START"),
        filter: None,
        active: false,
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
                    hover_color: HOVER_COLOR,
                },
            ),
            (
                String::from("Enemy"),
                InterfaceStyle {
                    text_color: BLACK,
                    background_color: RED,
                    hover_color: HOVER_COLOR,
                },
            ),
            (
                String::from("Obstacle"),
                InterfaceStyle {
                    text_color: WHITE,
                    background_color: BLACK,
                    hover_color: HOVER_COLOR,
                },
            ),
        ],
    ));

    let upload_map_button: Box<dyn Interface> = Box::new(StandardButton {
        height: 50,
        width: 200,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover_color: HOVER_COLOR,
        text: "Upload Map".to_string(),
        id: String::from("Upload Map"),

        filter: None,
        active: false,
    });

    let save_map_button: Box<dyn Interface> = Box::new(StandardButton {
        height: 50,
        width: 200,
        location: Point::new(0, 0),
        text_color: WHITE,
        background_color: PRIMARY_COLOR,
        hover_color: HOVER_COLOR,
        text: "Save Map".to_string(),
        id: String::from("Save Map"),

        filter: None,
        active: false,
    });

    let board_control_layout: Vec<Vec<&'static str>> = vec![
        vec!["Upload Map"],
        vec!["Save Map"],
        vec!["Piece_Select"],
        vec!["START"],
    ];

    let board_control_buttons = HashMap::from([
        ("Upload Map", upload_map_button),
        ("Save Map", save_map_button),
        ("START", start_board_button),
        ("Piece_Select", piece_select),
    ]);

    let mut board_control_widget: Widget = Widget {
        location: Point::new(window_width as i32 * 5 / 6, 0),
        id: String::from("Board_Control"),
        result: None,
        height: window_height,
        width: controls_width,
        active: false,
        buttons: board_control_buttons,
        layout: board_control_layout,
    };

    let save_layout: Vec<Vec<&'static str>> = vec![
        vec!["Display", "Display"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_File_Exp", "Save_File_Exp"],
        vec!["Save_Wid_Save", "Save_Wid_Exit"],
    ];

    let save_widget_buttons: HashMap<&'static str, Box<dyn Interface>> = HashMap::from([
        ("Display", save_widget_display),
        ("Save_File_Exp", save_widget_directories),
        ("Save_Wid_Save", save_widget_accept),
        ("Save_Wid_Exit", save_widget_exit),
    ]);

    let mut save_widget = Widget {
        location: Point::new(200, 60),
        id: String::from("SAVE_WIDGET"),
        result: Some(home_dir),
        height: 300,
        width: 500,
        buttons: save_widget_buttons,
        layout: save_layout,
        active: false,
    };

    let mut go_back_button = StandardButton {
        height: 50,
        width: 200,
        location: Point::new(window_width as i32 - 200, window_height as i32 - 25),
        text_color: BLACK,
        background_color: SECONDARY_COLOR,
        hover_color: HOVER_COLOR,
        text: "Back".to_string(),
        id: String::from("Back"),
        filter: None,
        active: false,
    };

    /*----- File Explorer Components ----- */

    let player_pos = HashSet::new();
    let enemy_pos = HashSet::new();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut game_board: Board = Board {
        location: Point::new(
            (window_width as i32 - BOARD_WIDTH as i32) / 2,
            (window_height as i32 - BOARD_HEIGHT as i32) / 2,
        ),
        width: BOARD_WIDTH,
        height: BOARD_HEIGHT,
        tile_amount_x: TILES_X,
        tile_amount_y: TILES_Y,
        enemy_pos,
        player_pos,
        obstacles: HashSet::new(),
        active: false,
        id: String::from("game_board"),
        selected_piece_type: TileType::Obstacle,
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
                (window_width as i32 - BOARD_WIDTH as i32) / 2,
                (window_height as i32 - BOARD_HEIGHT as i32) / 2,
            ));

            game_board.draw(&mut canvas);
            board_control_widget.change_location(Point::new(
                window_width as i32 - board_control_widget.get_width() as i32,
                board_control_widget.get_location().y(),
            ));
        }

        /*-------- Updates User UI Depending on State -------- */
        if save_file {
            let result = save_widget.get_result();
            if let Some(save_display) = save_widget.buttons.get_mut("Display") {
                if let Some(display) = save_display.as_any().downcast_mut::<InputBox>() {
                    display.change_text(result.expect("No path"));
                }
            }

            if util::mouse_over(save_widget.get_rect(), mouse_position) {
                save_widget.draw(
                    &mut canvas,
                    &texture_creator,
                    Some(mouse_position),
                    &mut font,
                );
            }
        } else if select_file {
            /*------- File Selection Menu -------*/
            board_control_widget.change_active(false);

            go_back_button.active = true;

            let search_text = match &search_file.text.trim().len() {
                0 => None,
                _ => Some(search_file.text.as_str()),
            };

            layout::layout_root(
                &mut directory_buttons,
                Point::new(window_width as i32 - 200, 62),
                200,
                25,
                search_text,
            );

            search_file.draw(&mut canvas, &texture_creator, None, &mut font);
            directory_buttons
                .iter_mut()
                .filter(|a| a.contains(search_text))
                .for_each(|a| {
                    if search_text.is_some() {
                        a.set_filter(search_text);
                    }
                    a.change_active(true);
                    a.draw(
                        &mut canvas,
                        &texture_creator,
                        Some(mouse_position),
                        &mut font,
                    );
                });
            go_back_button.draw(
                &mut canvas,
                &texture_creator,
                Some(mouse_position),
                &mut font,
            );

            components_drawn = true;

            /*------- File Selection Menu -------*/
        } else {
            /*------ Board Editing Components ------*/
            if components_drawn {
                board_control_widget.draw(
                    &mut canvas,
                    &texture_creator,
                    Some(mouse_position),
                    &mut font,
                );
            } else {
                directory_buttons
                    .iter_mut()
                    .for_each(|a: &mut Box<dyn ValidDropdownOption>| {
                        a.change_active(false);
                    });

                board_control_widget.change_active(true);

                canvas.set_draw_color(PRIMARY_COLOR);
                canvas.clear();
                game_board.draw(&mut canvas);
                board_control_widget.draw(
                    &mut canvas,
                    &texture_creator,
                    Some(mouse_position),
                    &mut font,
                );
                components_drawn = true;
            }
        }
        /*------ Board Editing Components ------*/

        /*-------- Updates User UI Depending on State --------*/

        /*-------- Handle Component Inputs --------*/
        if mouse_state.left() {
            if save_file {
                let (clicked_button, (_, inner_button_clicked)) =
                    save_widget.on_click(mouse_position);
                match clicked_button {
                    Some(id) => match id.as_str() {
                        "Save_Wid_Exit" => {
                            save_file = false;
                            components_drawn = false;
                        }
                        "Save_Wid_Save" => {
                            fileDialog::save_file(
                                save_widget.get_result().expect("No path given"),
                                game_board.map_json(),
                            );
                            save_file = false;
                            components_drawn = false;
                        }
                        "Save_File_Exp" => {
                            if inner_button_clicked.is_some() {
                                if let Some(file_exp) = save_widget.buttons.get_mut("Save_File_Exp")
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
                            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 15));
                        }
                        _ => {}
                    },
                    None => {}
                };
            }

            if game_board.on_click(mouse_position).0 {
                println!("Board Clicked On");
                game_board.draw(&mut canvas);
            }
            if !select_file {
                let (clicked_button, (_, inner_button_clicked)) =
                    board_control_widget.on_click(mouse_position);
                println!("{:#?}", clicked_button);
                match clicked_button {
                    Some(name) => match name.as_str() {
                        "START" => {
                            game_board.active = true;
                        }
                        "Upload Map" => {
                            canvas.set_draw_color(Color::RGB(87, 87, 81));
                            canvas.clear();
                            game_board.draw(&mut canvas);
                            select_file = true;
                            components_drawn = false;
                            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 15));
                        }
                        "Save Map" => {
                            save_file = true;
                            game_board.change_active(false);
                            save_widget.change_active(true);
                            save_widget.draw(
                                &mut canvas,
                                &texture_creator,
                                Some(mouse_position),
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
                                _ => {
                                    game_board.selected_piece_type = TileType::Floor;
                                }
                            },
                            None => {}
                        },
                        _ => {}
                    },
                    None => {}
                }
            } else {
                if search_file.on_click(mouse_position).0 {
                    video_subsystem.text_input().start();
                    search_file.active = true;
                } else if go_back_button.mouse_over_component(mouse_position) {
                    search_file.text = "".to_string();
                    search_file.active = false;
                    canvas.set_draw_color(Color::RGB(87, 87, 81));
                    canvas.clear();
                    game_board.draw(&mut canvas);
                    select_file = false;
                    components_drawn = false;
                    ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 15));
                } else {
                    // run on_click and relayout if any button handled the click
                    for b in directory_buttons.iter_mut() {
                        match b.get_type() {
                            ValidDropdownType::Standard => {
                                if b.on_click(mouse_position).0 {
                                    println!("Clicked ID: {:#?}", b.get_id());
                                    let (
                                        obstacle_map,
                                        player_map,
                                        enemy_map,
                                        tile_amount_x,
                                        tile_amount_y,
                                    ) = fileDialog::parse_map_file(fileDialog::read_file(
                                        &b.get_id(),
                                    ));
                                    game_board = Board {
                                        location: game_board.location,
                                        width: game_board.width,
                                        height: game_board.height,
                                        tile_amount_x,
                                        tile_amount_y,
                                        enemy_pos: enemy_map,
                                        player_pos: player_map,
                                        obstacles: obstacle_map,
                                        active: game_board.active,
                                        id: game_board.id,
                                        selected_piece_type: game_board.selected_piece_type,
                                    };
                                    game_board.draw(&mut canvas);
                                    ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 15));
                                }
                            }
                            ValidDropdownType::Dropdown => {
                                let (result, clicked_button) = b.on_click(mouse_position);
                                if result {
                                    if clicked_button.is_some() {
                                        println!("Clicked ID: {:#?}", clicked_button);
                                        let (
                                            obstacle_map,
                                            player_map,
                                            enemy_map,
                                            tile_amount_x,
                                            tile_amount_y,
                                        ) = fileDialog::parse_map_file(fileDialog::read_file(
                                            &clicked_button.unwrap(),
                                        ));
                                        game_board = Board {
                                            location: game_board.location,
                                            width: game_board.width,
                                            height: game_board.height,
                                            tile_amount_x,
                                            tile_amount_y,
                                            enemy_pos: enemy_map,
                                            player_pos: player_map,
                                            obstacles: obstacle_map,
                                            active: game_board.active,
                                            id: game_board.id,
                                            selected_piece_type: game_board.selected_piece_type,
                                        };
                                        game_board.draw(&mut canvas);
                                    }
                                }
                            }
                        }
                    }
                }
                // sleep for short period after input so to prevent accidental double clicks !BANDAID FIX!
                ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 15));
            }
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
                        video_subsystem.text_input().stop()
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Backspace),
                    ..
                } => {
                    if video_subsystem.text_input().is_active() {
                        if search_file.active {
                            search_file.text.pop();
                        }
                    }
                }

                Event::TextInput { text, .. } => {
                    search_file.text += &text;
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

        //let obs_y: u32 = rand::thread_rng().gen_range(0..TILES_Y);
        //let obs_x: u32 = rand::thread_rng().gen_range(0..TILES_X);
        /*-------- Updates values for board Generation -------- */

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
