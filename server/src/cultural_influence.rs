use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::city::City;
use crate::city_pieces::Building;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{
    PaymentRequest, PersistentEventType, SelectedStructure, Structure,
};
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::payment::PaymentOptions;
use crate::player_events::ActionInfo;
use crate::playing_actions::{
    base_or_custom_available, roll_boost_cost, PlayingAction, PlayingActionType,
};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use pathfinding::prelude::astar;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct InfluenceCultureAttempt {
    pub selected_structure: SelectedStructure,
    pub action_type: PlayingActionType,
}

impl InfluenceCultureAttempt {
    #[must_use]
    pub fn new(selected_structure: SelectedStructure, action_type: PlayingActionType) -> Self {
        Self {
            selected_structure,
            action_type,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct InfluenceCultureInfo {
    pub is_defender: bool,
    pub structure: Structure,
    pub prevent_boost: bool,
    pub range_boost_cost: PaymentOptions,
    pub(crate) info: ActionInfo,
    pub roll_boost: u8,
    pub position: Position,
    pub starting_city_position: Position,
}

impl InfluenceCultureInfo {
    #[must_use]
    pub(crate) fn new(
        range_boost_cost: PaymentOptions,
        info: ActionInfo,
        position: Position,
        structure: Structure,
        starting_city_position: Position,
    ) -> InfluenceCultureInfo {
        InfluenceCultureInfo {
            prevent_boost: false,
            structure,
            range_boost_cost,
            info,
            roll_boost: 0,
            is_defender: false,
            position,
            starting_city_position,
        }
    }

    pub fn set_no_boost(&mut self) {
        self.prevent_boost = true;
    }
}

#[derive(Clone, PartialEq)]
pub struct InfluenceCultureOutcome {
    pub success: bool,
    pub player: usize,
    pub position: Position,
}

impl InfluenceCultureOutcome {
    #[must_use]
    pub fn new(success: bool, player: usize, position: Position) -> InfluenceCultureOutcome {
        InfluenceCultureOutcome {
            success,
            player,
            position,
        }
    }
}

pub(crate) fn influence_culture_attempt(
    game: &mut Game,
    player_index: usize,
    c: &SelectedStructure,
    action_type: &PlayingActionType,
) {
    let target_city_position = c.position;
    let info = influence_culture_boost_cost(game, player_index, c, action_type)
        .expect("this should be a valid action");
    let self_influence = info.starting_city_position == target_city_position;

    // currently, there is no way to have different costs for this
    game.players[player_index].lose_resources(info.range_boost_cost.default.clone());
    let roll = game.next_dice_roll().value + info.roll_boost;
    let success = roll >= 5;
    if success {
        game.add_to_last_log_item(&format!(" and succeeded (rolled {roll})"));
        info.info.execute(game);
        influence_culture(game, player_index, c);
        return;
    }

    if self_influence || info.prevent_boost {
        game.add_to_last_log_item(&format!(" and failed (rolled {roll})"));
        info.info.execute(game);
        attempt_failed(game, player_index, target_city_position);
        return;
    }
    if let Some(roll_boost_cost) = PaymentOptions::resources(roll_boost_cost(roll))
        .first_valid_payment(&game.players[player_index].resources)
    {
        game.add_to_last_log_item(&format!(" and rolled a {roll}"));
        info.info.execute(game);
        game.add_info_log_item(&format!("{} now has the option to pay {roll_boost_cost} to increase the dice roll and proceed with the cultural influence", game.player_name(player_index)));
        ask_for_cultural_influence_payment(game, player_index, roll_boost_cost);
    } else {
        game.add_to_last_log_item(&format!(
            " but rolled a {roll} and has not enough culture tokens to increase the roll "
        ));
        info.info.execute(game);
        attempt_failed(game, player_index, target_city_position);
    }
}

pub(crate) fn ask_for_cultural_influence_payment(
    game: &mut Game,
    player_index: usize,
    roll_boost_cost: ResourcePile,
) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.influence_culture_resolution,
        roll_boost_cost,
        PersistentEventType::InfluenceCultureResolution,
    );
}

pub(crate) fn cultural_influence_resolution() -> Builtin {
    Builtin::builder(
        "Influence Culture",
        "Pay culture tokens to increase the dice roll",
    )
    .add_payment_request_listener(
        |e| &mut e.influence_culture_resolution,
        0,
        |_game, _player_index, cost| {
            Some(vec![PaymentRequest::new(
                PaymentOptions::resources(cost.clone()),
                &format!("Pay {cost} to increase the dice roll"),
                true,
            )])
        },
        |game, s, _| {
            let a = current_player_turn_log(game)
                .items
                .iter()
                .rev()
                .find_map(|l| {
                    if let Action::Playing(PlayingAction::InfluenceCultureAttempt(a)) = &l.action {
                        Some(a)
                    } else {
                        None
                    }
                })
                .expect(
                    "there should be a cultural influence attempt action log item before \
                    a cultural influence resolution action log item",
                )
                .clone();

            let roll_boost_cost = s.choice[0].clone();
            if roll_boost_cost.is_empty() {
                game.add_info_log_item(&format!(
                    "{} declined to pay to increase the dice roll and failed the \
                        cultural influence",
                    s.player_name
                ));
                attempt_failed(game, s.player_index, a.selected_structure.position);
                return;
            }

            game.add_info_log_item(&format!(
                "{} paid {roll_boost_cost} to increase the dice roll and proceed \
                    with the cultural influence",
                s.player_name
            ));

            influence_culture(game, s.player_index, &a.selected_structure);
        },
    )
    .build()
}

fn influence_distance(game: &Game, src: Position, dst: Position) -> u32 {
    astar(
        &src,
        |p| {
            p.neighbors()
                .into_iter()
                .filter(|p| game.map.is_inside(*p) && !game.map.is_unexplored(*p))
                .map(|n| (n, 1))
        },
        |p| p.distance(dst),
        |&p| p == dst,
    )
    .map_or(u32::MAX, |(_path, len)| len)
}

///
/// # Panics
///
/// This function panics if the selected structure is a wonder.
///
/// # Errors
///
/// This function returns an error if the target can't be influenced.
pub fn influence_culture_boost_cost(
    game: &Game,
    player_index: usize,
    selected: &SelectedStructure,
    action_type: &PlayingActionType,
) -> Result<InfluenceCultureInfo, String> {
    let target_city_position = selected.position;
    let structure = &selected.structure;
    let target_city = game.get_any_city(target_city_position);
    let target_city_owner = target_city.player_index;
    let target_owner = match structure {
        Structure::CityCenter => Some(target_city_owner),
        Structure::Building(b) => target_city.pieces.building_owner(*b),
        Structure::Wonder(_) => panic!("Wonder is not allowed here"),
    };

    if target_owner == Some(player_index) {
        return Err("Target is already owned".to_string());
    }

    if matches!(structure, Structure::Building(Building::Obelisk)) {
        return Err("Obelisk can't be influenced".to_string());
    }

    if game.successful_cultural_influence {
        return Err("Cultural influence already used".to_string());
    }

    let attacker = game.player(player_index);
    if !structure.is_available(attacker, game) {
        return Err("Structure is not available".to_string());
    }

    let target_player_index = target_city.player_index;

    let (start, range_boost) = affordable_start_city(game, player_index, target_city, action_type)?;

    let mut info = Ok(InfluenceCultureInfo::new(
        PaymentOptions::resources(ResourcePile::culture_tokens(range_boost)),
        ActionInfo::new(attacker),
        target_city_position,
        structure.clone(),
        start,
    ));
    attacker.trigger_event(
        |e| &e.on_influence_culture_attempt,
        &mut info,
        target_city,
        game,
    );

    let mut i = info?;
    i.is_defender = true;
    info = Ok(i);

    game.player(target_player_index).trigger_event(
        |e| &e.on_influence_culture_attempt,
        &mut info,
        target_city,
        game,
    );

    let i = info?;
    if i.prevent_boost && range_boost > 0 {
        return Err("Range boost not allowed".to_string());
    }

    Ok(i)
}

#[must_use]
pub fn available_influence_culture(
    game: &Game,
    player: usize,
    action_type: &PlayingActionType,
) -> Vec<(SelectedStructure, Result<InfluenceCultureInfo, String>)> {
    game.players
        .iter()
        .flat_map(|p| {
            p.cities
                .iter()
                .flat_map(|city| {
                    structures(city)
                        .into_iter()
                        .map(|s| {
                            let result =
                                influence_culture_boost_cost(game, player, &s, action_type);
                            (s, result)
                        })
                        .collect_vec()
                })
                .collect_vec()
        })
        .collect_vec()
}

fn structures(city: &City) -> Vec<SelectedStructure> {
    let mut structures: Vec<SelectedStructure> =
        vec![SelectedStructure::new(city.position, Structure::CityCenter)];
    for b in city.pieces.buildings(None) {
        structures.push(SelectedStructure::new(
            city.position,
            Structure::Building(b),
        ));
    }
    structures
}

fn influence_culture(game: &mut Game, influencer_index: usize, structure: &SelectedStructure) {
    let city_position = structure.position;
    let city_owner = game.get_any_city(city_position).player_index;
    match structure.structure {
        Structure::CityCenter => {
            let mut city = game
                .player_mut(city_owner)
                .take_city(city_position)
                .expect("city should be taken");
            city.player_index = influencer_index;
            game.player_mut(influencer_index).cities.push(city);
        }
        Structure::Building(b) => game
            .player_mut(city_owner)
            .get_city_mut(city_position)
            .pieces
            .set_building(b, influencer_index),
        Structure::Wonder(_) => panic!("Wonder is not allowed here"),
    }
    game.successful_cultural_influence = true;

    game.trigger_transient_event_with_game_value(
        influencer_index,
        |e| &mut e.on_influence_culture_resolve,
        &InfluenceCultureOutcome::new(true, influencer_index, city_position),
        &(),
    );
}

fn attempt_failed(game: &mut Game, player: usize, city_position: Position) {
    game.trigger_transient_event_with_game_value(
        player,
        |e| &mut e.on_influence_culture_resolve,
        &InfluenceCultureOutcome::new(false, player, city_position),
        &(),
    );
}

fn affordable_start_city(
    game: &Game,
    player_index: usize,
    target_city: &City,
    action_type: &PlayingActionType,
) -> Result<(Position, u32), String> {
    if target_city.player_index == player_index {
        Ok((target_city.position, 0))
    } else {
        let player = game.player(player_index);
        let available = action_type.remaining_resources(player);

        player
            .cities
            .iter()
            .filter_map(|c| {
                if c.influenced() {
                    return None;
                }

                let min_cost = c
                    .position
                    .distance(target_city.position)
                    .saturating_sub(c.size() as u32);
                if min_cost > available.culture_tokens {
                    // avoid unnecessary calculations
                    return None;
                }

                let boost_cost = influence_distance(game, c.position, target_city.position)
                    .saturating_sub(c.size() as u32);
                if boost_cost > available.culture_tokens {
                    return None;
                }
                Some((c.position, boost_cost))
            })
            .min_by_key(|(_, boost)| *boost)
            .ok_or("No starting city available".to_string())
    }
}

pub(crate) fn format_cultural_influence_attempt_log_item(
    game: &Game,
    player_index: usize,
    player_name: &str,
    i: &InfluenceCultureAttempt,
) -> String {
    let s = &i.selected_structure;
    let target_city_position = s.position;
    let target_city = game.get_any_city(target_city_position);
    let target_player_index = target_city.player_index;
    let info = influence_culture_boost_cost(game, player_index, s, &i.action_type)
        .expect("this should be a valid action");

    let player = if target_player_index == game.active_player() {
        String::from("themselves")
    } else {
        game.player_name(target_player_index)
    };
    let start = info.starting_city_position;
    let city = if start == target_city_position {
        String::new()
    } else {
        format!(" with the city at {start}")
    };
    let range_boost_cost = info.range_boost_cost;
    // this cost can't be changed by the player
    let cost = if range_boost_cost.is_free() {
        String::new()
    } else {
        format!(" and paid {} to boost the range", range_boost_cost.default)
    };
    let city_piece = match s.structure {
        Structure::CityCenter => "City Center",
        Structure::Building(b) => b.name(),
        Structure::Wonder(_) => panic!("Wonder is not allowed here"),
    };
    let suffix = if let PlayingActionType::Custom(_) = i.action_type {
        " using Arts"
    } else {
        ""
    };

    format!(
        "{player_name} tried to influence culture the {city_piece} in the city at {target_city_position} by {player}{city}{cost}{suffix}"
    )
}

#[must_use]
pub fn available_influence_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::InfluenceCultureAttempt,
        &CustomActionType::ArtsInfluenceCultureAttempt,
    )
}
