use serde::{Deserialize, Serialize};

use crate::{
    city::{
        Building::{self, *},
        BuildingData, City, CityData,
    },
    content::custom_actions,
    game::Game,
    player::Player,
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
    pub fn execute(self, player: &mut Player, game: &mut Game) {
        let player_name = player.name();
        match self {
            Advance { advance, payment } => {
                if !player.can_advance(&advance)
                    || payment.food + payment.ideas + payment.gold as u32 != 2
                {
                    panic!("Illegal action");
                }
                player.loose_resources(payment);
                player.advance(&advance);
            }
            Build {
                city,
                city_piece,
                payment,
                temple_bonus,
            } => {
                let city = City::from_data(city);
                let building = Building::from_data(&city_piece);
                let cost = player.building_cost(&building, &city);
                if city.player != player_name
                    || !city.can_increase_size(&building, player)
                    || !payment.can_afford(&cost)
                {
                    panic!("Illegal action");
                }
                if matches!(building, Temple) {
                    let building_bonus =
                        temple_bonus.expect("build data should contain temple bonus");
                    if building_bonus != ResourcePile::mood_tokens(1)
                        && building_bonus != ResourcePile::culture_tokens(1)
                    {
                        panic!("Illegal action");
                    }
                    player.gain_resources(building_bonus);
                }
                player.loose_resources(payment);
                let mut city = player.take_city(&city.position).expect("city should exist");
                city.increase_size(&building, player);
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
                    let city = player
                        .get_city(&city.position)
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
                if matches!(building, Obelisk)
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

                //todo! in the future get the city directly from its position on the map instead
                let target_player = &target_city.player;
                let target_player = game.get_player(target_player).expect("player should exist");
                let target_city = target_player
                    .get_city(&target_city.position)
                    .expect("city should exist");
                target_city.influence_culture(player, &building);
            }
            Custom { name, contents } => {
                custom_actions::get_custom_action(&name, &contents).execute(player)
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
