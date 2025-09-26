extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use std::collections::HashSet;
use std::time::Duration;

// mod button;
use crate::components::{board::*, button::*, inputbox::*};

mod components;

mod fileDialog;

const BOARD_WIDTH: u32 = 800;
const BOARD_HEIGHT: u32 = 800;
const WINDOW_WIDTH: u32 = 1200;
const WINDOW_HEIGHT: u32 = 800;
const TILES_X: u32 = 40;
const TILES_Y: u32 = 40;

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let directory_tree = fileDialog::get_files();
    let mut select_file: bool = false;
    let mut components_drawn: bool = false;
    let mut search_text_length = 0;

    /*----- File Explorer Components ----- */

    let mut search_file = InputBox {
        default_text: "Search File".to_string(),
        text: "".to_string(),
        active: false,
        text_color: [156, 156, 156],
        background_color: [84, 84, 84],
        height: 50,
        width: 400,
        location: Point::new(1000, 25),
    };

    let mut directory_buttons: Vec<Button> = directory_tree
        .iter()
        .enumerate()
        .map(|(count, (file, _))| Button {
            height: 24,
            width: 400,
            location: Point::new(1000, 62 + count as i32 * 25),
            text_color: [0, 0, 0],
            background_color: [150, 150, 150],
            hover_color: [255, 255, 255],
            text: file.to_string(),
            hover: false,
            active: false,
        })
        .collect();

    let mut go_back_button = Button {
        height: 50,
        width: 400,
        location: Point::new(1000, WINDOW_HEIGHT as i32 - 25),
        text_color: [0, 0, 0],
        background_color: [150, 150, 150],
        hover_color: [255, 255, 255],
        text: "Back".to_string(),
        hover: false,
        active: false,
    };

    /*----- File Explorer Components ----- */

    let mut start_board_button = Button {
        height: 100,
        width: 200,
        location: Point::new(1000, 600),
        text_color: [0, 0, 0],
        background_color: [150, 150, 150],
        hover_color: [255, 255, 255],
        text: "START".to_string(),
        hover: false,
        active: false,
    };

    let mut select_piece_one = Button {
        height: 50,
        width: 50,
        location: Point::new(940, 300),
        text_color: [0, 0, 0],
        background_color: [0, 255, 0],
        hover_color: [255, 255, 255],
        text: "Player".to_string(),
        hover: false,
        active: false,
    };

    let mut select_piece_two = Button {
        height: 50,
        width: 50,
        location: Point::new(1000, 300),
        text_color: [0, 0, 0],
        background_color: [255, 0, 0],
        hover_color: [255, 255, 255],
        text: "Enemy".to_string(),
        hover: false,
        active: false,
    };

    let mut select_piece_three = Button {
        height: 50,
        width: 50,
        location: Point::new(1060, 300),
        text_color: [255, 255, 255],
        background_color: [0, 0, 0],
        hover_color: [255, 255, 255],
        text: "Obstacle".to_string(),
        hover: false,
        active: false,
    };

    let mut upload_map_button = Button {
        height: 50,
        width: 380,
        location: Point::new(1000, 100),
        text_color: [255, 255, 255],
        background_color: [84, 84, 84],
        hover_color: [255, 255, 255],
        text: "Upload Map".to_string(),
        hover: false,
        active: false,
    };

    let mut save_map_button = Button {
        height: 50,
        width: 380,
        location: Point::new(1000, 200),
        text_color: [255, 255, 255],
        background_color: [84, 84, 84],
        hover_color: [255, 255, 255],
        text: "Save Map".to_string(),
        hover: false,
        active: false,
    };

    let player_pos = HashSet::new();
    let enemy_pos = HashSet::new();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut game_board: Board = Board {
        width: BOARD_WIDTH,
        height: BOARD_HEIGHT,
        tile_amount_x: TILES_X,
        tile_amount_y: TILES_Y,
        enemy_pos,
        player_pos,
        obstacles: HashSet::new(),
        active: false,
        selected_piece_type: TileType::Obstacle,
    };

    let video_subsystem = sdl_context.video().unwrap();
    video_subsystem.text_input().stop();
    let window = video_subsystem
        .window("PathMaker demo", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();

    let mut board_control_components: [Button; 6] = [
        upload_map_button.clone(),
        save_map_button.clone(),
        start_board_button.clone(),
        select_piece_one.clone(),
        select_piece_two.clone(),
        select_piece_three.clone(),
    ];
    canvas.set_draw_color(Color::RGB(87, 87, 81));
    canvas.clear();
    game_board.draw(&mut canvas, &texture_creator, None);
    'running: loop {
        let mouse_state: sdl2::mouse::MouseState = sdl2::mouse::MouseState::new(&event_pump);
        let (mouse_x, mouse_y) = (mouse_state.x(), mouse_state.y());
        /*-------- User UI -------- */

        /*-------- Updates User UI Depending on State -------- */

        if game_board.active {
            game_board.draw(&mut canvas, &texture_creator, None);
        }

        if select_file {
            /*------- File Selection Menu -------*/
            save_map_button.active = false;
            upload_map_button.active = false;
            select_piece_one.active = false;
            select_piece_two.active = false;
            select_piece_three.active = false;
            start_board_button.active = false;
            go_back_button.active = true;
            if components_drawn {
                if search_text_length == 0 {
                    directory_buttons
                        .iter_mut()
                        .filter(|a| a.mouse_over_component((mouse_x, mouse_y)) || a.hover)
                        .for_each(|a| {
                            a.active = true;
                            a.hover = match a.mouse_over_component((mouse_x, mouse_y)) {
                                true => true,
                                false => !a.hover,
                            };
                            a.draw(&mut canvas, &texture_creator, Some(&mouse_state));
                        });
                }
                if search_file.text.trim().len() != search_text_length {
                    search_text_length = search_file.text.len();
                    canvas.set_draw_color(Color::RGB(87, 87, 81));
                    canvas.fill_rect(Rect::new(800, 0, 1000, 1000)).unwrap();
                    search_file.draw(&mut canvas, &texture_creator, None);
                    directory_buttons.iter_mut().for_each(|a| a.active = false);
                    directory_buttons
                        .iter_mut()
                        .filter(|a| a.text.contains(&search_file.text.trim()))
                        .enumerate()
                        .filter(|(count, _)| *count < 700 / 24)
                        .for_each(|(count, a)| {
                            a.active = true;
                            a.location = Point::new(1000, 62 + count as i32 * 25);
                            a.draw(&mut canvas, &texture_creator, Some(&mouse_state))
                        });
                    go_back_button.draw(&mut canvas, &texture_creator, Some(&mouse_state));
                } else if search_file.text.trim().len() == search_text_length {
                    directory_buttons
                        .iter_mut()
                        .filter(|a| a.text.contains(&search_file.text.trim()))
                        .filter(|a| a.mouse_over_component((mouse_x, mouse_y)) || a.hover)
                        .for_each(|a| {
                            a.hover = match a.mouse_over_component((mouse_x, mouse_y)) {
                                true => true,
                                false => !a.hover,
                            };
                            a.draw(&mut canvas, &texture_creator, Some(&mouse_state))
                        });
                }
                if go_back_button.mouse_over_component((mouse_x, mouse_y)) || go_back_button.hover {
                    go_back_button.hover =
                        match go_back_button.mouse_over_component((mouse_x, mouse_y)) {
                            true => true,
                            false => !go_back_button.hover,
                        };
                    go_back_button.draw(&mut canvas, &texture_creator, Some(&mouse_state));
                }
                if search_file.active {
                    search_file.draw(&mut canvas, &texture_creator, None);
                }
            } else {
                search_file.draw(&mut canvas, &texture_creator, None);
                directory_buttons
                    .iter_mut()
                    .enumerate()
                    .filter(|(count, _)| *count < 700 / 24)
                    .for_each(|(count, a)| {
                        a.location = Point::new(1000, 62 + count as i32 * 25);
                        a.draw(&mut canvas, &texture_creator, Some(&mouse_state))
                    });
                go_back_button.draw(&mut canvas, &texture_creator, Some(&mouse_state));
                components_drawn = true;
            }

            /*------- File Selection Menu -------*/
        } else {
            /*------ Board Editing Components ------*/
            if components_drawn {
                directory_buttons.iter_mut().for_each(|a| a.active = false);
                save_map_button.active = true;
                upload_map_button.active = true;
                select_piece_one.active = true;
                select_piece_two.active = true;
                select_piece_three.active = true;
                start_board_button.active = true;

                board_control_components
                    .iter_mut()
                    .filter(|a| a.mouse_over_component((mouse_x, mouse_y)) || a.hover)
                    .for_each(|a| {
                        match a.mouse_over_component((mouse_x, mouse_y)) {
                            true => a.change_mode(true),
                            false => a.change_mode(!a.hover),
                        };
                        a.draw(&mut canvas, &texture_creator, Some(&mouse_state));
                    });
            } else {
                board_control_components
                    .iter_mut()
                    .for_each(|a| a.draw(&mut canvas, &texture_creator, Some(&mouse_state)));
                components_drawn = true;
            }
        }
        /*------ Board Editing Components ------*/

        /*-------- Updates User UI Depending on State --------*/

        /*-------- Handle Component Inputs --------*/
        if mouse_state.left() {
            if game_board.on_click(&mouse_state) {
                game_board.draw(&mut canvas, &texture_creator, None);
            } else if start_board_button.on_click(&mouse_state) {
                game_board.active = true;
            } else if upload_map_button.on_click(&mouse_state) {
                canvas.set_draw_color(Color::RGB(87, 87, 81));
                canvas.clear();
                game_board.draw(&mut canvas, &texture_creator, None);
                select_file = true;
                components_drawn = false;
            } else if save_map_button.on_click(&mouse_state) {
                fileDialog::save_file(game_board.map_json());
            } else if select_piece_one.on_click(&mouse_state) {
                game_board.selected_piece_type = TileType::Player
            } else if select_piece_two.on_click(&mouse_state) {
                game_board.selected_piece_type = TileType::Enemy
            } else if select_piece_three.on_click(&mouse_state) {
                game_board.selected_piece_type = TileType::Obstacle
            } else if search_file.on_click(&mouse_state) {
                video_subsystem.text_input().start();
                search_file.active = true;
            } else if go_back_button.on_click(&mouse_state) {
                search_file.text = "".to_string();
                search_file.active = false;
                canvas.set_draw_color(Color::RGB(87, 87, 81));
                canvas.clear();
                game_board.draw(&mut canvas, &texture_creator, None);
                select_file = false;
                components_drawn = false;
            } else {
                match directory_buttons
                    .iter_mut()
                    .find(|a| a.on_click(&mouse_state))
                {
                    Some(value) => {
                        let file = fileDialog::read_file(
                            directory_tree.get(&value.text).expect("File doesn't Exist"),
                        );
                        let (obstacle_map, player_map, enemy_map, tile_amount_x, tile_amount_y) =
                            fileDialog::parse_map_file(file);
                        println!("{:#?}", player_map);
                        game_board = Board {
                            width: BOARD_WIDTH,
                            height: BOARD_HEIGHT,
                            tile_amount_x: tile_amount_x,
                            tile_amount_y: tile_amount_y,
                            enemy_pos: enemy_map,
                            player_pos: player_map,
                            obstacles: obstacle_map,
                            active: false,
                            selected_piece_type: TileType::Obstacle,
                        };

                        game_board.draw(&mut canvas, &texture_creator, None);
                    }

                    None => {}
                };
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
