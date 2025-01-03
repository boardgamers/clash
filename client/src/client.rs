use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::*;
use macroquad::prelude::{clear_background, vec2};
use macroquad::ui::root_ui;

use server::action::Action;
use server::game::Game;
use server::position::Position;

use crate::advance_ui::{pay_advance_dialog, show_advance_menu, show_free_advance_menu};
use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate, StateUpdates};
use crate::collect_ui::collect_resources_dialog;
use crate::construct_ui::pay_construction_dialog;
use crate::happiness_ui::{increase_happiness_click, increase_happiness_menu};
use crate::hex_ui::pixel_to_coordinate;
use crate::log_ui::show_log;
use crate::map_ui::{draw_map, show_tile_menu};
use crate::player_ui::{player_select, show_global_controls, show_top_center, show_top_left};
use crate::status_phase_ui::raze_city_confirm_dialog;
use crate::unit_ui::unit_selection_click;
use crate::{
    combat_ui, dialog_ui, influence_ui, move_ui, recruit_unit_ui, status_phase_ui, tooltip,
};

pub async fn init(features: &Features) -> State {
    let state = State::new(features).await;
    root_ui().push_skin(&state.assets.skin);
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

fn render(game: &Game, state: &mut State, features: &Features) -> StateUpdate {
    tooltip::update(state);

    clear_background(WHITE);

    let player = &state.shown_player(game);

    let s = state.screen_size;

    state.camera = Camera2D {
        zoom: vec2(state.zoom, state.zoom * s.x / s.y),
        offset: state.offset,
        ..Default::default()
    };

    let mut updates = StateUpdates::new();
    updates.add(draw_map(game, state));
    show_top_left(game, player, state);
    show_top_center(game, player, state);
    updates.add(player_select(game, player, state));
    updates.add(show_global_controls(game, state, features));

    if player.can_control {
        if let Some(u) = &state.pending_update {
            updates.add(dialog_ui::show_pending_update(u, player));
            return updates.result();
        }
    }

    if player.can_control || state.active_dialog.show_for_other_player() {
        updates.add(render_active_dialog(game, state, player));
    }

    if player.can_control {
        updates.add(try_click(game, state, player));
    }
    updates.result()
}

fn render_active_dialog(game: &Game, state: &mut State, player: &ShownPlayer) -> StateUpdate {
    match &state.active_dialog {
        ActiveDialog::None
        | ActiveDialog::MoveUnits(_)
        | ActiveDialog::WaitingForUpdate
        | ActiveDialog::CulturalInfluence
        | ActiveDialog::PlaceSettler => StateUpdate::None,
        ActiveDialog::Log => show_log(game, player),
        ActiveDialog::TileMenu(p) => show_tile_menu(game, *p, player, state),

        // playing actions
        ActiveDialog::IncreaseHappiness(h) => increase_happiness_menu(h, player, state, game),
        ActiveDialog::AdvanceMenu => show_advance_menu(game, player),
        ActiveDialog::AdvancePayment(p) => pay_advance_dialog(p, player, game, state),
        ActiveDialog::ConstructionPayment(p) => pay_construction_dialog(game, p, state),
        ActiveDialog::CollectResources(c) => collect_resources_dialog(game, c, state),
        ActiveDialog::RecruitUnitSelection(s) => {
            recruit_unit_ui::select_dialog(game, s, player, state)
        }
        ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::replace_dialog(game, r, state),
        ActiveDialog::CulturalInfluenceResolution(c) => {
            influence_ui::cultural_influence_resolution_dialog(c, player)
        }

        //status phase
        ActiveDialog::FreeAdvance => show_free_advance_menu(game, player),
        ActiveDialog::RazeSize1City => status_phase_ui::raze_city_dialog(state),
        ActiveDialog::CompleteObjectives => status_phase_ui::complete_objectives_dialog(player),
        ActiveDialog::DetermineFirstPlayer => {
            status_phase_ui::determine_first_player_dialog(game, player)
        }
        ActiveDialog::ChangeGovernmentType => {
            status_phase_ui::change_government_type_dialog(game, player)
        }
        ActiveDialog::ChooseAdditionalAdvances(a) => {
            status_phase_ui::choose_additional_advances_dialog(game, a, state)
        }

        //combat
        ActiveDialog::PlayActionCard => combat_ui::play_action_card_dialog(player),
        ActiveDialog::Retreat => combat_ui::retreat_dialog(player),
        ActiveDialog::RemoveCasualties(s) => combat_ui::remove_casualties_dialog(game, s, state),
    }
}

pub fn try_click(game: &Game, state: &mut State, player: &ShownPlayer) -> StateUpdate {
    let (x, y) = mouse_position();
    let mouse_pos = state.camera.screen_to_world(vec2(x, y));
    let pos = Position::from_coordinate(pixel_to_coordinate(mouse_pos));
    if !game.map.tiles.contains_key(&pos) {
        return StateUpdate::None;
    }

    if let ActiveDialog::CulturalInfluence = state.active_dialog {
        return influence_ui::hover(pos, game, player, mouse_pos, state);
    }

    if !is_mouse_button_pressed(MouseButton::Left) {
        return StateUpdate::None;
    }

    match &state.active_dialog {
        ActiveDialog::CollectResources(_) => StateUpdate::None,
        ActiveDialog::MoveUnits(s) => move_ui::click(pos, s, mouse_pos, game),
        ActiveDialog::RemoveCasualties(s) => {
            unit_selection_click(game, player, pos, mouse_pos, s, |new| {
                StateUpdate::SetDialog(ActiveDialog::RemoveCasualties(new.clone()))
            })
        }
        ActiveDialog::ReplaceUnits(s) => {
            unit_selection_click(game, player, pos, mouse_pos, s, |new| {
                StateUpdate::SetDialog(ActiveDialog::ReplaceUnits(new.clone()))
            })
        }
        ActiveDialog::RazeSize1City => raze_city_confirm_dialog(game, player, pos),
        ActiveDialog::PlaceSettler => {
            if player.get(game).get_city(pos).is_some() {
                StateUpdate::Execute(Action::PlaceSettler(pos))
            } else {
                StateUpdate::None
            }
        }
        ActiveDialog::IncreaseHappiness(h) => increase_happiness_click(game, player, pos, h),
        _ => StateUpdate::OpenDialog(ActiveDialog::TileMenu(pos)),
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
