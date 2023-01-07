use serde::{Deserialize, Serialize};

use crate::{
    city::{
        City, CityData,
        CityPiece::{self, *},
        CityPieceData, BUILDING_COST,
    },
    content::custom_actions,
    player::Player,
    resource_pile::ResourcePile,
};

use PlayingAction::*;

#[derive(Serialize, Deserialize)]
pub enum PlayingAction {
    Advance {
        technology: String,
        payment: ResourcePile,
    },
    Build {
        city: CityData,
        city_piece: CityPieceData,
        payment: ResourcePile,
    },
    IncreaseHappiness {
        cities: Vec<(CityData, u32)>,
    },
    InfluenceCulture {
        success: bool,
        starting_city: Box<CityData>,
        target_city: CityData,
        city_piece: CityPieceData,
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
    pub fn execute(self, player: &mut Player, user_specification: Option<String>) {
        let player_name = player.name();
        match self {
            Advance {
                technology,
                payment,
            } => {
                if !player.can_research_technology(&technology)
                    || payment.food + payment.ideas + payment.gold as u32 != 2
                {
                    panic!("Illegal action");
                }
                player.loose_resources(payment);
                player.research_technology(&technology);
            }
            Build {
                city,
                city_piece,
                payment,
            } => {
                let city = City::from_data(city);
                let city_piece = CityPiece::from_data(&city_piece);
                let mut cost = BUILDING_COST;
                player
                    .events()
                    .city_size_increase_cost
                    .trigger(&mut cost, &city, &city_piece);
                if city.player != player_name
                    || !city.can_increase_size(&city_piece, player)
                    || !payment.can_afford(&cost)
                {
                    panic!("Illegal action");
                }
                if matches!(city_piece, Temple) {
                    let building_bonus = serde_json::from_str(
                        &user_specification.expect("user should have specified the building bonus"),
                    )
                    .expect("use should have specified the building bonus");
                    if building_bonus != ResourcePile::mood_tokens(1)
                        && building_bonus != ResourcePile::culture_tokens(1)
                    {
                        panic!("Illegal action");
                    }
                    player.gain_resources(building_bonus);
                }
                player.loose_resources(payment);
                let mut city = player.cities.remove(
                    player
                        .cities
                        .iter()
                        .position(|player_city| player_city.position == city.position)
                        .expect("city should exist"),
                );
                city.increase_size(city_piece, player);
                player.cities.push(city);
            }
            IncreaseHappiness { cities } => {
                for (city, steps) in cities {
                    let city = City::from_data(city);
                    let cost = ResourcePile::mood_tokens(city.size()) * steps;
                    if city.player != player_name || !player.resources().can_afford(&cost) {
                        panic!("Illegal action");
                    }
                    player.loose_resources(cost);
                    for player_city in player.cities.iter_mut() {
                        if player_city.position == city.position {
                            for _ in 0..steps {
                                player_city.increase_mood_state();
                            }
                        }
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
                let city_piece = CityPiece::from_data(&city_piece);
                let cost = ResourcePile::culture_tokens(range_boost + result_boost);
                if matches!(city_piece, Obelisk)
                    || matches!(city_piece, Wonder(_))
                    || starting_city.position.distance(&target_city.position)
                        > starting_city.size() + range_boost
                    || starting_city.player != player_name
                    || !player.resources().can_afford(&cost)
                {
                    panic!("Illegal action");
                }
                if !success {
                    return;
                }
                player.loose_resources(cost);
                todo!()
            }
            Custom { name, contents } => {
                custom_actions::get_custom_action(&name, &contents).execute(player)
            }
            EndTurn => unreachable!(),
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
    fn execute(&self, player: &mut Player);
    fn action_type(&self) -> ActionType;
    fn name(&self) -> String;
}

#[derive(Default)]
pub struct ActionType {
    pub free: bool,
    pub once_per_turn: bool,
}

impl ActionType {
    pub fn new(free: bool, once_per_turn: bool) -> Self {
        Self {
            free,
            once_per_turn,
        }
    }
}
