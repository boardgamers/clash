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
use crate::ui_state::{ActiveDialog, ActiveDialogUpdate, CityMenu, State};
use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::{clear_background, next_frame, set_fullscreen, WHITE};
use server::game::Game;
use server::position::Position;
use crate::dialog_ui::active_dialog_window;

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

    if state.pending_update.is_some() {
        show_pending_update(game, state, player_index);
        return;
    }

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

    let update = match &mut state.active_dialog {
        ActiveDialog::AdvancePayment(p) => pay_advance_dialog(p),
        ActiveDialog::ConstructionPayment(p) => pay_construction_dialog(p),
        ActiveDialog::CollectResources(c) => collect_resources_dialog(game, c),
        ActiveDialog::None => ActiveDialogUpdate::None,
    };
    state.update(game, update);

    try_click(game, state);
}

fn show_pending_update(game: &mut Game, state: &mut State, player_index: usize) {
    active_dialog_window(|ui| {
        if let Some(update) = &state.pending_update {
            ui.label(None, &format!("Warning: {}", update.warning));
            if ui.button(None, "OK") {
                game.execute_action(state.pending_update.take().unwrap().action, player_index);
                state.active_dialog = ActiveDialog::None;
                state.pending_update = None;
            }
            if ui.button(None, "Cancel") {
                state.pending_update = None;
            }
        }
    });
}

pub fn try_click(game: &Game, state: &mut State) {
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();

        let pos = &Position::from_coordinate(pixel_to_coordinate(x, y));

        match &mut state.active_dialog {
            ActiveDialog::CollectResources(col) => click_collect_option(col, pos),
            _ => {
                if let Some(c) = game.get_any_city(pos) {
                    city_ui::city_click(state, game.get_player(c.player_index), c);
                }
            }
        }
    }
}
