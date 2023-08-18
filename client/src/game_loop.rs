use crate::advance_ui::{pay_advance_dialog, show_advance_menu};
use crate::city_ui;
use crate::city_ui::show_city_menu;
use crate::collect_ui::{click_collect_option, collect_resources_dialog};
use crate::construct_ui::pay_construction_dialog;
use crate::happiness_ui::show_increase_happiness;
use crate::hex_ui::pixel_to_coordinate;
use crate::log_ui::show_log;
use crate::map_ui::draw_map;
use crate::player_ui::{show_global_controls, show_globals, show_resources, show_wonders};
use crate::ui_state::{ActiveDialog, CityMenu, State};
use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::{clear_background, next_frame, set_fullscreen, WHITE};
use server::game::Game;
use server::position::Position;

pub async fn run(game: &mut Game) {
    let mut state = State::new();

    set_fullscreen(true);
    loop {
        game_loop(game, &mut state);
        next_frame().await
    }
}

fn game_loop(game: &mut Game, state: &mut State) {
    let player_index = game.current_player_index;
    clear_background(WHITE);

    draw_map(game, state);
    show_advance_menu(game, player_index, state);
    show_globals(game);
    show_log(game);
    show_resources(game, player_index);
    show_wonders(game, player_index);
    show_increase_happiness(game, player_index, state);
    show_global_controls(game, player_index, state);

    if let Some((city_owner_index, city_position)) = state.focused_city.clone() {
        let dialog = show_city_menu(
            game,
            CityMenu::new(player_index, city_owner_index, &city_position),
        );
        if let Some(dialog) = dialog {
            state.active_dialog = dialog;
        }
    }

    match &mut state.active_dialog {
        ActiveDialog::AdvancePayment(p) => {
            if pay_advance_dialog(game, p) {
                state.active_dialog = ActiveDialog::None;
            }
        }
        ActiveDialog::ConstructionPayment(p) => {
            if pay_construction_dialog(game, p) {
                state.active_dialog = ActiveDialog::None;
            }
        }
        ActiveDialog::CollectResources(c) => {
            if collect_resources_dialog(game, c) {
                state.active_dialog = ActiveDialog::None;
            }
        }
        ActiveDialog::None => {}
    }

    try_click(game, state);
}

pub fn try_click(game: &Game, state: &mut State) {
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();

        let c = pixel_to_coordinate(x, y);
        let p = Position::from_coordinate(c);

        match &mut state.active_dialog {
            ActiveDialog::CollectResources(col) => click_collect_option(col, &p),
            _ => {
                for p in game.players.iter() {
                    for city in p.cities.iter() {
                        if c == city.position.coordinate() {
                            city_ui::city_click(state, p, city);
                        };
                    }
                }
            }
        }
    }
}
