use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::{clear_background, set_fullscreen, vec2, WHITE};
use macroquad::ui::root_ui;

use server::action::Action;
use server::game::Game;
use server::position::Position;
use server::status_phase::StatusPhaseAction;

use crate::advance_ui::{pay_advance_dialog, show_advance_menu, show_free_advance_menu};
use crate::client_state::{ActiveDialog, PendingUpdate, State, StateUpdate, StateUpdates};
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
use crate::{combat_ui, influence_ui, move_ui, recruit_unit_ui, status_phase_ui};

pub async fn init() -> State {
    let state = State::new().await;

    set_fullscreen(true);
    state
}

pub fn render_and_update(
    game: &Game,
    state: &mut State,
    sync_result: &GameSyncResult,
    features: &Features,
) -> GameSyncRequest {
    match sync_result {
        GameSyncResult::None => {}
        GameSyncResult::Update => {
            state.update_from_game(game);
        }
        GameSyncResult::WaitingForUpdate => {
            state.set_dialog(ActiveDialog::WaitingForUpdate);
        }
    }

    let update = render(game, state, features);
    state.update(game, update)
}

fn render(game: &Game, state: &State, features: &Features) -> StateUpdate {
    let player_index = game.active_player();
    clear_background(WHITE);

    draw_map(game, state);
    let mut updates = StateUpdates::new();
    show_globals(game);
    show_resources(game, player_index);
    show_wonders(game, player_index);

    if root_ui().button(vec2(1200., 130.), "Log") {
        return StateUpdate::OpenDialog(ActiveDialog::Log);
    };
    if root_ui().button(vec2(1200., 100.), "Advances") {
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    };
    if features.import_export {
        if root_ui().button(vec2(1200., 290.), "Import") {
            return StateUpdate::Import;
        };
        if root_ui().button(vec2(1250., 290.), "Export") {
            return StateUpdate::Export;
        };
    }

    if let Some(u) = &state.pending_update {
        updates.add(show_pending_update(u));
        return updates.result();
    }

    if game.state == server::game::GameState::Playing {
        updates.add(show_increase_happiness(game, player_index));
    }
    updates.add(show_global_controls(game, state));

    updates.add(match &state.active_dialog {
        ActiveDialog::None => StateUpdate::None,
        ActiveDialog::Log => show_log(game),
        ActiveDialog::TileMenu(p) => show_tile_menu(game, *p),
        ActiveDialog::WaitingForUpdate => {
            active_dialog_window("Waiting for update", |_ui| StateUpdate::None)
        }

        // playing actions
        ActiveDialog::IncreaseHappiness(h) => increase_happiness_menu(h),
        ActiveDialog::AdvanceMenu => show_advance_menu(game, player_index),
        ActiveDialog::AdvancePayment(p) => pay_advance_dialog(p),
        ActiveDialog::ConstructionPayment(p) => pay_construction_dialog(game, p),
        ActiveDialog::CollectResources(c) => collect_resources_dialog(game, c),
        ActiveDialog::RecruitUnitSelection(s) => recruit_unit_ui::select_dialog(game, s),
        ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::replace_dialog(game, r),
        ActiveDialog::MoveUnits(s) => move_ui::move_units_dialog(game, s),
        ActiveDialog::CulturalInfluenceResolution(c) => {
            influence_ui::cultural_influence_resolution_dialog(c)
        }

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

fn show_pending_update(update: &PendingUpdate) -> StateUpdate {
    active_dialog_window("Are you sure?", |ui| {
        ui.label(None, &format!("Warning: {}", update.warning.join(", ")));
        if ui.button(None, "OK") {
            return StateUpdate::ResolvePendingUpdate(true);
        }
        if ui.button(None, "Cancel") {
            return StateUpdate::ResolvePendingUpdate(false);
        }
        StateUpdate::None
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

pub struct Features {
    pub import_export: bool,
}

pub enum GameSyncRequest {
    None,
    ExecuteAction(Action),
    Import,
    Export,
}

pub enum GameSyncResult {
    None,
    Update,
    WaitingForUpdate,
}
