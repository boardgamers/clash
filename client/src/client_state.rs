use crate::assets::Assets;
use crate::client::{Features, GameSyncRequest};
use crate::collect_ui::CollectResources;
use crate::construct_ui::ConstructionPayment;
use crate::custom_phase_ui::{
    MultiSelection, SelectedStructureInfo, SelectedStructureStatus, UnitsSelection,
};
use crate::dialog_ui::BaseOrCustomDialog;
use crate::event_ui::{custom_phase_event_help, custom_phase_event_origin, event_help, pay_help};
use crate::happiness_ui::IncreaseHappinessConfig;
use crate::layout_ui::{FONT_SIZE, IconBackground};
use crate::log_ui::LogDialog;
use crate::map_ui::ExploreResolutionConfig;
use crate::move_ui::{MoveIntent, MovePayment, MoveSelection};
use crate::payment_ui::{Payment, new_gain};
use crate::recruit_unit_ui::{RecruitAmount, RecruitSelection};
use crate::render_context::{RenderContext, RenderStage};
use crate::status_phase_ui::ChooseAdditionalAdvances;
use macroquad::prelude::*;
use server::action::Action;
use server::advance::Advance;
use server::card::HandCard;
use server::city::{City, MoodState};
use server::content::persistent_events::{
    AdvanceRequest, EventResponse, MultiRequest, PersistentEventRequest, PersistentEventType,
    PlayerRequest, UnitTypeRequest,
};
use server::game::{Game, GameState};
use server::movement::{CurrentMove, MoveDestination};
use server::playing_actions::PlayingActionType;
use server::position::Position;

#[derive(Clone, Debug)]
pub(crate) enum ActiveDialog {
    None,
    Log(LogDialog),
    Info(InfoDialog),
    WaitingForUpdate,
    DialogChooser(Box<DialogChooser>),

    // playing actions
    IncreaseHappiness(IncreaseHappinessConfig),
    AdvanceMenu,
    AdvancePayment(Payment<Advance>),
    ConstructionPayment(ConstructionPayment),
    CollectResources(CollectResources),
    RecruitUnitSelection(RecruitAmount),
    ReplaceUnits(RecruitSelection),
    MoveUnits(MoveSelection),
    MovePayment(MovePayment),

    // event requests
    ExploreResolution(ExploreResolutionConfig),
    ResourceRewardRequest(Payment<String>),
    AdvanceRequest(AdvanceRequest),
    PaymentRequest(Vec<Payment<String>>),
    PlayerRequest(PlayerRequest),
    PositionRequest(MultiSelection<Position>),
    UnitTypeRequest(UnitTypeRequest),
    UnitsRequest(UnitsSelection),
    StructuresRequest(
        Option<BaseOrCustomDialog>,
        MultiSelection<SelectedStructureInfo>,
    ),
    HandCardsRequest(MultiSelection<HandCard>),
    BoolRequest(String),
    ChangeGovernmentType,
    ChooseAdditionalAdvances(ChooseAdditionalAdvances),
}

