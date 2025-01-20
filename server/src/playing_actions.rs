use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use PlayingAction::*;

use crate::action::Action;
use crate::content::advances;
use crate::game::{CulturalInfluenceResolution, CurrentMove, GameState, MoveState};
use crate::{
    city::City,
    city_pieces::Building::{self, *},
    consts::{MOVEMENT_ACTIONS, PORT_CHOICES},
    content::custom_actions::CustomAction,
    game::{Game, UndoContext},
    map::Terrain,
    position::Position,
    resource_pile::ResourcePile,
    unit::UnitType,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Construct {
    pub city_position: Position,
    pub city_piece: Building,
    pub payment: ResourcePile,
    pub port_position: Option<Position>,
    pub temple_bonus: Option<ResourcePile>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Collect {
    pub city_position: Position,
    pub collections: Vec<(Position, ResourcePile)>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Recruit {
    pub units: Vec<UnitType>,
    pub city_position: Position,
    pub payment: ResourcePile,
    pub leader_index: Option<usize>,
    pub replaced_units: Vec<u32>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct InfluenceCultureAttempt {
    pub starting_city_position: Position,
    pub target_player_index: usize,
    pub target_city_position: Position,
    pub city_piece: Building,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct IncreaseHappiness {
    pub happiness_increases: Vec<(Position, u32)>,
}

#[derive(Clone, Copy)]
pub enum PlayingActionType {
    Advance,
    FoundCity,
    Construct,
    Collect,
    Recruit,
    MoveUnits,
    IncreaseHappiness,
    InfluenceCultureAttempt,
    Custom,
    EndTurn,
}

impl PlayingActionType {
    #[must_use]
    pub fn is_available(&self, game: &Game, player_index: usize) -> bool {
        let mut possible = true;
        let p = &game.players[player_index];
        p.get_events()
            .is_playing_action_available
            .trigger(&mut possible, self, p);
        possible
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum PlayingAction {
    Advance {
        advance: String,
        payment: ResourcePile,
    },
    FoundCity {
        settler: u32,
    },
    Construct(Construct),
    Collect(Collect),
    Recruit(Recruit),
    MoveUnits,
    IncreaseHappiness(IncreaseHappiness),
    InfluenceCultureAttempt(InfluenceCultureAttempt),
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
        assert!(
            self.playing_action_type().is_available(game, player_index),
            "Illegal action"
        );
        if !self.action_type().free {
            assert_ne!(game.actions_left, 0, "Illegal action");
            game.actions_left -= 1;
        }
        game.players[player_index].loose_resources(self.action_type().cost);

        match self {
            Advance { advance, payment } => {
                let player = &mut game.players[player_index];
                let cost = player.advance_cost(&advance);
                assert!(
                    player.can_advance(&advances::get_advance_by_name(&advance))
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
                let city = City::new(player_index, settler.position);
                player.cities.push(city);
                game.undo_context_stack
                    .push(UndoContext::FoundCity { settler });
            }
            Construct(c) => {
                let player = &game.players[player_index];
                let city = player.get_city(c.city_position).expect("Illegal action");
                let cost = player.construct_cost(c.city_piece, city);
                assert!(
                    city.can_construct(c.city_piece, player, game)
                        && cost.is_valid_payment(&c.payment),
                    "Illegal action"
                );
                if matches!(c.city_piece, Port) {
                    let port_position = c.port_position.as_ref().expect("Illegal action");
                    assert!(
                        city.position.neighbors().contains(port_position),
                        "Illegal action"
                    );
                } else if c.port_position.is_some() {
                    panic!("Illegal action");
                }
                let player_mut = &mut game.players[player_index];
                if matches!(c.city_piece, Temple) {
                    let building_bonus = c.temple_bonus.expect("Illegal action");
                    assert!(
                        building_bonus == ResourcePile::mood_tokens(1)
                            || building_bonus == ResourcePile::culture_tokens(1),
                        "Illegal action"
                    );
                    player_mut.gain_resources(building_bonus);
                } else if c.temple_bonus.is_some() {
                    panic!("Illegal action");
                }
                player_mut.loose_resources(c.payment);
                player_mut.construct(c.city_piece, c.city_position, c.port_position);
            }
            Collect(c) => {
                if game.action_log.iter().any(|a| {
                    matches!(
                        a,
                        Action::Playing(PlayingAction::Custom(CustomAction::FreeEconomyCollect(_)))
                    )
                }) {
                    assert!(game.state == GameState::Playing, "Illegal action");
                }
                collect(game, player_index, &c);
            }
            Recruit(r) => {
                let cost = r.units.iter().map(UnitType::cost).sum::<ResourcePile>();
                let player = &mut game.players[player_index];
                assert!(
                    player.can_recruit(
                        &r.units,
                        r.city_position,
                        r.leader_index,
                        &r.replaced_units
                    ) && cost.is_valid_payment(&r.payment)
                );
                player.loose_resources(r.payment);
                game.recruit(
                    player_index,
                    r.units,
                    r.city_position,
                    r.leader_index,
                    r.replaced_units,
                );
            }
            MoveUnits => {
                game.state = GameState::Movement(MoveState {
                    movement_actions_left: MOVEMENT_ACTIONS,
                    moved_units: Vec::new(),
                    current_move: CurrentMove::None,
                });
            }
            IncreaseHappiness(i) => {
                increase_happiness(game, player_index, i);
            }
            InfluenceCultureAttempt(c) => {
                let starting_city_position = c.starting_city_position;
                let target_player_index = c.target_player_index;
                let target_city_position = c.target_city_position;
                let city_piece = c.city_piece;
                let range_boost_cost = game
                    .influence_culture_boost_cost(
                        player_index,
                        starting_city_position,
                        target_player_index,
                        target_city_position,
                        city_piece,
                    )
                    .expect("Illegal action");

                let self_influence = starting_city_position == target_city_position;

                game.players[player_index].loose_resources(range_boost_cost);
                let roll = game.get_next_dice_roll().value;
                let success = roll == 5 || roll == 6;
                if success {
                    game.influence_culture(
                        player_index,
                        target_player_index,
                        target_city_position,
                        city_piece,
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
                game.add_to_last_log_item(&format!(" and rolled a {roll}. {} now has the option to pay {} culture tokens to increase the dice roll and proceed with the cultural influence", game.players[player_index].get_name(), 5 - roll as u32));
            }
            Custom(custom_action) => {
                let action = custom_action.custom_action_type();
                assert!(
                    !game
                        .get_player(player_index)
                        .played_once_per_turn_actions
                        .contains(&action),
                    "Already played once per turn"
                );
                assert!(action.is_available(game, player_index), "Not available");
                if action.action_type().once_per_turn {
                    game.players[player_index]
                        .played_once_per_turn_actions
                        .push(action);
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
            EndTurn => ActionType::free(ResourcePile::empty()),
            _ => ActionType::default(),
        }
    }

    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        match self {
            PlayingAction::Advance { .. } => PlayingActionType::Advance,
            PlayingAction::FoundCity { .. } => PlayingActionType::FoundCity,
            PlayingAction::Construct { .. } => PlayingActionType::Construct,
            PlayingAction::Collect { .. } => PlayingActionType::Collect,
            PlayingAction::Recruit { .. } => PlayingActionType::Recruit,
            PlayingAction::MoveUnits => PlayingActionType::MoveUnits,
            PlayingAction::IncreaseHappiness { .. } => PlayingActionType::IncreaseHappiness,
            PlayingAction::InfluenceCultureAttempt { .. } => {
                PlayingActionType::InfluenceCultureAttempt
            }
            PlayingAction::Custom(_) => PlayingActionType::Custom,
            PlayingAction::EndTurn => PlayingActionType::EndTurn,
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
        game.players[player_index].gain_resources(self.action_type().cost);

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
                player.units.push(settler);
                player
                    .cities
                    .pop()
                    .expect("The player should have a city after founding one");
            }
            Construct(c) => {
                let player = &mut game.players[player_index];
                player.undo_construct(c.city_piece, c.city_position);
                player.gain_resources(c.payment);
                if matches!(c.city_piece, Temple) {
                    player.loose_resources(
                        c.temple_bonus
                            .expect("build data should contain temple bonus"),
                    );
                }
            }
            Collect(c) => undo_collect(game, player_index, c),
            Recruit(r) => {
                game.players[player_index].gain_resources(r.payment);
                game.undo_recruit(player_index, &r.units, r.city_position, r.leader_index);
            }
            MoveUnits => game.state = GameState::Playing,
            IncreaseHappiness(i) => undo_increase_happiness(game, player_index, i),
            Custom(custom_action) => custom_action.undo(game, player_index),
            InfluenceCultureAttempt(_) | EndTurn => panic!("Action can't be undone"),
        }
    }
}

#[derive(Default)]
pub struct ActionType {
    pub free: bool,
    pub once_per_turn: bool,
    pub cost: ResourcePile,
}

impl ActionType {
    #[must_use]
    pub fn free(cost: ResourcePile) -> Self {
        Self::new(true, false, cost)
    }

    #[must_use]
    pub fn once_per_turn(cost: ResourcePile) -> Self {
        Self::new(false, true, cost)
    }

    #[must_use]
    pub fn free_and_once_per_turn(cost: ResourcePile) -> Self {
        Self::new(true, true, cost)
    }

    #[must_use]
    fn new(free: bool, once_per_turn: bool, cost: ResourcePile) -> Self {
        Self {
            free,
            once_per_turn,
            cost,
        }
    }
}

///
/// # Panics
///
/// Panics if the action is illegal
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

        let terrain = game
            .map
            .get(*position)
            .expect("Position should be on the map");

        if city.port_position == Some(*position) {
            if !PORT_CHOICES.iter().any(|r| r == collect) {
                return None;
            }
        } else if !player
            .collect_options
            .get(terrain)
            .is_some_and(|o| o.contains(collect))
        {
            return None;
        }
        let terrain_left = available_terrain.entry(terrain.clone()).or_insert(0);
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
    if let Some(terrain) = game.map.get(adjacent_tile) {
        let terrain_left = available_terrain.entry(terrain.clone()).or_insert(0);
        if terrain == &Terrain::Water {
            *terrain_left = 1;
            return;
        }
        *terrain_left += 1;
    }
}

pub(crate) fn collect(game: &mut Game, player_index: usize, c: &Collect) {
    let total_collect = get_total_collection(game, player_index, c.city_position, &c.collections)
        .expect("Illegal action");
    let city = game.players[player_index]
        .get_city_mut(c.city_position)
        .expect("Illegal action");
    assert!(city.can_activate(), "Illegal action");
    city.activate();
    game.players[player_index].gain_resources(total_collect);
}

pub(crate) fn undo_collect(game: &mut Game, player_index: usize, c: Collect) {
    game.players[player_index]
        .get_city_mut(c.city_position)
        .expect("city should be owned by the player")
        .undo_activate();
    let total_collect = c.collections.into_iter().map(|(_, collect)| collect).sum();
    game.players[player_index].loose_resources(total_collect);
}

pub(crate) fn increase_happiness(game: &mut Game, player_index: usize, i: IncreaseHappiness) {
    let player = &mut game.players[player_index];
    for (city_position, steps) in i.happiness_increases {
        let city = player.get_city(city_position).expect("Illegal action");
        let cost = city.increase_happiness_cost(steps).expect("Illegal action");
        player.loose_resources(cost);
        let city = player.get_city_mut(city_position).expect("Illegal action");
        for _ in 0..steps {
            city.increase_mood_state();
        }
    }
}

pub(crate) fn undo_increase_happiness(game: &mut Game, player_index: usize, i: IncreaseHappiness) {
    let mut cost = 0;
    let player = &mut game.players[player_index];
    for (city_position, steps) in i.happiness_increases {
        let city = player.get_city(city_position).expect("Illegal action");
        cost += city.size() as u32 * steps;
        let city = player.get_city_mut(city_position).expect("Illegal action");
        for _ in 0..steps {
            city.decrease_mood_state();
        }
    }
    player.gain_resources(ResourcePile::mood_tokens(cost));
}
