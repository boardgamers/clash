use macroquad::prelude::*;
use server::action::Action;
use server::city::{City, MoodState};
use server::combat::{active_attackers, active_defenders, CombatPhase};
use server::game::{CulturalInfluenceResolution, Game, GameState};
use server::player::Player;
use server::position::Position;
use server::status_phase::{StatusPhaseAction, StatusPhaseState};

use crate::advance_ui::AdvancePayment;
use crate::assets::Assets;
use crate::client::{Features, GameSyncRequest};
use crate::collect_ui::CollectResources;
use crate::combat_ui::RemoveCasualtiesSelection;
use crate::construct_ui::ConstructionPayment;
use crate::happiness_ui::IncreaseHappiness;
use crate::move_ui::MoveSelection;
use crate::recruit_unit_ui::{RecruitAmount, RecruitSelection};
use crate::status_phase_ui::ChooseAdditionalAdvances;

#[derive(Clone)]
pub enum ActiveDialog {
    None,
    TileMenu(Position),
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
            ActiveDialog::TileMenu(_) => "tile menu",
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
    pub fn is_map_dialog(&self) -> bool {
        matches!(
            self,
            ActiveDialog::TileMenu(_)
                | ActiveDialog::IncreaseHappiness(_)
                | ActiveDialog::CollectResources(_)
                | ActiveDialog::MoveUnits(_)
                | ActiveDialog::PlaceSettler
                | ActiveDialog::RazeSize1City
        )
    }
}

pub struct PendingUpdate {
    pub action: Action,
    pub warning: Vec<String>,
    pub info: Vec<String>,
    pub can_confirm: bool,
}

#[must_use]
pub enum StateUpdate {
    None,
    SetDialog(ActiveDialog),
    OpenDialog(ActiveDialog),
    CloseDialog,
    Cancel,
    ResolvePendingUpdate(bool),
    Execute(Action),
    ExecuteWithWarning(PendingUpdate),
    Import,
    Export,
    SetShownPlayer(usize),
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
                can_confirm: true,
            })
        }
    }

    pub fn execute_with_confirm(info: Vec<String>, action: Action) -> StateUpdate {
        StateUpdate::ExecuteWithWarning(PendingUpdate {
            action,
            warning: vec![],
            info,
            can_confirm: true,
        })
    }

    pub fn execute_with_cancel(info: Vec<String>) -> StateUpdate {
        StateUpdate::ExecuteWithWarning(PendingUpdate {
            action: Action::Undo, // never used
            warning: vec![],
            info,
            can_confirm: false,
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

#[derive(Clone)]
pub struct ShownPlayer {
    pub index: usize,
    pub can_control: bool,
    pub can_play_action: bool,
    pub active_dialog: ActiveDialog,
    pub pending_update: bool,
}

impl ShownPlayer {
    #[must_use]
    pub fn get<'a>(&self, game: &'a Game) -> &'a Player {
        game.get_player(self.index)
    }
}

pub struct State {
    pub assets: Assets,
    pub control_player: Option<usize>,
    pub show_player: usize,
    pub active_dialog: ActiveDialog,
    pub pending_update: Option<PendingUpdate>,
    pub camera: Camera2D,
    pub zoom: f32,
    pub offset: Vec2,
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
                ..Default::default()
            },
            zoom: ZOOM,
            offset: OFFSET,
        }
    }

    #[must_use]
    pub fn shown_player(&self, game: &Game) -> ShownPlayer {
        let a = game.active_player();
        let control = self.control_player == Some(a) && self.show_player == a;
        ShownPlayer {
            index: self.show_player,
            can_control: control,
            can_play_action: control && game.state == GameState::Playing && game.actions_left > 0,
            active_dialog: self.active_dialog.clone(),
            pending_update: self.pending_update.is_some(),
        }
    }

    pub fn clear(&mut self) {
        self.active_dialog = ActiveDialog::None;
        self.pending_update = None;
    }

    #[must_use]
    pub fn is_collect(&self) -> bool {
        if let ActiveDialog::CollectResources(_c) = &self.active_dialog {
            return true;
        }
        false
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
                    self.pending_update = None;
                    self.close_dialog();
                    GameSyncRequest::None
                }
            }
            StateUpdate::SetDialog(dialog) => {
                self.set_dialog(dialog);
                GameSyncRequest::None
            }
            StateUpdate::OpenDialog(dialog) => {
                self.open_dialog(dialog);
                GameSyncRequest::None
            }
            StateUpdate::CloseDialog => {
                self.close_dialog();
                GameSyncRequest::None
            }
            StateUpdate::Import => GameSyncRequest::Import,
            StateUpdate::Export => GameSyncRequest::Export,
            StateUpdate::SetShownPlayer(p) => {
                self.show_player = p;
                GameSyncRequest::None
            }
        }
    }

    fn open_dialog(&mut self, dialog: ActiveDialog) {
        if self.active_dialog.title() == dialog.title() {
            self.close_dialog();
            return;
        }
        if matches!(self.active_dialog, ActiveDialog::TileMenu(_)) {
            self.close_dialog();
        }
        self.active_dialog = dialog;
    }

    pub fn set_dialog(&mut self, dialog: ActiveDialog) {
        self.active_dialog = dialog;
    }

    fn close_dialog(&mut self) {
        self.active_dialog = ActiveDialog::None;
    }

    pub fn update_from_game(&mut self, game: &Game) -> GameSyncRequest {
        let last_dialog = self.active_dialog.clone();
        self.clear();

        self.active_dialog = self.game_state_dialog(game, &last_dialog);
        GameSyncRequest::None
    }

    #[must_use]
    pub fn game_state_dialog(&self, game: &Game, last_dialog: &ActiveDialog) -> ActiveDialog {
        match &game.state {
            GameState::Movement { .. } => {
                let start = if let ActiveDialog::TileMenu(p) = last_dialog {
                    Some(*p)
                } else {
                    None
                };
                ActiveDialog::MoveUnits(MoveSelection::new(game.active_player(), start))
            }
            GameState::CulturalInfluenceResolution(c) => {
                ActiveDialog::CulturalInfluenceResolution(c.clone())
            }
            GameState::StatusPhase(state) => match state {
                StatusPhaseState::CompleteObjectives => ActiveDialog::CompleteObjectives,
                StatusPhaseState::FreeAdvance => ActiveDialog::FreeAdvance,
                StatusPhaseState::RaseSize1City => ActiveDialog::RazeSize1City,
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

    // fn execute_status_phase(&mut self, game: &Game, action: StatusPhaseAction) -> ActiveDialog {
    //     self.update(game, StateUpdate::status_phase(action));
    //     ActiveDialog::None
    // }
}
