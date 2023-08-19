use crate::advance_ui::{pay_advance_dialog, show_advance_menu};
use crate::city_ui;
use crate::city_ui::show_city_menu;
use crate::collect_ui::{click_collect_option, collect_resources_dialog};
use crate::construct_ui::pay_construction_dialog;
use crate::dialog_ui::active_dialog_window;
use crate::happiness_ui::show_increase_happiness;
use crate::hex_ui::pixel_to_coordinate;
use crate::log_ui::show_log;
use crate::map_ui::draw_map;
use crate::player_ui::{show_global_controls, show_globals, show_resources, show_wonders};
use crate::ui_state::{ActiveDialog, CityMenu, State, StateUpdate, StateUpdates};
use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::{clear_background, next_frame, set_fullscreen, WHITE};
use server::game::Game;
use server::position::Position;

pub async fn run(game: &mut Game) {
    let mut state = State::new();

    set_fullscreen(true);
    loop {
        let update = game_loop(game, &state);
        state.update(game, update);
        next_frame().await;
    }
}

fn game_loop(game: &Game, state: &State) -> StateUpdate {
    let player_index = game.current_player_index;
    clear_background(WHITE);

    draw_map(game, state);
    let mut updates = StateUpdates::new();
    updates.add(show_advance_menu(game, player_index));
    show_globals(game);
    show_log(game);
    show_resources(game, player_index);
    show_wonders(game, player_index);

    if state.pending_update.is_some() {
        updates.add(show_pending_update(state));
        return updates.result();
    }

    updates.add(show_increase_happiness(game, player_index, state));
    updates.add(show_global_controls(game));

    if let Some((city_owner_index, city_position)) = state.focused_city {
        updates.add(show_city_menu(
            game,
            &CityMenu::new(player_index, city_owner_index, city_position),
        ));
    }

    updates.add(match &state.active_dialog {
        ActiveDialog::AdvancePayment(p) => pay_advance_dialog(p),
        ActiveDialog::ConstructionPayment(p) => pay_construction_dialog(game, p),
        ActiveDialog::CollectResources(c) => collect_resources_dialog(game, c),
        ActiveDialog::None => StateUpdate::None,
    });

    updates.add(try_click(game, state));

    updates.result()
}

#[must_use]
fn show_pending_update(state: &State) -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        if let Some(update) = &state.pending_update {
            ui.label(None, &format!("Warning: {}", update.warning.join(", ")));
            if ui.button(None, "OK") {
                updates.add(StateUpdate::ResolvePendingUpdate(true));
            }
            if ui.button(None, "Cancel") {
                updates.add(StateUpdate::ResolvePendingUpdate(false));
            }
        }
    });
    updates.result()
}

pub fn try_click(game: &Game, state: &State) -> StateUpdate {
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();

        let pos = Position::from_coordinate(pixel_to_coordinate(x, y));

        match &state.active_dialog {
            ActiveDialog::CollectResources(col) => return click_collect_option(col, pos),
            _ => {
                if let Some(c) = game.get_any_city(pos) {
                    return city_ui::city_click(state, game.get_player(c.player_index), c);
                }
            }
        }
    }
    StateUpdate::None
}
