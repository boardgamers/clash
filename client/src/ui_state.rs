use crate::advance_ui::AdvancePayment;
use crate::construct_ui::ConstructionPayment;

use crate::collect_ui::CollectResources;
use macroquad::prelude::*;
use server::city::City;
use server::game::{Game, GameState};
use server::player::Player;
use server::position::Position;
use server::resource_pile::ResourcePile;

pub enum ActiveDialog {
    None,
    AdvancePayment(AdvancePayment),
    ConstructionPayment(ConstructionPayment),
    CollectResources(CollectResources),
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

pub struct State {
    pub focused_city: Option<(usize, Position)>,
    pub active_dialog: ActiveDialog,
    pub increase_happiness: Option<IncreaseHappiness>,
}

impl State {
    pub fn new() -> State {
        State {
            active_dialog: ActiveDialog::None,
            focused_city: None,
            increase_happiness: None,
        }
    }
    pub fn clear(&mut self) {
        self.active_dialog = ActiveDialog::None;
        self.focused_city = None;
        self.increase_happiness = None;
    }

    pub fn is_collect(&self) -> bool {
        if let ActiveDialog::CollectResources(_c) = &self.active_dialog {
            return true;
        }
        false
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
        return game.players[self.city_owner_index]
            .get_city(self.city_position)
            .expect("city not found");
    }

    pub fn is_city_owner(&self) -> bool {
        self.player_index == self.city_owner_index
    }
}
