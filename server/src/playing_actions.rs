use serde::{Deserialize, Serialize};

use crate::{
    city::Building::{self, *},
    content::custom_actions::CustomAction,
    game::{Game, GameState::*},
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
    Construct {
        city_position: Position,
        city_piece: Building,
        payment: ResourcePile,
        temple_bonus: Option<ResourcePile>,
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
                temple_bonus,
            } => {
                let building = &city_piece;
                let player = &mut game.players[player_index];
                let city = player.get_city(&city_position).expect("Illegal action");
                let cost = player.construct_cost(building, city);
                if !city.can_construct(building, player) || !payment.can_afford(&cost) {
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
                player.construct(building, &city_position);
            }
            IncreaseHappiness {
                happiness_increases,
            } => {
                for (city_position, steps) in happiness_increases {
                    let player = &mut game.players[player_index];
                    let city = player.get_city(&city_position).expect("Illegal action");
                    let cost = ResourcePile::mood_tokens(city.size() as u32) * steps;
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
                //todo! allow cultural influence of barbarians
                let player = &mut game.players[player_index];
                let starting_city = player
                    .get_city(&starting_city_position)
                    .expect("player should have position");
                let range_boost = starting_city_position
                    .distance(&target_city_position)
                    .saturating_sub(starting_city.size() as u32);
                let range_boost_cost = ResourcePile::culture_tokens(range_boost);
                let self_influence = starting_city_position == target_city_position;
                let target_city = game.players[target_player_index]
                    .get_city(&target_city_position)
                    .expect("Illegal action");
                let target_building_owner = target_city
                    .city_pieces
                    .building_owner(&city_piece)
                    .expect("Illegal action");
                let player = &mut game.players[player_index];
                let starting_city = player
                    .get_city(&starting_city_position)
                    .expect("Illegal action");
                if matches!(&city_piece, Obelisk)
                    || starting_city.player_index != player_index
                    || !player.resources().can_afford(&range_boost_cost)
                    || (starting_city.influenced() && self_influence)
                    || game.successful_cultural_influence
                    || !player.available_buildings.can_build(&city_piece)
                    || target_building_owner == player_index
                {
                    panic!("Illegal action");
                }
                player.loose_resources(range_boost_cost);
                let roll = game.get_next_dice_roll();
                let success = roll == 5 || roll == 6;
                if success {
                    game.influence_culture(
                        player_index,
                        target_player_index,
                        &target_city_position,
                        &city_piece,
                    );
                }
                let roll_boost_cost = 5 - roll as u32;
                let can_afford_roll_boost_cost = game.players[player_index]
                    .resources()
                    .can_afford(&ResourcePile::culture_tokens(roll_boost_cost));
                if roll > 6 || self_influence || !can_afford_roll_boost_cost {
                    return;
                }
                game.state = CulturalInfluenceResolution {
                    roll_boost_cost,
                    target_player_index,
                    target_city_position,
                    city_piece,
                };
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
