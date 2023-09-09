use macroquad::prelude::*;

use server::action::Action;
use server::city::{City, MoodState};
use server::game::{Game, GameState};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::status_phase::{StatusPhaseAction, StatusPhaseState};

use crate::advance_ui::AdvancePayment;
use crate::collect_ui::CollectResources;
use crate::construct_ui::ConstructionPayment;
use crate::move_ui::MoveSelection;
use crate::recruit_unit_ui::{RecruitAmount, RecruitSelection};

pub enum ActiveDialog {
    None,
    AdvancePayment(AdvancePayment),
    ConstructionPayment(ConstructionPayment),
    CollectResources(CollectResources),
    RecruitUnitSelection(RecruitAmount),
    ReplaceUnits(RecruitSelection),
    MoveUnits(MoveSelection),
    FreeAdvance,
}

pub struct PendingUpdate {
    pub action: Action,
    pub warning: Vec<String>,
}

#[must_use]
pub enum StateUpdate {
    None,
    SetDialog(ActiveDialog),
    Cancel,
    ResolvePendingUpdate(bool),
    Execute(Action),
    ExecuteWithWarning(PendingUpdate),
    SetIncreaseHappiness(IncreaseHappiness),
    FocusTile(FocusedTile),
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
}

#[must_use]
pub struct StateUpdates {
    updates: Vec<StateUpdate>,
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

pub struct IncreaseHappiness {
    pub steps: Vec<(Position, u32)>,
    pub cost: ResourcePile,
}

impl IncreaseHappiness {
    pub fn new(steps: Vec<(Position, u32)>, cost: ResourcePile) -> IncreaseHappiness {
        IncreaseHappiness { steps, cost }
    }
}

pub struct FocusedTile {
    pub city_owner_index: Option<usize>,
    pub position: Position,
}

impl FocusedTile {
    pub fn new(city_owner_index: Option<usize>, position: Position) -> FocusedTile {
        FocusedTile {
            city_owner_index,
            position,
        }
    }
}

pub struct State {
    pub focused_tile: Option<FocusedTile>,
    pub active_dialog: ActiveDialog,
    pub pending_update: Option<PendingUpdate>,
    pub increase_happiness: Option<IncreaseHappiness>,
}

impl State {
    pub fn new() -> State {
        State {
            active_dialog: ActiveDialog::None,
            pending_update: None,
            focused_tile: None,
            increase_happiness: None,
        }
    }
    pub fn clear(&mut self) {
        self.active_dialog = ActiveDialog::None;
        self.focused_tile = None;
        self.increase_happiness = None;
        self.pending_update = None;
    }

    pub fn is_collect(&self) -> bool {
        if let ActiveDialog::CollectResources(_c) = &self.active_dialog {
            return true;
        }
        false
    }

    pub fn has_dialog(&self) -> bool {
        !matches!(self.active_dialog, ActiveDialog::None) || self.increase_happiness.is_some()
    }

    pub fn update(&mut self, game: &mut Game, update: StateUpdate) {
        match update {
            StateUpdate::None => {}
            StateUpdate::Execute(a) => {
                self.clear();
                self.execute(game, a);
                if let GameState::Movement {
                    movement_actions_left,
                    moved_units: _,
                } = game.state
                {
                    if movement_actions_left > 0 {
                        self.active_dialog =
                            ActiveDialog::MoveUnits(MoveSelection::new(game.active_player()));
                    }
                }
            }
            StateUpdate::ExecuteWithWarning(update) => {
                self.pending_update = Some(update);
            }
            StateUpdate::Cancel => self.clear(),
            StateUpdate::ResolvePendingUpdate(confirm) => {
                if confirm {
                    let action = self
                        .pending_update
                        .take()
                        .expect("no pending update")
                        .action;
                    self.execute(game, action);
                    self.clear();
                } else {
                    self.pending_update = None;
                }
            }
            StateUpdate::SetDialog(dialog) => {
                self.active_dialog = dialog;
            }
            StateUpdate::SetIncreaseHappiness(h) => {
                self.clear();
                self.increase_happiness = Some(h);
            }
            StateUpdate::FocusTile(f) => {
                self.clear();
                self.focused_tile = Some(f);
            }
        }
    }

    pub fn update_after_execute(&mut self, game: &mut Game) -> ActiveDialog {
        match &game.state {
            GameState::Movement {
                movement_actions_left: _,
                moved_units: _,
            } => ActiveDialog::MoveUnits(MoveSelection::new(game.active_player())),
            GameState::StatusPhase(state) => {
                match state {
                    StatusPhaseState::CompleteObjectives => self
                        .execute_status_phase(game, StatusPhaseAction::CompleteObjectives(vec![])),
                    StatusPhaseState::FreeAdvance => ActiveDialog::FreeAdvance,
                    StatusPhaseState::RaseSize1City => {
                        self.execute_status_phase(game, StatusPhaseAction::RaseSize1City(None))
                    } //todo(gregor)
                    StatusPhaseState::ChangeGovernmentType => self
                        .execute_status_phase(game, StatusPhaseAction::ChangeGovernmentType(None)), // todo(gregor)
                    StatusPhaseState::DetermineFirstPlayer => {
                        self.execute_status_phase(game, StatusPhaseAction::DetermineFirstPlayer(0))
                    } // todo(gregor)
                }
            }
            // todo(gregor) also for cultural influence resolution
            _ => ActiveDialog::None,
        }
    }

    fn execute_status_phase(&mut self, game: &mut Game, action: StatusPhaseAction) -> ActiveDialog {
        self.update(game, StateUpdate::Execute(Action::StatusPhase(action)));
        ActiveDialog::None
    }

    pub fn execute(&mut self, game: &mut Game, a: Action) {
        if let Action::Playing(p) = &a {
            if p == &PlayingAction::EndTurn {
                self.clear();
            }
        }
        game.execute_action(a, game.active_player());
        self.active_dialog = ActiveDialog::None;
    }
}

pub fn can_play_action(game: &Game) -> bool {
    game.state == GameState::Playing && game.actions_left > 0
}

pub struct CityMenu {
    pub player_index: usize,
    pub city_owner_index: usize,
    pub city_position: Position,
}

impl CityMenu {
    pub fn new(player_index: usize, city_owner_index: usize, city_position: Position) -> Self {
        CityMenu {
            player_index,
            city_owner_index,
            city_position,
        }
    }

    pub fn get_player<'a>(&self, game: &'a Game) -> &'a Player {
        game.get_player(self.player_index)
    }

    pub fn get_city_owner<'a>(&self, game: &'a Game) -> &'a Player {
        game.get_player(self.city_owner_index)
    }

    pub fn get_city<'a>(&self, game: &'a Game) -> &'a City {
        return game.get_city(self.city_owner_index, self.city_position);
    }

    pub fn is_city_owner(&self) -> bool {
        self.player_index == self.city_owner_index
    }
}
