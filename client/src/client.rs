use macroquad::input::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::prelude::clear_background;
use macroquad::prelude::*;

use server::action::Action;
use server::content::custom_phase_actions::CustomPhaseEventAction;
use server::game::Game;
use server::position::Position;

use crate::advance_ui::{pay_advance_dialog, show_free_advance_menu, show_paid_advance_menu};
use crate::client_state::{
    ActiveDialog, CameraMode, DialogChooser, State, StateUpdate, StateUpdates,
};
use crate::collect_ui::collect_dialog;
use crate::construct_ui::pay_construction_dialog;
use crate::dialog_ui::{cancel_button, ok_button, OkTooltip};
use crate::event_ui::custom_phase_event_origin;
use crate::happiness_ui::{increase_happiness_click, increase_happiness_menu};
use crate::hex_ui::pixel_to_coordinate;
use crate::layout_ui::{bottom_centered_text, icon_pos, top_right_texture};
use crate::log_ui::show_log;
use crate::map_ui::{draw_map, explore_dialog, show_tile_menu};
use crate::player_ui::{player_select, show_global_controls, show_top_center, show_top_left};
use crate::render_context::RenderContext;
use crate::status_phase_ui::raze_city_confirm_dialog;
use crate::unit_ui::unit_selection_click;
use crate::{
    combat_ui, custom_actions_ui, custom_phase_ui, dialog_ui, influence_ui, map_ui, move_ui,
    recruit_unit_ui, status_phase_ui, tooltip, unit_ui,
};

fn render_with_mutable_state(game: &Game, state: &mut State, features: &Features) -> StateUpdate {
    tooltip::update(state);
    if !state.active_dialog.is_modal() {
        map_ui::pan_and_zoom(state);
    }
    if matches!(state.active_dialog, ActiveDialog::Log) {
        state.log_scroll += mouse_wheel().1;
    }

    set_y_zoom(state);
    render(&state.render_context(game), features)
}

fn set_y_zoom(state: &mut State) {
    let s = state.screen_size;
    state.camera.zoom.y = state.camera.zoom.x * s.x / s.y;
}

fn render(rc: &RenderContext, features: &Features) -> StateUpdate {
    clear_background(WHITE);

    let state = &rc.state;
    let show_map = !state.active_dialog.is_modal();

    let mut updates = StateUpdates::new();
    if show_map {
        updates.add(rc.with_camera(CameraMode::World, draw_map));
    }
    if !state.active_dialog.is_modal() {
        show_top_left(rc);
    }
    if show_map {
        show_top_center(rc);
    }
    if !state.active_dialog.is_modal() {
        updates.add(player_select(rc));
        updates.add(show_global_controls(rc, features));
    }

    if top_right_texture(rc, &rc.assets().log, icon_pos(-1, 0), "Show log") {
        if let ActiveDialog::Log = state.active_dialog {
            return StateUpdate::CloseDialog;
        }
        return StateUpdate::OpenDialog(ActiveDialog::Log);
    };
    if top_right_texture(rc, &rc.assets().advances, icon_pos(-2, 0), "Show advances") {
        if state.active_dialog.is_advance() {
            return StateUpdate::CloseDialog;
        }
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    };

    let can_control = rc.can_control();
    if can_control {
        if let Some(u) = &state.pending_update {
            updates.add(dialog_ui::show_pending_update(u, rc));
        }
    }

    if can_control || state.active_dialog.show_for_other_player() {
        updates.add(render_active_dialog(rc));
    }

    if let Some(pos) = state.focused_tile {
        if matches!(state.active_dialog, ActiveDialog::None) {
            updates.add(show_tile_menu(rc, pos));
        }
    }
    updates.add(try_click(rc));
    updates.result()
}

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

    let update = render_with_mutable_state(game, state, features);
    state.update(game, update)
}

