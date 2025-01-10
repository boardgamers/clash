use macroquad::prelude::*;
use server::action::Action;
use server::city::{City, MoodState};
use server::combat::{active_attackers, active_defenders, CombatPhase};
use server::game::{CulturalInfluenceResolution, Game, GameState};
use server::position::Position;
use server::status_phase::{StatusPhaseAction, StatusPhaseState};

use crate::advance_ui::AdvancePayment;
use crate::assets::Assets;
use crate::city_ui::building_name;
use crate::client::{Features, GameSyncRequest};
use crate::collect_ui::CollectResources;
use crate::combat_ui::RemoveCasualtiesSelection;
use crate::construct_ui::ConstructionPayment;
use crate::happiness_ui::IncreaseHappiness;
use crate::layout_ui::FONT_SIZE;
use crate::move_ui::MoveSelection;
use crate::recruit_unit_ui::{RecruitAmount, RecruitSelection};
use crate::render_context::RenderContext;
use crate::status_phase_ui::ChooseAdditionalAdvances;

#[derive(Clone)]
pub enum ActiveDialog {
    None,
    Log,
    WaitingForUpdate,

    // playing actions
    IncreaseHappiness(IncreaseHappiness),
    AdvanceMenu,
    AdvancePayment(AdvancePayment),
    ConstructionPayment(ConstructionPayment),
    CollectResources(CollectResources),
    RecruitUnitSelection(RecruitAmount),
    ReplaceUnits(RecruitSelection),
    MoveUnits(MoveSelection),
    CulturalInfluence,
    CulturalInfluenceResolution(CulturalInfluenceResolution),

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
}

impl ActiveDialog {
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            ActiveDialog::None => "none",
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
            ActiveDialog::CulturalInfluence => "cultural influence",
            ActiveDialog::CulturalInfluenceResolution(_) => "cultural influence resolution",
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
        }
    }

    #[must_use]
    pub fn help_message(&self, game: &Game) -> Vec<String> {
        match self {
            ActiveDialog::None | ActiveDialog::Log | ActiveDialog::AdvanceMenu => vec![],
            ActiveDialog::IncreaseHappiness(_) => {
                vec!["Click on a city to increase happiness".to_string()]
            }
            ActiveDialog::AdvancePayment(a) => {
                vec![format!("Click on resources to pay for {}", a.name)]
            }
            ActiveDialog::ConstructionPayment(c) => {
                vec![format!("Click on resources to pay for {}", c.name)]
            }
            ActiveDialog::CollectResources(collect) => collect.help_text(game),
            ActiveDialog::RecruitUnitSelection(_) => vec!["Click on a unit to recruit".to_string()],
            ActiveDialog::ReplaceUnits(_) => vec!["Click on a unit to replace".to_string()],
            ActiveDialog::MoveUnits(m) => {
                if m.start.is_some() {
                    vec!["Click on a highlighted tile to move units".to_string()]
                } else {
                    vec!["Click on a unit to move".to_string()]
                }
            }
            ActiveDialog::CulturalInfluence => {
                vec!["Click on a building to influence its culture".to_string()]
            }
            ActiveDialog::CulturalInfluenceResolution(c) => vec![format!(
                "Pay {} culture tokens to influence {}",
                c.roll_boost_cost,
                building_name(c.city_piece)
            )],
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
                r.needed
            )],
            ActiveDialog::WaitingForUpdate => vec!["Waiting for server update".to_string()],
        }
    }

    #[must_use]
    pub fn show_for_other_player(&self) -> bool {
        matches!(self, ActiveDialog::Log | ActiveDialog::DetermineFirstPlayer) || self.is_advance()
    }

    #[must_use]
    pub fn is_modal(&self) -> bool {
        matches!(self, ActiveDialog::Log) || self.is_full_modal()
    }

    #[must_use]
    pub fn is_full_modal(&self) -> bool {
        self.is_advance()
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
        )
    }
}

pub struct PendingUpdate {
    pub action: Action,
    pub warning: Vec<String>,
    pub info: Vec<String>,
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

    pub fn status_phase(action: StatusPhaseAction) -> StateUpdate {
        StateUpdate::Execute(Action::StatusPhase(action))
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
        match &game.state {
            GameState::Movement { .. } => ActiveDialog::MoveUnits(MoveSelection::new(
                game.active_player(),
                self.focused_tile,
                game,
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
            GameState::Combat(c) => match c.phase {
                CombatPhase::PlayActionCard(_) => ActiveDialog::PlayActionCard,
                CombatPhase::RemoveCasualties {
                    player, casualties, ..
                } => {
                    let (position, selectable) = if player == c.attacker {
                        (
                            c.attacker_position,
                            active_attackers(game, c.attacker, &c.attackers, c.defender_position),
                        )
                    } else if player == c.defender {
                        (
                            c.defender_position,
                            active_defenders(game, c.defender, c.defender_position),
                        )
                    } else {
                        panic!("player should be either defender or attacker")
                    };
                    ActiveDialog::RemoveCasualties(RemoveCasualtiesSelection::new(
                        position, casualties, selectable,
                    ))
                }
                CombatPhase::Retreat => ActiveDialog::Retreat,
            },
            _ => ActiveDialog::None,
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
