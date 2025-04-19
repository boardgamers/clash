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
use crate::layout_ui::FONT_SIZE;
use crate::map_ui::ExploreResolutionConfig;
use crate::move_ui::{MoveDestination, MoveIntent, MovePayment, MoveSelection};
use crate::payment_ui::Payment;
use crate::recruit_unit_ui::{RecruitAmount, RecruitSelection};
use crate::render_context::RenderContext;
use crate::status_phase_ui::ChooseAdditionalAdvances;
use macroquad::prelude::*;
use server::action::Action;
use server::card::HandCard;
use server::city::{City, MoodState};
use server::content::persistent_events::{
    AdvanceRequest, ChangeGovernmentRequest, EventResponse, MultiRequest, PersistentEventRequest,
    PersistentEventType, PlayerRequest, UnitTypeRequest,
};
use server::events::EventOrigin;
use server::game::{Game, GameState};
use server::movement::CurrentMove;
use server::playing_actions::PlayingActionType;
use server::position::Position;

#[derive(Clone)]
pub enum ActiveDialog {
    None,
    Log,
    WaitingForUpdate,
    DialogChooser(Box<DialogChooser>),

    // playing actions
    IncreaseHappiness(IncreaseHappinessConfig),
    AdvanceMenu,
    AdvancePayment(Payment),
    ConstructionPayment(ConstructionPayment),
    CollectResources(CollectResources),
    RecruitUnitSelection(RecruitAmount),
    ReplaceUnits(RecruitSelection),
    MoveUnits(MoveSelection),
    MovePayment(MovePayment),
    ExploreResolution(ExploreResolutionConfig),

    // custom
    Theaters(Payment),
    Taxes(Payment),
    ResourceRewardRequest(Payment),
    AdvanceRequest(AdvanceRequest),
    PaymentRequest(Vec<Payment>),
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
    ChangeGovernmentType(ChangeGovernmentRequest),
    ChooseAdditionalAdvances(ChooseAdditionalAdvances),
}

impl ActiveDialog {
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            ActiveDialog::None => "none",
            ActiveDialog::DialogChooser(_) => "dialog chooser",
            ActiveDialog::Log => "log",
            ActiveDialog::WaitingForUpdate => "waiting for update",
            ActiveDialog::IncreaseHappiness(_) => "increase happiness",
            ActiveDialog::AdvanceMenu => "advance menu",
            ActiveDialog::AdvancePayment(_) => "advance payment",
            ActiveDialog::ConstructionPayment(_) => "construction payment",
            ActiveDialog::CollectResources(_) => "collect resources",
            ActiveDialog::RecruitUnitSelection(_) => "recruit unit selection",
            ActiveDialog::ReplaceUnits(_) => "replace units",
            ActiveDialog::MoveUnits(_) => "move units",
            ActiveDialog::MovePayment(_) => "move payment",
            ActiveDialog::ExploreResolution(_) => "explore resolution",
            ActiveDialog::ChangeGovernmentType(_) => "change government type",
            ActiveDialog::ChooseAdditionalAdvances(_) => "choose additional advances",
            ActiveDialog::Theaters(_) => "theaters",
            ActiveDialog::Taxes(_) => "collect taxes",
            ActiveDialog::ResourceRewardRequest(_) => "trade route selection",
            ActiveDialog::AdvanceRequest(_) => "advance selection",
            ActiveDialog::PaymentRequest(_) => "custom phase payment request",
            ActiveDialog::PlayerRequest(_) => "custom phase player request",
            ActiveDialog::PositionRequest(_) => "custom phase position request",
            ActiveDialog::UnitTypeRequest(_) => "custom phase unit request",
            ActiveDialog::UnitsRequest(_) => "custom phase units request",
            ActiveDialog::StructuresRequest(_, _) => "custom phase structures request",
            ActiveDialog::BoolRequest(_) => "custom phase bool request",
            ActiveDialog::HandCardsRequest(_) => "custom phase hand cards request",
        }
    }

    #[must_use]
    pub fn help_message(&self, rc: &RenderContext) -> Vec<String> {
        match self {
            ActiveDialog::None
            | ActiveDialog::Log
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
                vec!["Click on the new tile to rotate it".to_string()]
            }
            ActiveDialog::ChangeGovernmentType(r) => {
                if r.optional {
                    vec!["Click on a government type to change - or click cancel".to_string()]
                } else {
                    vec!["Click on a government type to change".to_string()]
                }
            }
            ActiveDialog::ChooseAdditionalAdvances(_) => {
                vec!["Click on an advance to choose it".to_string()]
            }
            ActiveDialog::WaitingForUpdate => vec!["Waiting for server update".to_string()],
            ActiveDialog::Taxes(_) => event_help(rc, &EventOrigin::Advance("Taxes".to_string())),
            ActiveDialog::Theaters(_) => {
                event_help(rc, &EventOrigin::Advance("Theaters".to_string()))
            }
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
                            &rc.shown_player.custom_actions[&c.custom_action_type],
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
    pub fn show_for_other_player(&self) -> bool {
        matches!(self, ActiveDialog::Log | ActiveDialog::PlayerRequest(_)) | self.is_advance()
    }

    #[must_use]
    pub fn is_modal(&self) -> bool {
        matches!(self, ActiveDialog::Log) || self.is_advance()
    }

    #[must_use]
    pub fn is_advance(&self) -> bool {
        matches!(
            self,
            ActiveDialog::AdvanceMenu
                | ActiveDialog::AdvancePayment(_)
                | ActiveDialog::ChangeGovernmentType(_)
                | ActiveDialog::ChooseAdditionalAdvances(_)
                | ActiveDialog::AdvanceRequest(_)
        )
    }
}

