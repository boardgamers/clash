use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use PlayingAction::*;

use crate::game::{CulturalInfluenceResolution, GameState};
use crate::{
    city::City,
    city_pieces::Building::{self, *},
    combat,
    consts::{MOVEMENT_ACTIONS, PORT_CHOICES},
    content::custom_actions::CustomAction,
    game::{Game, UndoContext},
    map::Terrain,
    position::Position,
    resource_pile::ResourcePile,
    unit::UnitType,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum PlayingAction {
    Advance {
        advance: String,
        payment: ResourcePile,
    },
    FoundCity {
        settler: u32,
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
    Recruit {
        units: Vec<UnitType>,
        city_position: Position,
        payment: ResourcePile,
        leader_index: Option<usize>,
        replaced_units: Vec<u32>,
    },
    MoveUnits,
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
    ///
    ///
    /// # Panics
    ///
    /// Panics if action is illegal
    pub fn execute(self, game: &mut Game, player_index: usize) {
        if !self.action_type().free {
            assert_ne!(game.actions_left, 0, "Illegal action");
            game.actions_left -= 1;
        }
        match self {
            Advance { advance, payment } => {
                let player = &mut game.players[player_index];
                let cost = player.advance_cost(&advance);
                assert!(
                    player.can_advance(&advance)
                        && payment.food + payment.ideas + payment.gold as u32 == cost,
                    "Illegal action"
                );
                player.loose_resources(payment);
                game.advance(&advance, player_index);
            }
            FoundCity { settler } => {
                let settler = game.players[player_index]
                    .remove_unit(settler)
                    .expect("Illegal action");
                assert!(settler.can_found_city(game), "Illegal action");
                let player = &mut game.players[player_index];
                player.available_settlements -= 1;
                player.available_units.settlers += 1;
                let city = City::new(player_index, settler.position);
                player.cities.push(city);
                game.undo_context_stack
                    .push(UndoContext::FoundCity { settler });
            }
            Construct {
                city_position,
                city_piece,
                payment,
                port_position,
                temple_bonus,
            } => {
                let player = &mut game.players[player_index];
                let city = player.get_city(city_position).expect("Illegal action");
                let cost = player.construct_cost(&city_piece, city);
                assert!(
                    city.can_construct(&city_piece, player) && cost.is_valid_payment(&payment),
                    "Illegal action"
                );
                if matches!(&city_piece, Port) {
                    let port_position = port_position.as_ref().expect("Illegal action");
                    assert!(
                        city.position.neighbors().contains(port_position),
                        "Illegal action"
                    );
                } else if port_position.is_some() {
                    panic!("Illegal action");
                }
                if matches!(&city_piece, Temple) {
                    let building_bonus = temple_bonus.expect("Illegal action");
                    assert!(
                        building_bonus == ResourcePile::mood_tokens(1)
                            || building_bonus == ResourcePile::culture_tokens(1),
                        "Illegal action"
                    );
                    player.gain_resources(building_bonus);
                } else if temple_bonus.is_some() {
                    panic!("Illegal action");
                }
                player.loose_resources(payment);
                player.construct(&city_piece, city_position, port_position);
            }
            Collect {
                city_position,
                collections,
            } => {
                let total_collect =
                    get_total_collection(game, player_index, city_position, &collections)
                        .expect("Illegal action");
                let city = game.players[player_index]
                    .get_city_mut(city_position)
                    .expect("Illegal action");
                assert!(city.can_activate(), "Illegal action");
                city.activate();
                game.players[player_index].gain_resources(total_collect);
            }
            Recruit {
                units,
                city_position,
                payment,
                leader_index,
                replaced_units,
            } => {
                let cost = units.iter().map(UnitType::cost).sum::<ResourcePile>();
                let player = &mut game.players[player_index];
                assert!(
                    player.can_recruit(&units, city_position, leader_index, &replaced_units)
                        && cost.is_valid_payment(&payment)
                );
                player.loose_resources(payment);
                game.recruit(
                    player_index,
                    units,
                    city_position,
                    leader_index,
                    replaced_units,
                );
            }
            MoveUnits => {
                game.state = GameState::Movement {
                    movement_actions_left: MOVEMENT_ACTIONS,
                    moved_units: Vec::new(),
                }
            }
            IncreaseHappiness {
                happiness_increases,
            } => {
                let player = &mut game.players[player_index];
                for (city_position, steps) in happiness_increases {
                    let city = player.get_city(city_position).expect("Illegal action");
                    let cost = city.increase_happiness_cost(steps).expect("Illegal action");
                    assert!(
                        city.player_index == player_index && player.resources.can_afford(&cost),
                        "Illegal action"
                    );
                    player.loose_resources(cost);
                    let city = player.get_city_mut(city_position).expect("Illegal action");
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
                        starting_city_position,
                        target_player_index,
                        target_city_position,
                        &city_piece,
                    )
                    .expect("Illegal action");

                let self_influence = starting_city_position == target_city_position;

                game.players[player_index].loose_resources(range_boost_cost);
                let roll = combat::dice_value(game.get_next_dice_roll());
                let success = roll == 5 || roll == 6;
                if success {
                    game.influence_culture(
                        player_index,
                        target_player_index,
                        target_city_position,
                        &city_piece,
                    );
                    game.add_to_last_log_item(&format!(" and succeeded (rolled {roll})"));
                    return;
                }
                if self_influence {
                    game.add_to_last_log_item(&format!(" and failed (rolled {roll})"));
                    return;
                }
                if !game.players[player_index]
                    .resources
                    .can_afford(&ResourcePile::culture_tokens(5 - roll as u32))
                {
                    game.add_to_last_log_item(&format!(" but rolled a {roll} and has not enough culture tokens to increase the roll "));
                    return;
                }
                game.state = GameState::CulturalInfluenceResolution(CulturalInfluenceResolution {
                    roll_boost_cost: 5 - roll as u32,
                    target_player_index,
                    target_city_position,
                    city_piece,
                });
                game.add_to_last_log_item(&format!("and rolled a {roll}. {} now has the option to pay {} culture tokens to increase the dice roll and proceed with the cultural influence", game.players[player_index].get_name(), 5 - roll as u32));
            }
            Custom(custom_action) => {
                let action = custom_action.custom_action_type();
                assert!(
                    !game.played_once_per_turn_actions.contains(&action),
                    "Illegal action"
                );
                if action.action_type().once_per_turn {
                    game.played_once_per_turn_actions.push(action);
                }
                custom_action.execute(game, player_index);
            }
            EndTurn => game.next_turn(),
        }
    }

    #[must_use]
    pub fn action_type(&self) -> ActionType {
        match self {
            Custom(custom_action) => custom_action.custom_action_type().action_type(),
            EndTurn => ActionType::free(),
            _ => ActionType::default(),
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if no temple bonus is given when undoing a construct temple action
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
            FoundCity { settler: _ } => {
                let settler = game.undo_context_stack.pop();
                let Some(UndoContext::FoundCity { settler }) = settler else {
                    panic!("Settler context should be stored in undo context");
                };
                let player = &mut game.players[player_index];
                player.available_settlements += 1;
                player.available_units.settlers -= 1;
                player.units.push(settler);
                player
                    .cities
                    .pop()
                    .expect("The player should have a city after founding one");
            }
            Construct {
                city_position,
                city_piece,
                payment,
                port_position: _,
                temple_bonus,
            } => {
                let player = &mut game.players[player_index];
                player.undo_construct(&city_piece, city_position);
                player.gain_resources(payment);
                if matches!(&city_piece, Temple) {
                    player.loose_resources(
                        temple_bonus.expect("build data should contain temple bonus"),
                    );
                }
            }
            Collect {
                city_position,
                collections,
            } => {
                game.players[player_index]
                    .get_city_mut(city_position)
                    .expect("city should be owned by the player")
                    .undo_activate();
                let total_collect = collections.into_iter().map(|(_, collect)| collect).sum();
                game.players[player_index].loose_resources(total_collect);
            }
            Recruit {
                units,
                city_position,
                payment,
                leader_index,
                replaced_units: _,
            } => {
                game.players[player_index].gain_resources(payment);
                game.undo_recruit(player_index, &units, city_position, leader_index);
            }
            MoveUnits => game.state = GameState::Playing,
            IncreaseHappiness {
                happiness_increases,
            } => {
                let mut cost = 0;
                let player = &mut game.players[player_index];
                for (city_position, steps) in happiness_increases {
                    let city = player.get_city(city_position).expect("Illegal action");
                    cost += city.size() as u32 * steps;
                    let city = player.get_city_mut(city_position).expect("Illegal action");
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
    #[must_use]
    pub fn free() -> Self {
        Self::new(true, false)
    }

    #[must_use]
    pub fn once_per_turn() -> Self {
        Self::new(false, true)
    }

    #[must_use]
    pub fn free_and_once_per_turn() -> Self {
        Self::new(true, true)
    }

    #[must_use]
    fn new(free: bool, once_per_turn: bool) -> Self {
        Self {
            free,
            once_per_turn,
        }
    }
}

#[must_use]
pub fn get_total_collection(
    game: &Game,
    player_index: usize,
    city_position: Position,
    collections: &Vec<(Position, ResourcePile)>,
) -> Option<ResourcePile> {
    let player = &game.players[player_index];
    let city = player.get_city(city_position)?;
    if city.mood_modified_size() < collections.len() || city.player_index != player_index {
        return None;
    }
    let mut available_terrain = HashMap::new();
    add_collect_terrain(game, &mut available_terrain, city_position);
    for adjacent_tile in city.position.neighbors() {
        if game.get_any_city(adjacent_tile).is_some() {
            continue;
        }

        add_collect_terrain(game, &mut available_terrain, adjacent_tile);
    }
    let mut total_collect = ResourcePile::empty();
    for (position, collect) in collections {
        total_collect += collect.clone();

        let terrain = game.map.tiles.get(position)?.clone();

        if city.port_position == Some(*position) {
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

fn add_collect_terrain(
    game: &Game,
    available_terrain: &mut HashMap<Terrain, i32>,
    adjacent_tile: Position,
) {
    if let Some(terrain) = game.map.tiles.get(&adjacent_tile) {
        let terrain_left = available_terrain.entry(terrain.clone()).or_insert(0);
        if terrain == &Terrain::Water {
            *terrain_left = 1;
            return;
        }
        *terrain_left += 1;
    }
}
