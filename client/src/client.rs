use macroquad::input::{MouseButton, is_mouse_button_pressed};
use macroquad::prelude::clear_background;
use macroquad::prelude::*;

use server::action::Action;
use server::game::Game;
use server::position::Position;

use crate::advance_ui::{pay_advance_dialog, show_paid_advance_menu};
use crate::cards_ui::show_cards;
use crate::client_state::{
    ActiveDialog, CameraMode, DialogChooser, State, StateUpdate, StateUpdates,
};
use crate::collect_ui::collect_dialog;
use crate::construct_ui::pay_construction_dialog;
use crate::event_ui::{custom_phase_event_origin, event_help};
use crate::happiness_ui::{increase_happiness_click, increase_happiness_menu};
use crate::hex_ui::pixel_to_coordinate;
use crate::layout_ui::{
    ICON_SIZE, bottom_center_anchor, bottom_centered_text_with_offset,
    draw_scaled_icon_with_tooltip, icon_pos, top_right_texture,
};
use crate::log_ui::show_log;
use crate::map_ui::{draw_map, explore_dialog, show_tile_menu};
use crate::player_ui::{player_select, show_global_controls, show_top_center, show_top_left};
use crate::render_context::RenderContext;
use crate::unit_ui::unit_selection_click;
use crate::{
    cards_ui, custom_phase_ui, dialog_ui, map_ui, move_ui, recruit_unit_ui, status_phase_ui,
    tooltip,
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
    let w = state.screen_size.x - 530.;
    let h = state.screen_size.y - 170.;
    state.camera.viewport = Some((270, 90, w as i32, h as i32));

    state.camera.zoom.y = state.camera.zoom.x * w / h;
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
        updates.add(show_cards(rc));
        updates.add(player_select(rc));
        updates.add(show_global_controls(rc, features));
    }

    if top_right_texture(rc, &rc.assets().log, icon_pos(-1, 0), "Show log") {
        if let ActiveDialog::Log = state.active_dialog {
            return StateUpdate::CloseDialog;
        }
        return StateUpdate::OpenDialog(ActiveDialog::Log);
    }
    if top_right_texture(rc, &rc.assets().advances, icon_pos(-2, 0), "Show advances") {
        if state.active_dialog.is_advance() {
            return StateUpdate::CloseDialog;
        }
        return StateUpdate::OpenDialog(ActiveDialog::AdvanceMenu);
    }
    if top_right_texture(
        rc,
        &rc.assets().show_permanent_effects,
        icon_pos(-3, 0),
        if state.show_permanent_effects {
            "Hide permanent effects"
        } else {
            "Show permanent effects"
        },
    ) {
        return StateUpdate::ToggleShowPermanentEffects;
    }

    let can_control = rc.can_control_shown_player();
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
    updates.add(rc.with_camera(CameraMode::World, try_click));
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
        ActiveDialog::None | ActiveDialog::WaitingForUpdate => StateUpdate::None,
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
        ActiveDialog::ExploreResolution(r) => explore_dialog(rc, r),
        ActiveDialog::MoveUnits(_) => move_ui::move_units_dialog(rc),
        ActiveDialog::MovePayment(p) => move_ui::move_payment_dialog(rc, p),

        //status phase
        ActiveDialog::ChangeGovernmentType => status_phase_ui::change_government_type_dialog(rc),
        ActiveDialog::ChooseAdditionalAdvances(a) => {
            status_phase_ui::choose_additional_advances_dialog(rc, a)
        }
        ActiveDialog::PaymentRequest(c) => custom_phase_ui::custom_phase_payment_dialog(rc, c),
        ActiveDialog::PlayerRequest(r) => custom_phase_ui::player_request_dialog(rc, r),
        ActiveDialog::ResourceRewardRequest(p) => custom_phase_ui::payment_reward_dialog(rc, p),
        ActiveDialog::AdvanceRequest(r) => custom_phase_ui::advance_reward_dialog(
            rc,
            r,
            &custom_phase_event_origin(rc).name(rc.game),
        ),
        ActiveDialog::UnitTypeRequest(r) => custom_phase_ui::unit_request_dialog(rc, r),
        ActiveDialog::UnitsRequest(r) => custom_phase_ui::select_units_dialog(rc, r),
        ActiveDialog::StructuresRequest(d, r) => {
            custom_phase_ui::select_structures_dialog(rc, d.into(), r)
        }
        ActiveDialog::BoolRequest(d) => custom_phase_ui::bool_request_dialog(rc, d),
        ActiveDialog::PositionRequest(r) => custom_phase_ui::position_request_dialog(rc, r),
        ActiveDialog::HandCardsRequest(r) => cards_ui::select_cards_dialog(rc, r),
    }
}

fn dialog_chooser(rc: &RenderContext, c: &DialogChooser) -> StateUpdate {
    let h = -50.;
    bottom_centered_text_with_offset(
        rc,
        &c.title,
        vec2(0., c.options.len() as f32 * h + 50.),
        &[],
    );

    for (i, (origin, d)) in c.options.iter().enumerate() {
        let offset = vec2(0., i as f32 * h + 35.);
        let (name, tooltip) = origin.as_ref().map_or_else(
            || ("standard action".to_string(), vec![]),
            |o| (o.name(rc.game), event_help(rc, o)),
        );

        bottom_centered_text_with_offset(rc, &name, offset, &tooltip);
        if draw_scaled_icon_with_tooltip(
            rc,
            &rc.assets().ok,
            &tooltip,
            bottom_center_anchor(rc) + offset + vec2(100., -70.),
            ICON_SIZE,
        ) {
            return StateUpdate::OpenDialog(d.clone());
        }
    }
    StateUpdate::None
}

pub fn try_click(rc: &RenderContext) -> StateUpdate {
    let game = rc.game;
    let mouse_pos = rc.mouse_pos();
    let pos = Position::from_coordinate(pixel_to_coordinate(mouse_pos));

    if !game.map.tiles.contains_key(&pos) {
        return StateUpdate::None;
    }

    if !is_mouse_button_pressed(MouseButton::Left) {
        return StateUpdate::None;
    }

    if rc.can_control_shown_player() {
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
        ActiveDialog::PositionRequest(r) => {
            StateUpdate::OpenDialog(ActiveDialog::PositionRequest(r.clone().toggle(pos)))
        }
        ActiveDialog::UnitsRequest(s) => unit_selection_click(rc, pos, mouse_pos, s, |new| {
            StateUpdate::OpenDialog(ActiveDialog::UnitsRequest(new.clone()))
        }),
        ActiveDialog::IncreaseHappiness(h) if h.city_restriction.is_none_or(|r| r == pos) => {
            increase_happiness_click(rc, pos, h)
        }
        _ => StateUpdate::SetFocusedTile(pos),
    }
}

pub struct Features {
    pub import_export: bool,
    pub assets_url: String,
    pub ai: bool,
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
    StartAutoplay,
    Import,
    Export,
}

pub enum GameSyncResult {
    None,
    Update,
    WaitingForUpdate,
}
