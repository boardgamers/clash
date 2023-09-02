use crate::advance_ui::{pay_advance_dialog, show_advance_menu};
use crate::city_ui::show_city_menu;
use crate::collect_ui::{click_collect_option, collect_resources_dialog};
use crate::construct_ui::pay_construction_dialog;
use crate::dialog_ui::active_dialog_window;
use crate::happiness_ui::show_increase_happiness;
use crate::hex_ui::pixel_to_coordinate;
use crate::log_ui::show_log;
use crate::map_ui::{draw_map, show_tile_menu};
use crate::player_ui::{show_global_controls, show_globals, show_resources, show_wonders};
use crate::ui_state::{ActiveDialog, CityMenu, FocusedTile, State, StateUpdate, StateUpdates};
use crate::{city_ui, move_ui, recruit_unit_ui};
use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::{clear_background, next_frame, set_fullscreen, vec2, WHITE};
use macroquad::ui::root_ui;
use server::game::{Game, GameData};
use server::position::Position;
use std::fs::File;
use std::io::BufReader;

const EXPORT_FILE: &str = "game.json";

pub async fn run(game: &mut Game) {
    let mut state = State::new();

    set_fullscreen(true);
    loop {
        let update = game_loop(game, &state);
        state.update(game, update);
        next_frame().await;
    }
}

fn game_loop(game: &mut Game, state: &State) -> StateUpdate {
    let player_index = game.current_player_index;
    clear_background(WHITE);

    draw_map(game, state);
    let mut updates = StateUpdates::new();
    updates.add(show_advance_menu(game, player_index));
    show_globals(game);
    show_log(game);
    show_resources(game, player_index);
    show_wonders(game, player_index);

    if root_ui().button(vec2(600., 450.), "Import") {
        import(game);
        return StateUpdate::None;
    };
    if root_ui().button(vec2(650., 450.), "Export") {
        export(game);
        return StateUpdate::None;
    };

    if state.pending_update.is_some() {
        updates.add(show_pending_update(state));
        return updates.result();
    }

    if game.state == server::game::GameState::Playing {
        updates.add(show_increase_happiness(game, player_index, state));
    }
    updates.add(show_global_controls(game, state));

    if let Some(f) = &state.focused_tile {
        if !matches!(state.active_dialog, ActiveDialog::MoveUnits(_)) {
            updates.add(if let Some(p) = f.city_owner_index {
                show_city_menu(game, &CityMenu::new(player_index, p, f.position))
            } else {
                show_tile_menu(game, f.position, None, |_, _| {})
            });
        }
    }

    updates.add(match &state.active_dialog {
        ActiveDialog::None => StateUpdate::None,
        ActiveDialog::AdvancePayment(p) => pay_advance_dialog(p),
        ActiveDialog::ConstructionPayment(p) => pay_construction_dialog(game, p),
        ActiveDialog::CollectResources(c) => collect_resources_dialog(game, c),
        ActiveDialog::RecruitUnitSelection(s) => recruit_unit_ui::select_dialog(game, s),
        ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::replace_dialog(game, r),
        ActiveDialog::MoveUnits(s) => move_ui::move_units_dialog(game, s),
    });

    updates.add(try_click(game, state, player_index));

    updates.result()
}

fn import(game: &mut Game) {
    let file = File::open(EXPORT_FILE).expect("Failed to open export file");
    let reader = BufReader::new(file);
    let data: GameData = serde_json::from_reader(reader).expect("Failed to read export file");
    *game = Game::from_data(data);
}

fn export(game: &Game) {
    serde_json::to_writer_pretty(
        std::fs::File::create(EXPORT_FILE).expect("Failed to create export file"),
        &game.cloned_data(),
    )
    .expect("Failed to write export file");
}

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

pub fn try_click(game: &Game, state: &State, player_index: usize) -> StateUpdate {
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();

        let pos = Position::from_coordinate(pixel_to_coordinate(x, y));

        match &state.active_dialog {
            ActiveDialog::MoveUnits(s) => move_ui::click(pos, s),
            ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::click_replace(pos, r),
            ActiveDialog::CollectResources(col) => click_collect_option(col, pos),
            _ => {
                if let Some(c) = game.get_any_city(pos) {
                    city_ui::city_click(state, game.get_player(player_index), c)
                } else if matches!(state.active_dialog, ActiveDialog::None) {
                    StateUpdate::FocusTile(FocusedTile::new(None, pos))
                } else {
                    StateUpdate::None
                }
            }
        }
    } else {
        StateUpdate::None
    }
}
