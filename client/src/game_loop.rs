use std::fs::File;
use std::io::BufReader;

use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::{clear_background, next_frame, set_fullscreen, vec2, WHITE};
use macroquad::ui::root_ui;

use server::action::Action;
use server::game::{Game, GameData};
use server::position::Position;
use server::status_phase::StatusPhaseAction;

use crate::advance_ui::{pay_advance_dialog, show_advance_menu, show_free_advance_menu};
use crate::collect_ui::{click_collect_option, collect_resources_dialog};
use crate::construct_ui::pay_construction_dialog;
use crate::dialog_ui::active_dialog_window;
use crate::happiness_ui::{
    add_increase_happiness, increase_happiness_menu, show_increase_happiness,
};
use crate::hex_ui::pixel_to_coordinate;
use crate::log_ui::show_log;
use crate::map_ui::{draw_map, show_tile_menu};
use crate::player_ui::{show_global_controls, show_globals, show_resources, show_wonders};
use crate::ui_state::{ActiveDialog, State, StateUpdate, StateUpdates};
use crate::{combat_ui, move_ui, recruit_unit_ui, status_phase_ui};

const EXPORT_FILE: &str = "game.json";

pub async fn run(game: &mut Game) {
    let mut state = State::new().await;

    set_fullscreen(true);
    loop {
        let update = game_loop(game, &state);
        state.update(game, update);

        next_frame().await;
    }
}

fn game_loop(game: &mut Game, state: &State) -> StateUpdate {
    let player_index = game.active_player();
    clear_background(WHITE);

    draw_map(game, state);
    let mut updates = StateUpdates::new();
    show_globals(game);
    show_log(game);
    show_resources(game, player_index);
    show_wonders(game, player_index);

    if root_ui().button(vec2(1200., 350.), "Advances") {
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    };
    if root_ui().button(vec2(1200., 450.), "Import") {
        import(game);
        return StateUpdate::Cancel;
    };
    if root_ui().button(vec2(1250., 450.), "Export") {
        export(game);
        return StateUpdate::None;
    };

    if state.pending_update.is_some() {
        updates.add(show_pending_update(state));
        return updates.result();
    }

    if game.state == server::game::GameState::Playing {
        updates.add(show_increase_happiness(game, player_index));
    }
    updates.add(show_global_controls(game, state));

    updates.add(match &state.active_dialog {
        ActiveDialog::None => StateUpdate::None,
        ActiveDialog::IncreaseHappiness(h) => increase_happiness_menu(h),
        ActiveDialog::TileMenu(p) => show_tile_menu(game, *p),
        ActiveDialog::AdvanceMenu => show_advance_menu(game, player_index),
        ActiveDialog::AdvancePayment(p) => pay_advance_dialog(p),
        ActiveDialog::ConstructionPayment(p) => pay_construction_dialog(game, p),
        ActiveDialog::CollectResources(c) => collect_resources_dialog(game, c),
        ActiveDialog::RecruitUnitSelection(s) => recruit_unit_ui::select_dialog(game, s),
        ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::replace_dialog(game, r),
        ActiveDialog::MoveUnits(s) => move_ui::move_units_dialog(game, s),

        //status phase
        ActiveDialog::FreeAdvance => show_free_advance_menu(game, player_index),
        ActiveDialog::RaseSize1City => status_phase_ui::raze_city_dialog(),
        ActiveDialog::DetermineFirstPlayer => status_phase_ui::determine_first_player_dialog(game),
        ActiveDialog::ChangeGovernmentType => status_phase_ui::change_government_type_dialog(game),
        ActiveDialog::ChooseAdditionalAdvances(a) => {
            status_phase_ui::choose_additional_advances_dialog(game, a)
        }

        //combat
        ActiveDialog::PlaceSettler => combat_ui::place_settler_dialog(),
        ActiveDialog::Retreat => combat_ui::retreat_dialog(),
        ActiveDialog::RemoveCasualties(s) => combat_ui::remove_casualties_dialog(game, s),
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
        File::create(EXPORT_FILE).expect("Failed to create export file"),
        &game.cloned_data(),
    )
    .expect("Failed to write export file");
}

fn show_pending_update(state: &State) -> StateUpdate {
    active_dialog_window(|ui, updates| {
        if let Some(update) = &state.pending_update {
            ui.label(None, &format!("Warning: {}", update.warning.join(", ")));
            if ui.button(None, "OK") {
                updates.add(StateUpdate::ResolvePendingUpdate(true));
            }
            if ui.button(None, "Cancel") {
                updates.add(StateUpdate::ResolvePendingUpdate(false));
            }
        }
    })
}

pub fn try_click(game: &Game, state: &State, player_index: usize) -> StateUpdate {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return StateUpdate::None;
    }
    let (x, y) = mouse_position();

    let pos = Position::from_coordinate(pixel_to_coordinate(x, y));
    if game.map.tiles.get(&pos).is_none() {
        return StateUpdate::None;
    }

    match &state.active_dialog {
        ActiveDialog::MoveUnits(s) => move_ui::click(pos, s),
        ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::click_replace(pos, r),
        ActiveDialog::RemoveCasualties(_s) => StateUpdate::None,
        ActiveDialog::CollectResources(col) => click_collect_option(col, pos),
        ActiveDialog::RaseSize1City => {
            if game.players[player_index].can_raze_city(pos) {
                StateUpdate::status_phase(StatusPhaseAction::RaseSize1City(Some(pos)))
            } else {
                StateUpdate::None
            }
        }
        ActiveDialog::PlaceSettler => {
            if game.players[player_index].get_city(pos).is_some() {
                StateUpdate::Execute(Action::PlaceSettler(pos))
            } else {
                StateUpdate::None
            }
        }
        ActiveDialog::IncreaseHappiness(h) => {
            if let Some(city) = game.players[player_index].get_city(pos) {
                StateUpdate::SetDialog(ActiveDialog::IncreaseHappiness(add_increase_happiness(
                    &game.players[player_index],
                    city,
                    pos,
                    h,
                )))
            } else {
                StateUpdate::None
            }
        }
        _ => StateUpdate::OpenDialog(ActiveDialog::TileMenu(pos)),
    }
}