fn render_active_dialog(rc: &RenderContext) -> StateUpdate {
    let state = rc.state;
    match &state.active_dialog {
        ActiveDialog::None
        | ActiveDialog::WaitingForUpdate
        | ActiveDialog::CulturalInfluence(_)
        | ActiveDialog::PositionRequest(_) => StateUpdate::None,
        ActiveDialog::DialogChooser(d) => dialog_chooser(rc, d),
        ActiveDialog::Log => show_log(rc),

        // playing actions
        ActiveDialog::IncreaseHappiness(h) => increase_happiness_menu(rc, h),
        ActiveDialog::AdvanceMenu => show_paid_advance_menu(rc),
        ActiveDialog::AdvancePayment(p) => pay_advance_dialog(p, rc),
        ActiveDialog::ConstructionPayment(p) => pay_construction_dialog(rc, p),
        ActiveDialog::CollectResources(c) => collect_dialog(rc, c),
        ActiveDialog::RecruitUnitSelection(s) => recruit_unit_ui::select_dialog(rc, s),
        ActiveDialog::ReplaceUnits(r) => recruit_unit_ui::replace_dialog(rc, r),
        ActiveDialog::CulturalInfluenceResolution(r) => {
            influence_ui::cultural_influence_resolution_dialog(rc, r)
        }
        ActiveDialog::ExploreResolution(r) => explore_dialog(rc, r),
        ActiveDialog::MoveUnits(_) => move_ui::move_units_dialog(rc),
        ActiveDialog::MovePayment(p) => move_ui::move_payment_dialog(rc, p),

        //status phase
        ActiveDialog::FreeAdvance => show_free_advance_menu(rc),
        ActiveDialog::RazeSize1City => status_phase_ui::raze_city_dialog(rc),
        ActiveDialog::CompleteObjectives => status_phase_ui::complete_objectives_dialog(rc),
        ActiveDialog::ChangeGovernmentType => status_phase_ui::change_government_type_dialog(rc),
        ActiveDialog::ChooseAdditionalAdvances(a) => {
            status_phase_ui::choose_additional_advances_dialog(rc, a)
        }
        ActiveDialog::DetermineFirstPlayer => status_phase_ui::determine_first_player_dialog(rc),
        //combat
        ActiveDialog::PlayActionCard => combat_ui::play_action_card_dialog(rc),
        ActiveDialog::Retreat => combat_ui::retreat_dialog(rc),

        ActiveDialog::Sports((p, pos)) => custom_actions_ui::sports(rc, p, *pos),
        ActiveDialog::Taxes(p) => custom_actions_ui::taxes(rc, p),
        ActiveDialog::Theaters(p) => custom_actions_ui::theaters(rc, p),

        ActiveDialog::PaymentRequest(c) => {
            custom_phase_ui::custom_phase_payment_dialog(rc, c)
        }
        ActiveDialog::ResourceRewardRequest(p) => {
            custom_phase_ui::payment_reward_dialog(rc, p)
        }
        ActiveDialog::AdvanceRewardRequest(r) => {
            custom_phase_ui::advance_reward_dialog(rc, r, &custom_phase_event_origin(rc).name())
        }
        ActiveDialog::UnitTypeRequest(r) => custom_phase_ui::unit_request_dialog(rc, r),
        ActiveDialog::UnitsRequest(r) => custom_phase_ui::select_units_dialog(rc, r),
    }
}

fn dialog_chooser(rc: &RenderContext, c: &DialogChooser) -> StateUpdate {
    bottom_centered_text(rc, &c.title);
    if ok_button(rc, OkTooltip::Valid("OK".to_string())) {
        StateUpdate::OpenDialog(c.yes.clone())
    } else if cancel_button(rc) {
        StateUpdate::OpenDialog(c.no.clone())
    } else {
        StateUpdate::None
    }
}

pub fn try_click(rc: &RenderContext) -> StateUpdate {
    let game = rc.game;
    let state = &rc.state;
    let mouse_pos = state.camera.screen_to_world(mouse_position().into());
    let pos = Position::from_coordinate(pixel_to_coordinate(mouse_pos));

    if rc.can_control() {
        if let ActiveDialog::CulturalInfluence(b) = &state.active_dialog {
            return influence_ui::hover(rc, mouse_pos, b);
        }
    }

    if !game.map.tiles.contains_key(&pos) {
        return StateUpdate::None;
    }

    if !is_mouse_button_pressed(MouseButton::Left) {
        return StateUpdate::None;
    }

    if rc.can_control() {
        let update = controlling_player_click(rc, mouse_pos, pos);
        if !matches!(update, StateUpdate::None) {
            return update;
        }
    }
    StateUpdate::SetFocusedTile(pos)
}

fn controlling_player_click(rc: &RenderContext, mouse_pos: Vec2, pos: Position) -> StateUpdate {
    match &rc.state.active_dialog {
        ActiveDialog::CollectResources(_) => StateUpdate::None,
        ActiveDialog::MoveUnits(s) => move_ui::click(rc, pos, s, mouse_pos),
        ActiveDialog::ReplaceUnits(s) => unit_selection_click(rc, pos, mouse_pos, s, |new| {
            StateUpdate::OpenDialog(ActiveDialog::ReplaceUnits(new.clone()))
        }),
        ActiveDialog::RazeSize1City => raze_city_confirm_dialog(rc, pos),
        ActiveDialog::PositionRequest(r) => {
            if r.choices.contains(&pos) {
                StateUpdate::Execute(Action::CustomPhaseEvent(
                    CustomPhaseEventAction::SelectPosition(pos),
                ))
            } else {
                StateUpdate::None
            }
        }
        ActiveDialog::UnitsRequest(s) => {
            unit_selection_click(rc, pos, mouse_pos, s, |new| {
                StateUpdate::OpenDialog(ActiveDialog::UnitsRequest(new.clone()))
            })
        }
        ActiveDialog::IncreaseHappiness(h) => increase_happiness_click(rc, pos, h),
        _ => StateUpdate::SetFocusedTile(pos),
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
