use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use PlayingAction::*;

use crate::{
    city_pieces::Building::{self, *},
    content::custom_actions::CustomAction,
    game::{Game, GameState::*},
    map::Terrain,
    position::Position,
    resource_pile::ResourcePile,
};

pub const PORT_CHOICES: [ResourcePile; 3] = [
    ResourcePile {
        food: 1,
        wood: 0,
        ore: 0,
        ideas: 0,
        gold: 0,
        mood_tokens: 0,
        culture_tokens: 0,
    },
    ResourcePile {
        food: 0,
        wood: 0,
        ore: 0,
        ideas: 0,
        gold: 1,
        mood_tokens: 0,
        culture_tokens: 0,
    },
    ResourcePile {
        food: 0,
        wood: 0,
        ore: 0,
        ideas: 0,
        gold: 0,
        mood_tokens: 1,
        culture_tokens: 0,
    },
];

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum PlayingAction {
    Advance {
        advance: String,
        payment: ResourcePile,
    },
    Construct {
        city_position: Position,
        city_piece: Building,
        payment: ResourcePile,
        port_position: Option<Position>,
        temple_bonus: Option<ResourcePile>,
    },
    Collect {
        city_position: Position,
        collections: Vec<(Position, ResourcePile)>,
    },
    IncreaseHappiness {
        happiness_increases: Vec<(Position, u32)>,
    },
    InfluenceCultureAttempt {
        starting_city_position: Position,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: Building,
    },
    Custom(CustomAction),
    EndTurn,
}

impl PlayingAction {
    pub fn execute(self, game: &mut Game, player_index: usize) {
        if !self.action_type().free {
            if game.actions_left == 0 {
                panic!("Illegal action");
            }
            game.actions_left -= 1;
        }
        match self {
            Advance { advance, payment } => {
                let player = &mut game.players[player_index];
                let cost = player.advance_cost(&advance);
                if !player.can_advance(&advance)
                    || payment.food + payment.ideas + payment.gold as u32 != cost
                {
                    panic!("Illegal action");
                }
                player.loose_resources(payment);
                game.advance(&advance, player_index);
            }
            Construct {
                city_position,
                city_piece,
                payment,
                port_position,
                temple_bonus,
            } => {
                let player = &mut game.players[player_index];
                let city = player.get_city(&city_position).expect("Illegal action");
                let cost = player.construct_cost(&city_piece, city);
                if !city.can_construct(&city_piece, player) || !payment.can_afford(&cost) {
                    panic!("Illegal action");
                }
                if matches!(&city_piece, Port) {
                    let port_position = port_position.as_ref().expect("Illegal action");
                    if !city.position.neighbors().contains(port_position) {
                        panic!("Illegal action");
                    }
                } else if port_position.is_some() {
                    panic!("Illegal action");
                }
                if matches!(&city_piece, Temple) {
                    let building_bonus = temple_bonus.expect("Illegal action");
                    if building_bonus != ResourcePile::mood_tokens(1)
                        && building_bonus != ResourcePile::culture_tokens(1)
                    {
                        panic!("Illegal action");
                    }
                    player.gain_resources(building_bonus);
                } else if temple_bonus.is_some() {
                    panic!("Illegal action");
                }
                player.loose_resources(payment);
                player.construct(&city_piece, &city_position, port_position);
            }
            Collect {
                city_position,
                collections,
            } => {
                let total_collect =
                    get_total_collection(game, player_index, &city_position, &collections)
                        .expect("Illegal action");
                game.players[player_index].gain_resources(total_collect);
                game.players[player_index]
                    .get_city_mut(&city_position)
                    .expect("Illegal action")
                    .activate();
            }
            IncreaseHappiness {
                happiness_increases,
            } => {
                let player = &mut game.players[player_index];
                for (city_position, steps) in happiness_increases {
                    let city = player.get_city(&city_position).expect("Illegal action");
                    let cost = city.increase_happiness_cost(steps).expect("Illegal action");
                    if city.player_index != player_index || !player.resources().can_afford(&cost) {
                        panic!("Illegal action");
                    }
                    player.loose_resources(cost);
                    let city = player.get_city_mut(&city_position).expect("Illegal action");
                    for _ in 0..steps {
                        city.increase_mood_state();
                    }
                }
            }
            InfluenceCultureAttempt {
                starting_city_position,
                target_player_index,
                target_city_position,
                city_piece,
            } => {
                let range_boost_cost = game
                    .influence_culture_boost_cost(
                        player_index,
                        &starting_city_position,
                        target_player_index,
                        &target_city_position,
                        &city_piece,
                    )
                    .expect("Illegal action");

                let self_influence = starting_city_position == target_city_position;

                game.players[player_index].loose_resources(range_boost_cost);
                let roll = game.get_next_dice_roll();
                let success = roll == 5 || roll == 6;
                if success {
                    game.influence_culture(
                        player_index,
                        target_player_index,
                        &target_city_position,
                        &city_piece,
                    );
                    game.add_to_last_log_item(&format!(" and succeeded (rolled {roll})"));
                    return;
                }
                if roll > 6 || self_influence {
                    game.add_to_last_log_item(&format!(" and failed (rolled {roll})"));
                    return;
                }
                if !game.players[player_index]
                    .resources()
                    .can_afford(&ResourcePile::culture_tokens(5 - roll as u32))
                {
                    game.add_to_last_log_item(&format!(" but rolled a {roll} and has not enough culture tokens to increase the roll "));
                    return;
                }
                game.state = CulturalInfluenceResolution {
                    roll_boost_cost: 5 - roll as u32,
                    target_player_index,
                    target_city_position,
                    city_piece,
                };
                game.add_to_last_log_item(&format!("and rolled a {roll}. {} now has the option to pay {} culture tokens to increase the dice roll and proceed with the cultural influence", game.players[player_index].get_name(), 5 - roll as u32))
            }
            Custom(custom_action) => {
                let action = custom_action.custom_action_type();
                if game.played_once_per_turn_actions.contains(&action) {
                    panic!("Illegal action");
                }
                if action.action_type().once_per_turn {
                    game.played_once_per_turn_actions.push(action);
                }
                custom_action.execute(game, player_index)
            }
            EndTurn => game.next_turn(),
        }
    }