impl ActiveDialog {
    #[must_use]
    pub(crate) fn help_message(&self, rc: &RenderContext) -> Vec<String> {
        match self {
            ActiveDialog::None
            | ActiveDialog::Log(_)
            | ActiveDialog::Info(_)
            | ActiveDialog::DialogChooser(_)
            | ActiveDialog::AdvanceMenu => vec![],
            ActiveDialog::IncreaseHappiness(h) => {
                vec![
                    h.custom.title.clone(),
                    "Click on a city to increase happiness".to_string(),
                ]
            }
            ActiveDialog::AdvancePayment(p) => pay_help(rc, p),
            ActiveDialog::ConstructionPayment(p) => pay_help(rc, &p.payment),
            ActiveDialog::MovePayment(p) => pay_help(rc, &p.payment),
            ActiveDialog::CollectResources(collect) => collect.help_text(rc),
            ActiveDialog::RecruitUnitSelection(_) => vec!["Click on a unit to recruit".to_string()],
            ActiveDialog::ReplaceUnits(_) => vec!["Click on a unit to replace".to_string()],
            ActiveDialog::MoveUnits(m) => Self::move_units_help(rc, m),
            ActiveDialog::ExploreResolution(_) => {
                vec!["Click on the rotate button to rotate".to_string()]
            }
            ActiveDialog::ChangeGovernmentType => {
                vec!["Click on a government type to change".to_string()]
            }
            ActiveDialog::ChooseAdditionalAdvances(_) => {
                vec!["Click on an advance to choose it".to_string()]
            }
            ActiveDialog::WaitingForUpdate => vec!["Waiting for server update".to_string()],
            ActiveDialog::ResourceRewardRequest(_)
            | ActiveDialog::AdvanceRequest(_)
            | ActiveDialog::PaymentRequest(_) => event_help(rc, &custom_phase_event_origin(rc)),
            ActiveDialog::BoolRequest(d) => custom_phase_event_help(rc, d),
            ActiveDialog::UnitTypeRequest(r) => custom_phase_event_help(rc, &r.description),
            ActiveDialog::UnitsRequest(r) => {
                custom_phase_event_help(rc, &r.selection.request.description)
            }
            ActiveDialog::StructuresRequest(d, r) => {
                if let Some(b) = d {
                    let v = vec!["Click on a building to influence its culture".to_string()];
                    if let PlayingActionType::Custom(c) = &b.action_type {
                        let mut r = v.clone();
                        r.extend(event_help(
                            rc,
                            &rc.shown_player.custom_action_info(*c).event_origin,
                        ));
                    }
                    v
                } else {
                    custom_phase_event_help(rc, &r.request.description)
                }
            }
            ActiveDialog::PositionRequest(r) => custom_phase_event_help(rc, &r.request.description),
            ActiveDialog::HandCardsRequest(r) => {
                custom_phase_event_help(rc, &r.request.description)
            }
            ActiveDialog::PlayerRequest(r) => custom_phase_event_help(rc, &r.description),
        }
    }

    fn move_units_help(rc: &RenderContext, m: &MoveSelection) -> Vec<String> {
        if m.start.is_some() {
            let mut result = vec![];
            let destinations = &m.destinations.list;
            if destinations.is_empty() {
                result.push("No unit on this tile can move".to_string());
            }
            if destinations
                .iter()
                .any(|d| matches!(d, MoveDestination::Tile(_, _)))
            {
                result.push("Click on a highlighted tile to move units".to_string());
            }
            if destinations
                .iter()
                .any(|d| matches!(d, MoveDestination::Carrier(_)))
            {
                result.push("Click on a carrier to embark units".to_string());
            }
            m.destinations.modifiers.iter().for_each(|m| {
                result.extend(event_help(rc, m));
            });
            result
        } else {
            vec!["Click on a unit to move".to_string()]
        }
    }

    #[must_use]
    pub(crate) fn show_for_other_player(&self) -> bool {
        self.is_modal()
    }

    #[must_use]
    pub(crate) fn is_modal(&self) -> bool {
        matches!(self, ActiveDialog::Log(_) | ActiveDialog::Info(_)) || self.is_advance()
    }

