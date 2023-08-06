use server::game::Game;
use macroquad::prelude::{clear_background, next_frame, set_fullscreen};
use macroquad::color::GREEN;
use crate::advance_ui::{pay_advance_dialog, show_advance_menu};
use crate::city_ui::{CityMenu, pay_construction_dialog, show_city_menu, try_city_click};
use crate::log_ui::show_log;
use crate::map_ui::draw_map;
use crate::player_ui::{show_global_controls, show_globals, show_increase_happiness, show_resources};
use crate::ui::{ActiveDialog, State};

pub async fn run(game: &mut Game) {
    let mut state = State::new();

    set_fullscreen(true);
    loop {
        game_loop(game, &mut state);
        next_frame().await
    }
}

fn game_loop(game: &mut Game, state: &mut State) {
    let player_index = &game.current_player_index.clone();
    clear_background(GREEN);

    draw_map(game, state);
    show_advance_menu(game, player_index, state);
    show_globals(game);
    show_log(game);
    show_resources(game, player_index);
    show_increase_happiness(game, player_index, state);
    show_global_controls(game, player_index, state);

    if let Some((city_owner_index, city_position)) = &state.focused_city {
        let dialog = show_city_menu(game, CityMenu::new(player_index, city_owner_index, city_position));
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
        ActiveDialog::None => {}
    }

    try_city_click(game, state);
}