#[derive(Clone)]
pub struct PendingUpdate {
    pub action: Action,
    pub warning: Vec<String>,
    pub info: Vec<String>,
}

#[derive(Clone)]
pub struct DialogChooser {
    pub title: String,
    pub yes: ActiveDialog,
    pub no: ActiveDialog,
}

#[must_use]
#[derive(Clone)]
pub enum StateUpdate {
    None,
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

impl StateUpdate {
    pub fn execute(action: Action) -> StateUpdate {
        StateUpdate::Execute(action)
    }

    pub fn execute_with_warning(action: Action, warning: Vec<String>) -> StateUpdate {
        if warning.is_empty() {
            StateUpdate::Execute(action)
        } else {
            StateUpdate::ExecuteWithWarning(PendingUpdate {
                action,
                warning,
                info: vec![],
            })
        }
    }

    pub fn execute_with_confirm(info: Vec<String>, action: Action) -> StateUpdate {
        StateUpdate::ExecuteWithWarning(PendingUpdate {
            action,
            warning: vec![],
            info,
        })
    }

    pub fn execute_activation(action: Action, warning: Vec<String>, city: &City) -> StateUpdate {
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

    pub fn dialog_chooser(
        title: &str,
        yes: Option<ActiveDialog>,
        no: Option<ActiveDialog>,
    ) -> StateUpdate {
        match yes {
            Some(yes) => match no {
                Some(no) => {
                    StateUpdate::OpenDialog(ActiveDialog::DialogChooser(Box::new(DialogChooser {
                        title: title.to_string(),
                        yes,
                        no,
                    })))
                }
                _ => StateUpdate::OpenDialog(yes),
            },
            _ => match no {
                Some(no) => StateUpdate::OpenDialog(no),
                _ => {
                    panic!("no dialog to open")
                }
            },
        }
    }

    pub fn response(action: EventResponse) -> StateUpdate {
        StateUpdate::Execute(Action::Response(action))
    }

    pub fn move_units(
        rc: &RenderContext,
        pos: Option<Position>,
        intent: MoveIntent,
    ) -> StateUpdate {
        let game = rc.game;
        StateUpdate::OpenDialog(ActiveDialog::MoveUnits(MoveSelection::new(
            game.active_player(),
            pos,
            game,
            intent,
            &CurrentMove::None,
        )))
    }

    pub fn or(self, other: impl FnOnce() -> StateUpdate) -> StateUpdate {
        match self {
            StateUpdate::None => other(),
            _ => self,
        }
    }
}

#[must_use]
pub struct StateUpdates {
    updates: Vec<StateUpdate>,
}

impl Default for StateUpdates {
    fn default() -> Self {
        Self::new()
    }
}

impl StateUpdates {
    pub fn new() -> StateUpdates {
        StateUpdates { updates: vec![] }
    }
    pub fn add(&mut self, update: StateUpdate) {
        if !matches!(update, StateUpdate::None) {
            self.updates.push(update);
        }
    }