    pub fn action_type(&self) -> ActionType {
        match self {
            Custom(custom_action) => custom_action.custom_action_type().action_type(),
            EndTurn => ActionType::free(),
            _ => ActionType::default(),
        }
    }

    pub fn undo(self, game: &mut Game, player_index: usize) {
        let free_action = self.action_type().free;
        if !free_action {
            game.actions_left += 1;
        }
        match self {
            Advance { advance, payment } => {
                let player = &mut game.players[player_index];
                player.gain_resources(payment);
                game.undo_advance(&advance, player_index);
            }
            Construct {
                city_position,
                city_piece,
                payment,
                port_position: _,
                temple_bonus,
            } => {
                let player = &mut game.players[player_index];
                player.undo_construct(&city_piece, &city_position);
                player.gain_resources(payment);
                if matches!(&city_piece, Temple) {
                    player.loose_resources(
                        temple_bonus.expect("build data should contain temple bonus"),
                    );
                }
            }
            Collect {
                city_position: _,
                collections,
            } => {
                let total_collect = collections.into_iter().map(|(_, collect)| collect).sum();
                game.players[player_index].loose_resources(total_collect);
            }
            IncreaseHappiness {
                happiness_increases,
            } => {
                let mut cost = 0;
                let player = &mut game.players[player_index];
                for (city_position, steps) in happiness_increases {
                    let city = player.get_city(&city_position).expect("Illegal action");
                    cost += city.size() as u32 * steps;
                    let city = player.get_city_mut(&city_position).expect("Illegal action");
                    for _ in 0..steps {
                        city.decrease_mood_state();
                    }
                }
                player.gain_resources(ResourcePile::mood_tokens(cost));
            }
            Custom(custom_action) => custom_action.undo(game, player_index),
            InfluenceCultureAttempt { .. } | EndTurn => panic!("Action can't be undone"),
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

    pub fn free_and_once_per_turn() -> Self {
        Self::new(true, true)
    }

    fn new(free: bool, once_per_turn: bool) -> Self {
        Self {
            free,
            once_per_turn,
        }
    }
}

pub fn get_total_collection(
    game: &Game,
    player_index: usize,
    city_position: &Position,
    collections: &Vec<(Position, ResourcePile)>,
) -> Option<ResourcePile> {
    let player = &game.players[player_index];
    let city = player.get_city(city_position)?;
    if city.mood_modified_size() < collections.len() || city.player_index != player_index {
        return None;
    }
    let mut available_terrain = HashMap::new();
    for adjacent_tile in city.position.neighbors() {
        if game.get_any_city(&adjacent_tile).is_some() {
            continue;
        }

        let Some(terrain) = game.map.tiles.get(&adjacent_tile) else {
            continue;
        };
        let terrain_left = available_terrain.entry(terrain.clone()).or_insert(0);
        if terrain == &Terrain::Water {
            *terrain_left = 1;
            continue;
        }
        *terrain_left += 1;
    }
    let mut total_collect = ResourcePile::empty();
    for (position, collect) in collections.iter() {
        total_collect += collect.clone();

        let terrain = game.map.tiles.get(position)?.clone();

        if city.port_position == Some(position.clone()) {
            if !PORT_CHOICES.iter().any(|r| r == collect) {
                return None;
            }
        } else if !player
            .collect_options
            .get(&terrain)
            .is_some_and(|o| o.contains(collect))
        {
            return None;
        }
        let terrain_left = available_terrain.entry(terrain).or_insert(0);
        *terrain_left -= 1;
        if *terrain_left < 0 {
            return None;
        }
    }
    Some(total_collect)
}
