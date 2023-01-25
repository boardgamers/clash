use serde::{Deserialize, Serialize};

use crate::{
    city::{
        Building::{self, *},
        BuildingData, City, CityData,
    },
    content::custom_actions,
    game::Game,
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
        city: CityData,
        city_piece: BuildingData,
        payment: ResourcePile,
        temple_bonus: Option<ResourcePile>,
    },
    IncreaseHappiness {
        cities: Vec<(CityData, u32)>,
    },
    InfluenceCulture {
        success: bool,
        starting_city: Box<CityData>,
        target_city: CityData,
        city_piece: BuildingData,
        range_boost: u32,
        result_boost: u32,
    },
    Custom {
        name: String,
        contents: String,
    },
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
                city,
                city_piece,
                payment,
                temple_bonus,
            } => {
                let city = City::from_data(city);
                let building = Building::from_data(&city_piece);
                let player = &mut game.players[player_index];
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
                player.increase_size(&building, &city.position);
            }
            IncreaseHappiness { cities } => {
                for (city, steps) in cities {
                    let city = City::from_data(city);
                    let cost = ResourcePile::mood_tokens(city.size()) * steps;
                    let player = &mut game.players[player_index];
                    if city.player_index != player_index || !player.resources().can_afford(&cost) {
                        panic!("Illegal action");
                    }
                    player.loose_resources(cost);
                    let city = player
                        .get_city_mut(&city.position)
                        .expect("player should have city");
                    for _ in 0..steps {
                        city.increase_mood_state();
                    }
                }
            }
            InfluenceCulture {
                success,
                starting_city,
                target_city,
                city_piece,
                range_boost,
                result_boost,
            } => {
                let starting_city = City::from_data(*starting_city);
                let target_city = City::from_data(target_city);
                let building = Building::from_data(&city_piece);
                let cost = ResourcePile::culture_tokens(range_boost + result_boost);
                let player = &mut game.players[player_index];
                if matches!(building, Obelisk)
                    || starting_city.position.distance(&target_city.position)
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

                //todo! in the future get the city directly from its position on the map instead
                let target_city = game.players[target_city.player_index]
                    .get_city_mut(&target_city.position)
                    .expect("city should exist");
                //todo! influence culture in target city
            }
            Custom { name, contents } => {
                custom_actions::get_custom_action(&name, &contents).execute(game, player_index)
            }
            EndTurn => unreachable!("end turn should be returned before executing the action"),
        }
    }

    pub fn action_type(&self) -> ActionType {
        match self {
            Custom { name, contents } => {
                custom_actions::get_custom_action(name, contents).action_type()
            }
            _ => ActionType::default(),
        }
    }
}

pub trait CustomAction {
    fn execute(&self, game: &mut Game, player_index: usize);
    fn action_type(&self) -> ActionType;
    fn name(&self) -> String;
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