    pub fn result(self) -> StateUpdate {
        self.updates
            .into_iter()
            .find(|u| !matches!(u, StateUpdate::None))
            .unwrap_or(StateUpdate::None)
    }
}

pub struct MousePosition {
    pub position: Vec2,
    pub time: f64,
}

pub enum CameraMode {
    Screen,
    World,
}

pub struct State {
    pub assets: Assets,
    pub control_player: Option<usize>,
    pub show_player: usize,
    pub active_dialog: ActiveDialog,
    pub pending_update: Option<PendingUpdate>,
    pub camera: Camera2D,
    pub screen_size: Vec2,
    pub mouse_positions: Vec<MousePosition>,
    pub log_scroll: f32,
    pub focused_tile: Option<Position>,
    pub show_permanent_effects: bool,
    pub ai_autoplay: bool,
    pub pan_map: bool,
}

pub const ZOOM: f32 = 0.001;
pub const OFFSET: Vec2 = vec2(0., 0.45);
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
            camera: Camera2D {
                zoom: vec2(ZOOM, ZOOM),
                offset: OFFSET,
                ..Default::default()
            },
            screen_size: vec2(0., 0.),
            mouse_positions: vec![],
            log_scroll: 0.0,
            focused_tile: None,
            pan_map: false,
            show_permanent_effects: false,
            ai_autoplay: false,
        }
    }

    #[must_use]
    pub fn render_context<'a>(&'a self, game: &'a Game) -> RenderContext<'a> {
        RenderContext {
            shown_player: game.player(self.show_player),
            game,
            state: self,
            camera_mode: CameraMode::Screen,
        }
    }

    pub fn clear(&mut self) {
        self.active_dialog = ActiveDialog::None;
        self.pending_update = None;
        self.focused_tile = None;
    }

    pub fn update(&mut self, game: &Game, update: StateUpdate) -> GameSyncRequest {
        match update {
            StateUpdate::None => GameSyncRequest::None,
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
                self.log_scroll = 0.0;
                GameSyncRequest::None
            }
            StateUpdate::CloseDialog => {
                let d = self.game_state_dialog(game);
                if d.is_advance() {
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

    pub fn set_dialog(&mut self, dialog: ActiveDialog) {
        self.active_dialog = dialog;
    }

    pub fn update_from_game(&mut self, game: &Game) -> GameSyncRequest {
        let dialog = self.game_state_dialog(game);
        self.clear();
        self.active_dialog = dialog;
        GameSyncRequest::None
    }

    #[must_use]
    pub fn game_state_dialog(&self, game: &Game) -> ActiveDialog {
        if let Some(e) = &game.current_event_handler() {
            return match &e.request {
                PersistentEventRequest::Payment(r) => ActiveDialog::PaymentRequest(
                    r.iter()
                        .map(|p| {
                            Payment::new(
                                &p.cost,
                                &game.player(game.active_player()).resources,
                                &p.name,
                                p.optional,
                            )
                        })
                        .collect(),
                ),
                PersistentEventRequest::ResourceReward(r) => {
                    ActiveDialog::ResourceRewardRequest(Payment::new_gain(&r.reward, &r.name))
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
                PersistentEventRequest::ChangeGovernment(r) => {
                    ActiveDialog::ChangeGovernmentType(r.clone())
                }
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
    pub fn measure_text(&self, text: &str) -> TextDimensions {
        measure_text(text, Some(&self.assets.font), FONT_SIZE, 1.0)
    }

    pub fn draw_text(&self, text: &str, x: f32, y: f32) {
        self.draw_text_with_color(text, x, y, BLACK);
    }

    pub fn draw_text_with_color(&self, text: &str, x: f32, y: f32, color: Color) {
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
