use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::*;
use macroquad::prelude::{clear_background, vec2};
use macroquad::ui::root_ui;

use server::action::Action;
use server::game::Game;
use server::position::Position;

use crate::advance_ui::{pay_advance_dialog, show_advance_menu, show_free_advance_menu};
use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate, StateUpdates};
use crate::collect_ui::{click_collect_option, collect_resources_dialog};
use crate::construct_ui::pay_construction_dialog;
use crate::dialog_ui::active_dialog_window;
use crate::happiness_ui::{increase_happiness_dialog, increase_happiness_menu};
use crate::hex_ui::pixel_to_coordinate;
use crate::log_ui::show_log;
use crate::map_ui::{draw_map, show_tile_menu};
use crate::player_ui::{show_global_controls, show_globals};
use crate::status_phase_ui::raze_city_confirm_dialog;
use crate::{combat_ui, dialog_ui, influence_ui, move_ui, recruit_unit_ui, status_phase_ui};

pub async fn init(features: &Features) -> State {
    State::new(features).await
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

fn render(game: &Game, state: &mut State, features: &Features) -> StateUpdate {
    root_ui().push_skin(&state.assets.skin);
    clear_background(BLACK);

    let player = &state.shown_player(game);

    state.camera = Camera2D {
        zoom: vec2(state.zoom, state.zoom * screen_width() / screen_height()),
        offset: state.offset,
        ..Default::default()
    };
    set_camera(&state.camera);

    if matches!(state.active_dialog, ActiveDialog::None) || state.active_dialog.is_map_dialog() {
        draw_map(game, state);
    }
    let mut updates = StateUpdates::new();
    updates.add(show_globals(game, player));
    updates.add(show_global_controls(game, state, features));

    if player.can_control {
        if let Some(u) = &state.pending_update {
            updates.add(dialog_ui::show_pending_update(u, player));
            return updates.result();
        }
    }

    updates.add(match &state.active_dialog {
        ActiveDialog::None => StateUpdate::None,
        ActiveDialog::Log => show_log(game, player),
        ActiveDialog::TileMenu(p) => show_tile_menu(game, *p, player),
        ActiveDialog::WaitingForUpdate => {
            active_dialog_window(player, "Waiting for update", |_ui| StateUpdate::None)
        }

        // playing actions
        ActiveDialog::IncreaseHappiness(h) => increase_happiness_menu(h, player),
        ActiveDialog::AdvanceMenu => show_advance_menu(game, player),
        ActiveDialog::AdvancePayment(p) => pay_advance_dialog(p, player, game),
        ActiveDialog::ConstructionPayment(p) => pay_construction_dialog(game, p, player),
        ActiveDialog::CollectResources(c) => collect_resources_dialog(game, c, player),
        ActiveDialog::RecruitUnitSelection(s) => recruit_unit_ui::select_dialog(game, s, player),
        ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::replace_dialog(game, r, player),
        ActiveDialog::MoveUnits(s) => move_ui::move_units_dialog(game, s, player),
        ActiveDialog::CulturalInfluenceResolution(c) => {
            influence_ui::cultural_influence_resolution_dialog(c, player)
        }

        //status phase
        ActiveDialog::FreeAdvance => show_free_advance_menu(game, player),
        ActiveDialog::RazeSize1City => status_phase_ui::raze_city_dialog(player),
        ActiveDialog::CompleteObjectives => status_phase_ui::complete_objectives_dialog(player),
        ActiveDialog::DetermineFirstPlayer => {
            status_phase_ui::determine_first_player_dialog(game, player)
        }
        ActiveDialog::ChangeGovernmentType => {
            status_phase_ui::change_government_type_dialog(game, player)
        }
        ActiveDialog::ChooseAdditionalAdvances(a) => {
            status_phase_ui::choose_additional_advances_dialog(game, a, player)
        }

        //combat
        ActiveDialog::PlayActionCard => combat_ui::play_action_card_dialog(player),
        ActiveDialog::PlaceSettler => combat_ui::place_settler_dialog(player),
        ActiveDialog::Retreat => combat_ui::retreat_dialog(player),
        ActiveDialog::RemoveCasualties(s) => combat_ui::remove_casualties_dialog(game, s, player),
    });

    updates.add(try_click(game, state, player));

    updates.result()
}

pub fn try_click(game: &Game, state: &State, player: &ShownPlayer) -> StateUpdate {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return StateUpdate::None;
    }
    let (x, y) = mouse_position();
    let pos = Position::from_coordinate(pixel_to_coordinate(
        state.camera.screen_to_world(vec2(x, y)),
    ));
    if !game.map.tiles.contains_key(&pos) {
        return StateUpdate::None;
    }

    if player.can_control {
        match &state.active_dialog {
            ActiveDialog::MoveUnits(s) => move_ui::click(pos, s),
            ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::click_replace(pos, r),
            ActiveDialog::RemoveCasualties(_s) => StateUpdate::None,
            ActiveDialog::CollectResources(col) => click_collect_option(col, pos),
            ActiveDialog::RazeSize1City => raze_city_confirm_dialog(game, player, pos),
            ActiveDialog::PlaceSettler => {
                if player.get(game).get_city(pos).is_some() {
                    StateUpdate::Execute(Action::PlaceSettler(pos))
                } else {
                    StateUpdate::None
                }
            }
            ActiveDialog::IncreaseHappiness(h) => increase_happiness_dialog(game, player, pos, h),
            _ => StateUpdate::OpenDialog(ActiveDialog::TileMenu(pos)),
        }
    } else {
        StateUpdate::OpenDialog(ActiveDialog::TileMenu(pos))
    }
}

pub struct Features {
    pub import_export: bool,
    pub assets_url: String,
}

impl Features {
    #[must_use]
    pub fn get_asset(&self, asset: &str) -> String {
        format!("{}{}", self.assets_url, asset)
    }
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
