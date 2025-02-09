use crate::assets::Assets;
use crate::client::{Features, GameSyncRequest};
use crate::collect_ui::CollectResources;
use crate::combat_ui::RemoveCasualtiesSelection;
use crate::construct_ui::ConstructionPayment;
use crate::happiness_ui::IncreaseHappinessConfig;
use crate::layout_ui::FONT_SIZE;
use crate::log_ui::{add_advance_help, advance_help};
use crate::map_ui::ExploreResolutionConfig;
use crate::move_ui::{MoveDestination, MoveIntent, MovePayment, MoveSelection};
use crate::payment_ui::Payment;
use crate::recruit_unit_ui::{RecruitAmount, RecruitSelection};
use crate::render_context::RenderContext;
use crate::status_phase_ui::ChooseAdditionalAdvances;
use macroquad::prelude::*;
use server::action::Action;
use server::city::{City, MoodState};
use server::combat::{active_attackers, active_defenders, CombatPhase};
use server::content::advances::{NAVIGATION, ROADS};
use server::content::custom_phase_actions::{CustomPhaseAdvanceRewardRequest, CustomPhaseRequest};
use server::events::EventOrigin;
use server::game::{CulturalInfluenceResolution, CurrentMove, Game, GameState};
use server::position::Position;
use server::status_phase::{StatusPhaseAction, StatusPhaseState};
use server::unit::carried_units;

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
    CulturalInfluence,
    CulturalInfluenceResolution(CulturalInfluenceResolution),
    ExploreResolution(ExploreResolutionConfig),

    // status phase
    FreeAdvance,
    RazeSize1City,
    CompleteObjectives,
    DetermineFirstPlayer,
    ChangeGovernmentType,
    ChooseAdditionalAdvances(ChooseAdditionalAdvances),

    // combat
    PlayActionCard,
    PlaceSettler,
    Retreat,
    RemoveCasualties(RemoveCasualtiesSelection),

    CustomPhaseResourceRewardRequest(Payment),
    CustomPhaseAdvanceRewardRequest(CustomPhaseAdvanceRewardRequest),
    CustomPhasePaymentRequest(Vec<Payment>),
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
            ActiveDialog::CulturalInfluence => "cultural influence",
            ActiveDialog::CulturalInfluenceResolution(_) => "cultural influence resolution",
            ActiveDialog::ExploreResolution(_) => "explore resolution",
            ActiveDialog::FreeAdvance => "free advance",
            ActiveDialog::RazeSize1City => "raze size 1 city",
            ActiveDialog::CompleteObjectives => "complete objectives",
            ActiveDialog::DetermineFirstPlayer => "determine first player",
            ActiveDialog::ChangeGovernmentType => "change government type",
            ActiveDialog::ChooseAdditionalAdvances(_) => "choose additional advances",
            ActiveDialog::PlayActionCard => "play action card",
            ActiveDialog::PlaceSettler => "place settler",
            ActiveDialog::Retreat => "retreat",
            ActiveDialog::RemoveCasualties(_) => "remove casualties",
            ActiveDialog::CustomPhaseResourceRewardRequest(_) => "trade route selection",
            ActiveDialog::CustomPhaseAdvanceRewardRequest(_) => "advance selection",
            ActiveDialog::CustomPhasePaymentRequest(_) => "custom phase payment request",
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
            ActiveDialog::AdvancePayment(_)
            | ActiveDialog::ConstructionPayment(_)
            | ActiveDialog::MovePayment(_) => {
                vec!["Pay resources".to_string()]
            }
            ActiveDialog::CollectResources(collect) => collect.help_text(rc.game),
            ActiveDialog::RecruitUnitSelection(_) => vec!["Click on a unit to recruit".to_string()],
            ActiveDialog::ReplaceUnits(_) => vec!["Click on a unit to replace".to_string()],
            ActiveDialog::MoveUnits(m) => {
                if m.start.is_some() {
                    let mut result = vec![];
                    if m.destinations.is_empty() {
                        result.push("No unit on this tile can move".to_string());
                    }
                    if m.destinations
                        .iter()
                        .any(|d| matches!(d, MoveDestination::Tile(_)))
                    {
                        result.push("Click on a highlighted tile to move units".to_string());
                    };
                    if m.destinations
                        .iter()
                        .any(|d| matches!(d, MoveDestination::Carrier(_)))
                    {
                        result.push("Click on a carrier to embark units".to_string());
                    };
                    add_advance_help(rc, &mut result, NAVIGATION);
                    add_advance_help(rc, &mut result, ROADS);
                    result
                } else {
                    vec!["Click on a unit to move".to_string()]
                }
            }
            ActiveDialog::CulturalInfluence => {
                vec!["Click on a building to influence its culture".to_string()]
            }
            ActiveDialog::CulturalInfluenceResolution(c) => vec![format!(
                "Pay {} to influence {}",
                c.roll_boost_cost,
                c.city_piece.name()
            )],
            ActiveDialog::ExploreResolution(_) => {
                vec!["Click on the new tile to rotate it".to_string()]
            }
            ActiveDialog::FreeAdvance => {
                vec!["Click on an advance to take it for free".to_string()]
            }
            ActiveDialog::RazeSize1City => {
                vec!["Click on a city to raze it - or click cancel".to_string()]
            }
            ActiveDialog::CompleteObjectives => {
                vec!["Click on an objective to complete it".to_string()]
            }
            ActiveDialog::DetermineFirstPlayer => {
                vec!["Click on a player to determine first player".to_string()]
            }
            ActiveDialog::ChangeGovernmentType => {
                vec!["Click on a government type to change - or click cancel".to_string()]
            }
            ActiveDialog::ChooseAdditionalAdvances(_) => {
                vec!["Click on an advance to choose it".to_string()]
            }
            ActiveDialog::PlayActionCard => vec!["Click on an action card to play it".to_string()],
            ActiveDialog::PlaceSettler => vec!["Click on a tile to place a settler".to_string()],
            ActiveDialog::Retreat => vec!["Do you want to retreat?".to_string()],
            ActiveDialog::RemoveCasualties(r) => vec![format!(
                "Remove {} units: click on a unit to remove it",
                if r.needed_carried > 0 {
                    format!("{} ships and {} carried units", r.needed, r.needed_carried)
                } else {
                    r.needed.to_string()
                }
            )],
            ActiveDialog::WaitingForUpdate => vec!["Waiting for server update".to_string()],
            ActiveDialog::CustomPhaseResourceRewardRequest(_)
            | ActiveDialog::CustomPhaseAdvanceRewardRequest(_)
            | ActiveDialog::CustomPhasePaymentRequest(_) => Self::event_help(rc),
        }
    }

    #[must_use]
    pub fn event_help(rc: &RenderContext) -> Vec<String> {
        match &Self::event_origin(rc) {
            EventOrigin::Advance(a) => advance_help(rc, a),
            _ => vec![], // TODO
        }
    }

    #[must_use]
    pub fn event_origin(rc: &RenderContext) -> EventOrigin {
        rc.game
            .custom_phase_state
            .current
            .as_ref()
            .unwrap()
            .origin
            .clone()
    }

    #[must_use]
    pub fn show_for_other_player(&self) -> bool {
        matches!(self, ActiveDialog::Log | ActiveDialog::DetermineFirstPlayer) || self.is_advance()
    }

    #[must_use]
    pub fn is_modal(&self) -> bool {
        matches!(self, ActiveDialog::Log) || self.is_advance()
    }

    #[must_use]
    pub fn is_full_modal(&self) -> bool {
        self.is_modal()
    }

    #[must_use]
    pub fn is_advance(&self) -> bool {
        matches!(
            self,
            ActiveDialog::AdvanceMenu
                | ActiveDialog::FreeAdvance
                | ActiveDialog::AdvancePayment(_)
                | ActiveDialog::ChangeGovernmentType
                | ActiveDialog::ChooseAdditionalAdvances(_)
                | ActiveDialog::CustomPhaseAdvanceRewardRequest(_)
        )
    }
}

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
        if let Some(yes) = yes {
            if let Some(no) = no {
                StateUpdate::OpenDialog(ActiveDialog::DialogChooser(Box::new(DialogChooser {
                    title: title.to_string(),
                    yes,
                    no,
                })))
            } else {
                StateUpdate::OpenDialog(yes)
            }
        } else if let Some(no) = no {
            StateUpdate::OpenDialog(no)
        } else {
            panic!("no dialog to open")
        }
    }

    pub fn status_phase(action: StatusPhaseAction) -> StateUpdate {
        StateUpdate::Execute(Action::StatusPhase(action))
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
        }
    }

    #[must_use]
    pub fn render_context<'a>(&'a self, game: &'a Game) -> RenderContext<'a> {
        RenderContext {
            shown_player: game.get_player(self.show_player),
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
        if let Some(e) = &game.custom_phase_state.current {
            return match &e.request {
                CustomPhaseRequest::Payment(r) => ActiveDialog::CustomPhasePaymentRequest(
                    r.iter()
                        .map(|p| {
                            Payment::new(
                                &p.cost,
                                &game.get_player(game.active_player()).resources,
                                &p.name,
                                p.optional,
                            )
                        })
                        .collect(),
                ),
                CustomPhaseRequest::ResourceReward(r) => {
                    ActiveDialog::CustomPhaseResourceRewardRequest(Payment::new_gain(
                        &r.reward, &r.name,
                    ))
                }
                CustomPhaseRequest::AdvanceReward(r) => {
                    ActiveDialog::CustomPhaseAdvanceRewardRequest(r.clone())
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
            GameState::CulturalInfluenceResolution(c) => {
                ActiveDialog::CulturalInfluenceResolution(c.clone())
            }
            GameState::StatusPhase(state) => match state {
                StatusPhaseState::CompleteObjectives => ActiveDialog::CompleteObjectives,
                StatusPhaseState::FreeAdvance => ActiveDialog::FreeAdvance,
                StatusPhaseState::RazeSize1City => ActiveDialog::RazeSize1City,
                StatusPhaseState::ChangeGovernmentType => ActiveDialog::ChangeGovernmentType,
                StatusPhaseState::DetermineFirstPlayer => ActiveDialog::DetermineFirstPlayer,
            },
            GameState::PlaceSettler { .. } => ActiveDialog::PlaceSettler,
            GameState::Combat(c) => match &c.phase {
                CombatPhase::PlayActionCard(_) => ActiveDialog::PlayActionCard,
                CombatPhase::RemoveCasualties(r) => {
                    let (position, selectable) = if r.player == c.attacker {
                        (
                            c.attacker_position,
                            active_attackers(game, c.attacker, &c.attackers, c.defender_position)
                                .clone()
                                .into_iter()
                                .chain(c.attackers.iter().flat_map(|a| {
                                    let units = carried_units(*a, game.get_player(r.player));
                                    units
                                }))
                                .collect(),
                        )
                    } else if r.player == c.defender {
                        (
                            c.defender_position,
                            active_defenders(game, c.defender, c.defender_position),
                        )
                    } else {
                        panic!("player should be either defender or attacker")
                    };
                    ActiveDialog::RemoveCasualties(RemoveCasualtiesSelection::new(
                        r.player,
                        position,
                        r.casualties,
                        r.carried_units_casualties,
                        selectable,
                    ))
                }
                CombatPhase::Retreat => ActiveDialog::Retreat,
            },
            GameState::ExploreResolution(r) => {
                ActiveDialog::ExploreResolution(ExploreResolutionConfig {
                    block: r.block.clone(),
                    rotation: r.block.position.rotation,
                })
            }
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
