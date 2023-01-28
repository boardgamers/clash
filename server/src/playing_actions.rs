use serde::{Deserialize, Serialize};

use crate::{
    city::{
        Building::{self, *},
        BuildingData, City, CityData,
    },
    content::custom_actions::CustomAction,
    game::Game,
    hexagon::Position,
    resource_pile::ResourcePile,
};

use PlayingAction::*;

#[derive(Serialize, Deserialize)]
pub enum PlayingAction {
    Advance {
        advance: String,
        payment: ResourcePile,
    },
    Build {
        city_position: Position,
        city_piece: BuildingData,
        payment: ResourcePile,
        temple_bonus: Option<ResourcePile>,
    },
    IncreaseHappiness {
        happiness_increases: Vec<(Position, u32)>,
    },
    InfluenceCulture {
        success: bool,
        starting_city_position: Position,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: BuildingData,
        range_boost: u32,
        result_boost: u32,
    },
    Custom(CustomAction),
    EndTurn,
}

impl PlayingAction {
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match self {
            Advance { advance, payment } => {
                let player = &mut game.players[player_index];
                if !player.can_advance(&advance)
                    || payment.food + payment.ideas + payment.gold as u32 != 2
                {
                    panic!("Illegal action");
                }
                player.loose_resources(payment);
                game.advance(&advance, player_index);
            }
            Build {
                city_position,
                city_piece,
                payment,
                temple_bonus,
            } => {
                let building = Building::from_data(&city_piece);
                let player = &mut game.players[player_index];
                let city = player
                    .get_city(&city_position)
                    .expect("player should have city");
                let cost = player.building_cost(&building, &city);
                if !city.can_increase_size(&building, player) || !payment.can_afford(&cost) {
                    panic!("Illegal action");
                }
                if matches!(building, Temple) {
                    let building_bonus =
                        temple_bonus.expect("build data should contain temple bonus");
                    if building_bonus != ResourcePile::mood_tokens(1)
                        && building_bonus != ResourcePile::culture_tokens(1)
                    {
                        panic!("Invalid temple bonus");
                    }
                    player.gain_resources(building_bonus);
                }
                player.loose_resources(payment);
                player.increase_size(&building, &city_position);
            }
            IncreaseHappiness {
                happiness_increases,
            } => {
                for (city_position, steps) in happiness_increases {
                    let player = &mut game.players[player_index];
                    let city = player
                        .get_city(&city_position)
                        .expect("player should have city");
                    let cost = ResourcePile::mood_tokens(city.size()) * steps;
                    if city.player_index != player_index || !player.resources().can_afford(&cost) {
                        panic!("Illegal action");
                    }
                    player.loose_resources(cost);
                    let city = player
                        .get_city_mut(&city_position)
                        .expect("player should have city");
                    for _ in 0..steps {
                        city.increase_mood_state();
                    }
                }
            }
            InfluenceCulture {
                success,
                starting_city_position,
                target_player_index,
                target_city_position,
                city_piece,
                range_boost,
                result_boost,
            } => {
                let building = Building::from_data(&city_piece);
                let cost = ResourcePile::culture_tokens(range_boost + result_boost);
                let player = &mut game.players[player_index];
                let starting_city = player
                    .get_city(&starting_city_position)
                    .expect("player should have position");
                if matches!(building, Obelisk)
                    || starting_city_position.distance(&target_city_position)
                        > starting_city.size() + range_boost
                    || starting_city.player_index != player_index
                    || !player.resources().can_afford(&cost)
                {
                    panic!("Illegal action");
                }
                if !success {
                    return;
                }
                player.loose_resources(cost);
                game.influence_culture(
                    player_index,
                    target_player_index,
                    &target_city_position,
                    &building,
                );
            }
            Custom(custom_action) => custom_action.execute(game, player_index),
            EndTurn => unreachable!("end turn should be returned before executing the action"),
        }
    }

    pub fn action_type(&self) -> ActionType {
        match self {
            Custom(custom_action) => custom_action.custom_action_type().action_type(),
            _ => ActionType::default(),
        }
    }
}

#[derive(Default)]
pub struct ActionType {
    pub free: bool,
    pub once_per_turn: bool,
}

impl ActionType {
    pub fn free() -> Self {
        Self::new(true, false)
    }

    pub fn once_per_turn() -> Self {
        Self::new(false, true)
    }

    pub fn new(free: bool, once_per_turn: bool) -> Self {
        Self {
            free,
            once_per_turn,
        }
    }
}
