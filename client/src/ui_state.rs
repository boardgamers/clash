use crate::advance_ui::AdvancePayment;
use crate::construct_ui::ConstructionPayment;

use crate::collect_ui::CollectResources;
use macroquad::prelude::*;
use server::action::Action;
use server::city::{City, MoodState};
use server::game::{Game, GameState};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

pub enum ActiveDialog<'a> {
    None,
    AdvancePayment(AdvancePayment),
    ConstructionPayment(ConstructionPayment<'a>),
    CollectResources(CollectResources),
}

pub struct PendingUpdate {
    pub action: Action,
    pub warning: Vec<String>,
}

pub enum StateUpdate<'a> {
    None,
    NewDialog(ActiveDialog<'a>),
    Cancel,
    ResolvePendingUpdate(bool),
    Execute(Action),
    ExecuteWithWarning(PendingUpdate),
    InitIncreaseHappiness(IncreaseHappiness),
    FocusCity(usize, Position),
    UpdateActiveDialog(Box<dyn FnOnce(&mut ActiveDialog<'a>)>),
}

impl<'a> StateUpdate<'a> {
    pub fn execute(action: Action, warning: Vec<String>) -> StateUpdate<'a> {
        if warning.is_empty() {
            StateUpdate::Execute(action)
        } else {
            StateUpdate::ExecuteWithWarning(PendingUpdate { action, warning })
        }
    }

    pub fn execute_activation(
        action: Action,
        warning: Vec<String>,
        city: &City,
    ) -> StateUpdate<'a> {
        if city.is_activated() && city.mood_state != MoodState::Angry {
            let mut warn = vec!["City will become angry".to_string()];
            warn.extend(warning);
            StateUpdate::execute(action, warn)
        } else {
            StateUpdate::execute(action, warning)
        }
    }
}

pub struct StateUpdates<'a> {
    updates: Vec<StateUpdate<'a>>,
}

impl<'a> StateUpdates<'a> {
    pub fn new() -> StateUpdates<'a> {
        StateUpdates { updates: vec![] }
    }
    pub fn add(&mut self, update: StateUpdate<'a>) {
        self.updates.push(update);
    }

    #[must_use]
    pub fn result(&self) -> StateUpdate<'a> {
        self.updates
            .into_iter()
            .find(|u| match u {
                StateUpdate::None => false,
                _ => true,
            })
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

pub struct State<'a> {
    pub focused_city: Option<(usize, Position)>,
    pub active_dialog: ActiveDialog<'a>,
    pub pending_update: Option<PendingUpdate>,
    pub increase_happiness: Option<IncreaseHappiness>,
}

impl<'a> State<'a> {
    pub fn new() -> State<'a> {
        State {
            active_dialog: ActiveDialog::None,
            pending_update: None,
            focused_city: None,
            increase_happiness: None,
        }
    }
    pub fn clear(&mut self) {
        self.active_dialog = ActiveDialog::None;
        self.focused_city = None;
        self.increase_happiness = None;
        self.pending_update = None;
    }

    pub fn is_collect(&self) -> bool {
        if let ActiveDialog::CollectResources(_c) = &self.active_dialog {
            return true;
        }
        false
    }

    pub fn update(&mut self, game: &mut Game, update: StateUpdate<'a>) {
        match update {
            StateUpdate::None => {}
            StateUpdate::Execute(a) => {
                self.execute(game, a);
                self.clear()
            }
            StateUpdate::ExecuteWithWarning(update) => {
                self.pending_update = Some(update);
            }
            StateUpdate::Cancel => {
                self.clear()
            }
            StateUpdate::ResolvePendingUpdate(confirm) => {
                if confirm {
                    let action = self.pending_update.take().expect("no pending update").action;
                    self.execute(game, action);
                    self.clear();
                } else {
                    self.pending_update = None;
                }
            }
            StateUpdate::NewDialog(dialog) => {
                self.active_dialog = dialog;
            }
            StateUpdate::InitIncreaseHappiness(h) => {
                self.clear();
                self.increase_happiness = Some(h)
            }
            StateUpdate::FocusCity(p, c) => {
                self.clear();
                self.focused_city = Some((p, c))
            }
        }
    }

    pub fn execute(&mut self, game: &mut Game, a: Action) {
        if let Action::Playing(p) = &a {
            if p == &PlayingAction::EndTurn {
                self.clear();
            }
        }
        game.execute_action(a, game.current_player_index);
        self.active_dialog = ActiveDialog::None;
    }
}

pub fn can_play_action(game: &Game) -> bool {
    game.state == GameState::Playing && game.actions_left > 0
}

pub struct CityMenu<'a> {
    pub player_index: usize,
    pub city_owner_index: usize,
    pub city_position: &'a Position,
}

impl<'a> CityMenu<'a> {
    pub fn new(player_index: usize, city_owner_index: usize, city_position: &'a Position) -> Self {
        CityMenu {
            player_index,
            city_owner_index,
            city_position,
        }
    }

    pub fn get_player(&self, game: &'a Game) -> &'a Player {
        game.get_player(self.player_index)
    }

    pub fn get_city_owner(&self, game: &'a Game) -> &Player {
        game.get_player(self.city_owner_index)
    }

    pub fn get_city(&self, game: &'a Game) -> &City {
        return game.get_city(self.city_owner_index, self.city_position);
    }

    pub fn is_city_owner(&self) -> bool {
        self.player_index == self.city_owner_index
    }
}
