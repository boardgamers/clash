use macroquad::input::KeyCode::N;
use macroquad::prelude::*;

use server::action::{Action, CombatAction};
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
    RaseSize1City,
    DetermineFirstPlayer,
    ChangeGovernmentType,
    ChooseAdditionalAdvances(ChooseAdditionalAdvances),

    // combat
    PlaceSettler,
    Retreat,
    RemoveCasualties(RemoveCasualtiesSelection),
}

pub struct PendingUpdate {
    pub action: Action,
    pub warning: Vec<String>,
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
            StateUpdate::ExecuteWithWarning(PendingUpdate { action, warning })
        }
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
    dialog_stack: Vec<ActiveDialog>,
    pub pending_update: Option<PendingUpdate>,
}

impl State {
    pub async fn new(features: &Features) -> State {
        State {
            active_dialog: ActiveDialog::None,
            dialog_stack: vec![],
            pending_update: None,
            assets: Assets::new(features).await,
            control_player: None,
            show_player: 0,
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
        }
    }

    pub fn clear(&mut self) {
        self.active_dialog = ActiveDialog::None;
        self.dialog_stack.clear();
        self.pending_update = None;
    }

    #[must_use]
    pub fn is_collect(&self) -> bool {
        if let ActiveDialog::CollectResources(_c) = &self.active_dialog {
            return true;
        }
        false
    }

    #[must_use]
    pub fn has_modal_dialog(&self) -> bool {
        !matches!(
            self.active_dialog,
            ActiveDialog::None |
            ActiveDialog::TileMenu(_) |
            ActiveDialog::Log 
        )
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
        if matches!(self.active_dialog, ActiveDialog::TileMenu(_)) {
            self.close_dialog();
        }
        if !matches!(self.active_dialog, ActiveDialog::None) {
            self.dialog_stack.push(self.active_dialog.clone());
        }
        self.active_dialog = dialog;
    }

    pub fn set_dialog(&mut self, dialog: ActiveDialog) {
        self.active_dialog = dialog;
        self.dialog_stack.clear();
    }

    fn close_dialog(&mut self) {
        if let Some(dialog) = self.dialog_stack.pop() {
            self.active_dialog = dialog;
        } else {
            self.active_dialog = ActiveDialog::None;
        }
    }

    pub fn update_from_game(&mut self, game: &Game) -> GameSyncRequest {
        let last_dialog = self.active_dialog.clone();
        self.clear();

        self.active_dialog = match &game.state {
            GameState::Movement { .. } => {
                let start = if let ActiveDialog::TileMenu(p) = last_dialog {
                    Some(p)
                } else {
                    None
                };
                ActiveDialog::MoveUnits(MoveSelection::new(game.active_player(), start))
            }
            GameState::CulturalInfluenceResolution(c) => {
                ActiveDialog::CulturalInfluenceResolution(c.clone())
            }
            GameState::StatusPhase(state) => match state {
                StatusPhaseState::CompleteObjectives => {
                    self.execute_status_phase(game, StatusPhaseAction::CompleteObjectives(vec![]))
                }
                StatusPhaseState::FreeAdvance => ActiveDialog::FreeAdvance,
                StatusPhaseState::RaseSize1City => ActiveDialog::RaseSize1City,
                StatusPhaseState::ChangeGovernmentType => ActiveDialog::ChangeGovernmentType,
                StatusPhaseState::DetermineFirstPlayer => ActiveDialog::DetermineFirstPlayer,
            },
            GameState::PlaceSettler { .. } => ActiveDialog::PlaceSettler,
            GameState::Combat(c) => match c.phase {
                CombatPhase::PlayActionCard(_) => {
                    self.update(
                        game,
                        StateUpdate::Execute(Action::Combat(CombatAction::PlayActionCard(None))),
                    );
                    ActiveDialog::None
                } //todo(gregor)
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
        };
        GameSyncRequest::None
    }

    fn execute_status_phase(&mut self, game: &Game, action: StatusPhaseAction) -> ActiveDialog {
        self.update(game, StateUpdate::status_phase(action));
        ActiveDialog::None
    }
}
