use serde::{Deserialize, Serialize};

use PlayingAction::*;

use crate::action::Action;
use crate::city::MoodState;
use crate::collect::{collect, undo_collect};
use crate::content::advances::get_advance;
use crate::content::custom_phase_actions::CustomPhaseEventType;
use crate::game::{CulturalInfluenceResolution, GameState};
use crate::payment::PaymentOptions;
use crate::player_events::CostInfo;
use crate::unit::{Unit, Units};
use crate::{
    city::City,
    city_pieces::Building::{self, *},
    content::custom_actions::CustomAction,
    game::{Game, UndoContext},
    position::Position,
    resource_pile::ResourcePile,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Construct {
    pub city_position: Position,
    pub city_piece: Building,
    pub payment: ResourcePile,
    pub port_position: Option<Position>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Collect {
    pub city_position: Position,
    pub collections: Vec<(Position, ResourcePile)>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Recruit {
    pub units: Units,
    pub city_position: Position,
    pub payment: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader_name: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    pub payment: ResourcePile,
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
        let _ = p.trigger_event(|e| &e.is_playing_action_available, &mut possible, self, p);
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
        game.players[player_index].lose_resources(self.action_type().cost);

        match self {
            Advance { advance, payment } => {
                game.get_player(player_index)
                    .advance_cost(&get_advance(&advance), Some(payment.clone()))
                    .execute(game, &payment);
                game.advance(&advance, player_index, payment);
            }
            FoundCity { settler } => {
                let settler = game.players[player_index].remove_unit(settler);
                assert!(settler.can_found_city(game), "Illegal action");
                let player = &mut game.players[player_index];
                let city = City::new(player_index, settler.position);
                player.cities.push(city);
                let unit_data = settler.data(player);
                game.push_undo_context(UndoContext::FoundCity { settler: unit_data });
            }
            Construct(c) => {
                let player = &game.players[player_index];
                let city = player.get_city(c.city_position).expect("Illegal action");
                let cost = player.construct_cost(c.city_piece, city, Some(c.payment.clone()));
                assert!(
                    city.can_construct(c.city_piece, player, game),
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
                game.players[player_index].construct(
                    c.city_piece,
                    c.city_position,
                    c.port_position,
                );
                cost.execute(game, &c.payment);
                Self::on_construct(game, player_index, c.city_piece);
            }
            Collect(c) => {
                if game.action_log.iter().any(|i| {
                    matches!(
                        i.action,
                        Action::Playing(PlayingAction::Custom(CustomAction::FreeEconomyCollect(_)))
                    )
                }) {
                    assert!(game.state == GameState::Playing, "Illegal action");
                }
                collect(game, player_index, &c);
            }
            Recruit(r) => {
                let player = &mut game.players[player_index];
                if let Some(cost) = player.recruit_cost(
                    &r.units,
                    r.city_position,
                    r.leader_name.as_ref(),
                    &r.replaced_units,
                    Some(r.payment.clone()),
                ) {
                    cost.execute(game, &r.payment);
                } else {
                    panic!("Cannot pay for units")
                }
                game.recruit(
                    player_index,
                    r.units,
                    r.city_position,
                    r.leader_name.as_ref(),
                    &r.replaced_units,
                );
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

                // currectly, there is no way to have different costs for this
                game.players[player_index].lose_resources(range_boost_cost.default);
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
                if let Some(roll_boost_cost) =
                    PaymentOptions::resources(Self::roll_boost_cost(roll))
                        .first_valid_payment(&game.players[player_index].resources)
                {
                    game.add_to_last_log_item(&format!(" and rolled a {roll}. {} now has the option to pay {roll_boost_cost} to increase the dice roll and proceed with the cultural influence", game.players[player_index].get_name()));
                    game.state =
                        GameState::CulturalInfluenceResolution(CulturalInfluenceResolution {
                            roll_boost_cost,
                            target_player_index,
                            target_city_position,
                            city_piece,
                        });
                } else {
                    game.add_to_last_log_item(&format!(" but rolled a {roll} and has not enough culture tokens to increase the roll "));
                }
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

    pub(crate) fn on_construct(game: &mut Game, player_index: usize, building: Building) {
        game.trigger_custom_phase_event(
            player_index,
            |e| &mut e.on_construct,
            CustomPhaseEventType::OnConstruct,
            &building,
        );
    }

    pub(crate) fn roll_boost_cost(roll: u8) -> ResourcePile {
        ResourcePile::culture_tokens(5 - roll as u32)
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
    pub fn undo(self, game: &mut Game, player_index: usize, was_custom_phase: bool) {
        let free_action = self.action_type().free;
        if !free_action {
            game.actions_left += 1;
        }
        game.players[player_index].gain_resources_in_undo(self.action_type().cost);

        match self {
            Advance { advance, payment } => {
                let player = &mut game.players[player_index];
                player.gain_resources_in_undo(payment);
                game.undo_advance(&get_advance(&advance), player_index, was_custom_phase);
            }
            FoundCity { settler: _ } => {
                let Some(UndoContext::FoundCity { settler }) = game.pop_undo_context() else {
                    panic!("Settler context should be stored in undo context");
                };
                let player = &mut game.players[player_index];
                let units = Unit::from_data(player_index, settler);
                player.units.push(
                    units
                        .into_iter()
                        .next()
                        .expect("The player should have a unit after founding a city"),
                );
                player
                    .cities
                    .pop()
                    .expect("The player should have a city after founding one");
            }
            Construct(c) => {
                let player = &mut game.players[player_index];
                player.undo_construct(c.city_piece, c.city_position);
                player.gain_resources_in_undo(c.payment);
            }
            Collect(c) => undo_collect(game, player_index, c),
            Recruit(r) => {
                game.players[player_index].gain_resources_in_undo(r.payment);
                game.undo_recruit(
                    player_index,
                    r.units,
                    r.city_position,
                    r.leader_name.as_ref(),
                );
            }
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

pub(crate) fn increase_happiness(game: &mut Game, player_index: usize, i: IncreaseHappiness) {
    let player = &mut game.players[player_index];
    let mut total_cost: Option<CostInfo> = None;
    let mut angry_activations = vec![];
    for (city_position, steps) in i.happiness_increases {
        let city = player.get_city(city_position).expect("Illegal action");
        let cost = player
            .increase_happiness_cost(city, steps, None)
            .expect("Illegal action");
        if steps == 0 {
            continue;
        }
        if city.mood_state == MoodState::Angry {
            angry_activations.push(city_position);
        }
        if let Some(ref mut total_cost) = total_cost {
            total_cost.cost.default += cost.cost.default;
        } else {
            total_cost = Some(cost);
        }
        let city = player.get_city_mut(city_position).expect("Illegal action");
        for _ in 0..steps {
            city.increase_mood_state();
        }
    }
    total_cost
        .expect("cost should be set")
        .execute(game, &i.payment);
    game.push_undo_context(UndoContext::IncreaseHappiness { angry_activations });
}

pub(crate) fn undo_increase_happiness(game: &mut Game, player_index: usize, i: IncreaseHappiness) {
    let player = &mut game.players[player_index];
    for (city_position, steps) in i.happiness_increases {
        let city = player.get_city_mut(city_position).expect("Illegal action");
        for _ in 0..steps {
            city.decrease_mood_state();
        }
    }
    player.gain_resources_in_undo(i.payment);

    if let Some(UndoContext::IncreaseHappiness { angry_activations }) = game.pop_undo_context() {
        for city_position in angry_activations {
            let city = game.players[player_index]
                .get_city_mut(city_position)
                .expect("Illegal action");
            city.angry_activation = true;
        }
    } else {
        panic!("Increase happiness context should be stored in undo context")
    }
}