    #[must_use]
    pub(crate) fn is_advance(&self) -> bool {
        matches!(
            self,
            ActiveDialog::AdvanceMenu
                | ActiveDialog::AdvancePayment(_)
                | ActiveDialog::ChangeGovernmentType
                | ActiveDialog::ChooseAdditionalAdvances(_)
                | ActiveDialog::AdvanceRequest(_)
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PendingUpdate {
    pub action: Action,
    pub warning: Vec<String>,
    pub info: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct DialogChooser {
    pub title: String,
    pub options: Vec<(Option<EventOrigin>, ActiveDialog)>,
}

#[must_use]
#[derive(Clone, Debug)]
pub(crate) enum StateUpdate {
    OpenDialog(ActiveDialog),
    CloseDialog,
    Cancel,
    ResolvePendingUpdate(bool),
    Execute(Action),
    ExecuteWithWarning(PendingUpdate),
    Import,
    Export,
    SetShownPlayer(usize),
    SetFocusedTile(Position),
    ToggleShowPermanentEffects,
    ToggleAiPlay,
}

pub(crate) type RenderResult = Result<(), Box<StateUpdate>>;

pub(crate) const NO_UPDATE: RenderResult = Ok(());

impl StateUpdate {
    pub(crate) fn of(update: StateUpdate) -> RenderResult {
        Err(Box::new(update))
    }

    pub(crate) fn open_dialog(dialog: ActiveDialog) -> RenderResult {
        Self::of(StateUpdate::OpenDialog(dialog))
    }

    pub(crate) fn close_dialog() -> RenderResult {
        Self::of(StateUpdate::CloseDialog)
    }

    pub(crate) fn execute(action: Action) -> RenderResult {
        Self::of(StateUpdate::Execute(action))
    }

    pub(crate) fn cancel() -> RenderResult {
        Self::of(StateUpdate::Cancel)
    }

    pub(crate) fn set_focused_tile(pos: Position) -> RenderResult {
        Self::of(StateUpdate::SetFocusedTile(pos))
    }

    pub(crate) fn resolve_pending_update(confirm: bool) -> RenderResult {
        Self::of(StateUpdate::ResolvePendingUpdate(confirm))
    }

    pub(crate) fn execute_with_warning(action: Action, warning: Vec<String>) -> RenderResult {
        Self::of(if warning.is_empty() {
            StateUpdate::Execute(action)
        } else {
            StateUpdate::ExecuteWithWarning(PendingUpdate {
                action,
                warning,
                info: vec![],
            })
        })
    }

    pub(crate) fn execute_with_confirm(info: Vec<String>, action: Action) -> RenderResult {
        Self::of(StateUpdate::ExecuteWithWarning(PendingUpdate {
            action,
            warning: vec![],
            info,
        }))
    }

    pub(crate) fn execute_activation(
        action: Action,
        warning: Vec<String>,
        city: &City,
    ) -> RenderResult {
        if city.is_activated() {
            match city.mood_state {
                MoodState::Happy => {
                    let mut warn = vec!["City will become neutral".to_string()];
                    warn.extend(warning);
                    StateUpdate::execute_with_warning(action, warn)
                }
                MoodState::Neutral => {
                    let mut warn = vec!["City will become angry".to_string()];
                    warn.extend(warning);
                    StateUpdate::execute_with_warning(action, warn)
                }
                MoodState::Angry => StateUpdate::execute_with_warning(action, warning),
            }
        } else {
            StateUpdate::execute_with_warning(action, warning)
        }
    }

    pub(crate) fn dialog_chooser(
        title: &str,
        options: Vec<(Option<EventOrigin>, ActiveDialog)>,
    ) -> RenderResult {
        match options.len() {
            0 => {
                panic!("no dialog options provided");
            }
            1 => StateUpdate::open_dialog(options[0].1.clone()),
            _ => StateUpdate::open_dialog(ActiveDialog::DialogChooser(Box::new(DialogChooser {
                title: title.to_string(),
                options,
            }))),
        }
    }

    pub(crate) fn response(action: EventResponse) -> RenderResult {
        Self::execute(Action::Response(action))
    }

    pub(crate) fn move_units(
        rc: &RenderContext,
        pos: Option<Position>,
        intent: MoveIntent,
    ) -> RenderResult {
        let game = rc.game;
        StateUpdate::open_dialog(ActiveDialog::MoveUnits(MoveSelection::new(
            game.active_player(),
            pos,
            game,
            intent,
            &CurrentMove::None,
        )))
    }
}

pub(crate) struct MousePosition {
    pub position: Vec2,
    pub time: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CameraMode {
    Screen,
    World,
}

use crate::info_ui::InfoDialog;
#[cfg(not(target_arch = "wasm32"))]
use server::ai::AI;
use server::events::EventOrigin;

pub struct State {
    pub(crate) assets: Assets,
    pub control_player: Option<usize>,
    pub show_player: usize,
    pub(crate) active_dialog: ActiveDialog,
    pub(crate) pending_update: Option<PendingUpdate>,
    pub world_camera: Camera2D,
    pub world_zoom: f32,
    pub world_zoom_factor: f32,
    pub ui_camera: Camera2D,
    pub ui_scale: f32,
    pub raw_screen_size: Vec2,
    pub screen_size: Vec2, // scaled by ui_scale
    pub(crate) mouse_positions: Vec<MousePosition>,
    pub focused_tile: Option<Position>,
    pub show_permanent_effects: bool,
    pub ai_autoplay: bool,
    pub pan_map: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub ai_players: Vec<AI>,
}

pub const ZOOM: f32 = 0.001;
pub const OFFSET: Vec2 = vec2(-0.75, 0.55);
pub const MIN_OFFSET: Vec2 = vec2(-1.7, -1.);
pub const MAX_OFFSET: Vec2 = vec2(1.2, 3.);

impl State {
    pub async fn new(features: &Features) -> State {
        State {
            active_dialog: ActiveDialog::None,
            pending_update: None,
            assets: Assets::new(features).await,
            control_player: None,
            show_player: 0,
            world_camera: Camera2D {
                offset: OFFSET,
                ..Default::default()
            },
            ui_camera: Camera2D::default(),
            ui_scale: 1.0,
            world_zoom_factor: 1.0,
            world_zoom: ZOOM,
            raw_screen_size: vec2(0., 0.),
            screen_size: vec2(0., 0.),
            mouse_positions: vec![],
            focused_tile: None,
            pan_map: false,
            show_permanent_effects: false,
            ai_autoplay: false,
            #[cfg(not(target_arch = "wasm32"))]
            ai_players: vec![],
        }
    }

    #[must_use]
    pub(crate) fn render_context<'a>(
        &'a self,
        game: &'a Game,
        stage: RenderStage,
    ) -> RenderContext<'a> {
        RenderContext {
            shown_player: game.player(self.show_player),
            game,
            state: self,
            camera_mode: CameraMode::Screen,
            stage,
            icon_background: IconBackground::Auto,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.active_dialog = ActiveDialog::None;
        self.pending_update = None;
        self.focused_tile = None;
    }

    pub(crate) fn update(&mut self, game: &Game, update: StateUpdate) -> GameSyncRequest {
        match update {
            StateUpdate::Execute(a) => GameSyncRequest::ExecuteAction(a),
            StateUpdate::ExecuteWithWarning(update) => {
                self.pending_update = Some(update);
                GameSyncRequest::None
            }
            StateUpdate::Cancel => self.update_from_game(game),
            StateUpdate::ResolvePendingUpdate(confirm) => {
                if confirm {
                    let action = self
                        .pending_update
                        .take()
                        .expect("no pending update")
                        .action;
                    GameSyncRequest::ExecuteAction(action)
                } else {
                    self.clear();
                    GameSyncRequest::None
                }
            }
            StateUpdate::OpenDialog(dialog) => {
                let d = self.game_state_dialog(game);
                if matches!(dialog, ActiveDialog::AdvanceMenu) && d.is_advance() {
                    self.set_dialog(d);
                } else {
                    self.set_dialog(dialog);
                }
                self.focused_tile = None;
                GameSyncRequest::None
            }
            StateUpdate::CloseDialog => {
                let d = self.game_state_dialog(game);
                // should be able to look around before making a decision
                if d.is_advance() || matches!(d, ActiveDialog::Info(_)) {
                    self.set_dialog(ActiveDialog::None);
                } else {
                    self.set_dialog(d);
                }

                GameSyncRequest::None
            }
            StateUpdate::Import => GameSyncRequest::Import,
            StateUpdate::Export => GameSyncRequest::Export,
            StateUpdate::SetShownPlayer(p) => {
                self.show_player = p;
                GameSyncRequest::None
            }
            StateUpdate::SetFocusedTile(p) => {
                self.focused_tile = Some(p);
                GameSyncRequest::None
            }
            StateUpdate::ToggleShowPermanentEffects => {
                self.show_permanent_effects = !self.show_permanent_effects;
                GameSyncRequest::None
            }
            StateUpdate::ToggleAiPlay => {
                self.ai_autoplay = !self.ai_autoplay;
                if self.ai_autoplay {
                    GameSyncRequest::StartAutoplay
                } else {
                    GameSyncRequest::None
                }
            }
        }
    }

    pub(crate) fn set_dialog(&mut self, dialog: ActiveDialog) {
        self.active_dialog = dialog;
    }

    pub fn update_from_game(&mut self, game: &Game) -> GameSyncRequest {
        let dialog = self.game_state_dialog(game);
        self.clear();
        self.active_dialog = dialog;
        GameSyncRequest::None
    }

    #[must_use]
    pub(crate) fn game_state_dialog(&self, game: &Game) -> ActiveDialog {
        if let Some(e) = &game.current_event_handler() {
            return match &e.request {
                PersistentEventRequest::Payment(r) => ActiveDialog::PaymentRequest(
                    r.iter()
                        .map(|p| {
                            Payment::new(
                                &p.cost,
                                &game.player(game.active_player()).resources,
                                p.name.clone(),
                                &p.name,
                                p.optional,
                            )
                        })
                        .collect(),
                ),
                PersistentEventRequest::ResourceReward(r) => {
                    ActiveDialog::ResourceRewardRequest(new_gain(&r.reward, &r.name))
                }
                PersistentEventRequest::SelectAdvance(r) => ActiveDialog::AdvanceRequest(r.clone()),
                PersistentEventRequest::SelectPositions(r) => {
                    ActiveDialog::PositionRequest(MultiSelection::new(r.clone()))
                }
                PersistentEventRequest::SelectUnitType(r) => {
                    ActiveDialog::UnitTypeRequest(r.clone())
                }
                PersistentEventRequest::SelectUnits(r) => {
                    ActiveDialog::UnitsRequest(UnitsSelection::new(r))
                }
                PersistentEventRequest::SelectStructures(r) => ActiveDialog::StructuresRequest(
                    None,
                    MultiSelection::new(MultiRequest::new(
                        r.choices
                            .iter()
                            .map(|s| {
                                SelectedStructureInfo::new(
                                    s.position,
                                    s.structure.clone(),
                                    SelectedStructureStatus::Valid,
                                    None,
                                )
                            })
                            .collect(),
                        r.needed.clone(),
                        &r.description,
                    )),
                ),
                PersistentEventRequest::SelectPlayer(r) => ActiveDialog::PlayerRequest(r.clone()),
                PersistentEventRequest::BoolRequest(d) => ActiveDialog::BoolRequest(d.clone()),
                PersistentEventRequest::ChangeGovernment => ActiveDialog::ChangeGovernmentType,
                PersistentEventRequest::ExploreResolution => {
                    match &game.current_event().event_type {
                        PersistentEventType::ExploreResolution(r) => {
                            ActiveDialog::ExploreResolution(ExploreResolutionConfig {
                                block: r.block.clone(),
                                rotation: r.block.position.rotation,
                            })
                        }
                        _ => {
                            panic!("ExploreResolution expected");
                        }
                    }
                }
                PersistentEventRequest::SelectHandCards(r) => {
                    ActiveDialog::HandCardsRequest(MultiSelection::new(r.clone()))
                }
            };
        }
        match &game.state {
            GameState::ChooseCivilization => {
                ActiveDialog::Info(InfoDialog::choose_civilization(game))
            }
            GameState::Playing | GameState::Finished => ActiveDialog::None,
            GameState::Movement(move_state) => ActiveDialog::MoveUnits(MoveSelection::new(
                game.active_player(),
                self.focused_tile,
                game,
                MoveIntent::Land, // is not used, because no tile is focused
                &move_state.current_move,
            )),
        }
    }

    #[must_use]
    pub(crate) fn measure_text(&self, text: &str) -> TextDimensions {
        measure_text(text, Some(&self.assets.font), FONT_SIZE, 1.0)
    }

    pub(crate) fn draw_text(&self, text: &str, x: f32, y: f32) {
        self.draw_text_with_color(text, x, y, BLACK);
    }

    pub(crate) fn draw_text_with_color(&self, text: &str, x: f32, y: f32, color: Color) {
        draw_text_ex(
            text,
            x,
            y,
            TextParams {
                font: Some(&self.assets.font),
                font_size: FONT_SIZE,
                color,
                ..Default::default()
            },
        );
    }
}
