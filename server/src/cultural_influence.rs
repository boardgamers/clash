use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::city::{City, gain_city, lose_city};
use crate::city_pieces::{Building, gain_building};
use crate::consts::INFLUENCE_MIN_ROLL;
use crate::content::ability::Ability;
use crate::content::custom_actions::custom_action_modifier_event_origin;
use crate::content::persistent_events::{PaymentRequest, PersistentEventType, SelectedStructure};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::log::{ActionLogEntry, current_turn_log_without_redo};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::ActionInfo;
use crate::playing_actions::{PlayingAction, PlayingActionType, base_or_modified_available};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use crate::structure::Structure;
use crate::wonder::Wonder;
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct InfluenceCultureAttemptInfo {
    pub target_player: usize,
    pub structure: Structure,
    pub prevent_boost: bool,
    pub range_boost_cost: PaymentOptions,
    pub(crate) info: ActionInfo,
    pub roll_boost: u8,
    pub position: Position,
    pub starting_city_position: Position,
    pub barbarian_takeover: bool,
}

impl InfluenceCultureAttemptInfo {
    #[must_use]
    pub(crate) fn new(
        range_boost_cost: PaymentOptions,
        info: ActionInfo,
        position: Position,
        structure: Structure,
        starting_city_position: Position,
        barbarian_takeover: bool,
        target_player: usize,
    ) -> InfluenceCultureAttemptInfo {
        InfluenceCultureAttemptInfo {
            prevent_boost: false,
            structure,
            range_boost_cost,
            info,
            roll_boost: 0,
            position,
            starting_city_position,
            barbarian_takeover,
            target_player,
        }
    }

    pub fn set_no_boost(&mut self) {
        self.prevent_boost = true;
    }

    #[must_use]
    pub(crate) fn player(&self, player: usize) -> EventPlayer {
        EventPlayer::new(player, self.info.origin.clone())
    }

    #[must_use]
    pub(crate) fn is_defender(&self, player: usize) -> bool {
        self.target_player == player
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct InfluenceCultureBoostInfo {
    pub attempt: InfluenceCultureAttemptInfo,
    pub roll: u8,
    pub roll_boost_cost: PaymentOptions,
}

impl InfluenceCultureBoostInfo {
    #[must_use]
    pub(crate) fn new(attempt: InfluenceCultureAttemptInfo) -> InfluenceCultureBoostInfo {
        InfluenceCultureBoostInfo {
            attempt,
            roll: 0,
            roll_boost_cost: PaymentOptions::free(),
        }
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

pub(crate) fn execute_influence_culture_attempt(
    game: &mut Game,
    player_index: usize,
    i: &InfluenceCultureAttempt,
) -> Result<(), String> {
    let s = &i.selected_structure;
    let info = influence_culture_boost_cost(
        game,
        player_index,
        s,
        &i.action_type,
        false,
        false,
        game.get_any_city(s.position).player_index,
    )?;

    info.player(player_index)
        .add_log_entry(game, ActionLogEntry::InfluenceCultureAttempt(info.clone()));
    on_cultural_influence(game, player_index, InfluenceCultureBoostInfo::new(info));
    Ok(())
}

pub(crate) fn on_cultural_influence(
    game: &mut Game,
    player_index: usize,
    info: InfluenceCultureBoostInfo,
) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.influence_culture_boost,
        info,
        PersistentEventType::InfluenceCultureBoost,
    );
}

pub(crate) fn use_cultural_influence() -> Ability {
    Ability::builder("Influence Culture", "")
        .add_payment_request_listener(
            |e| &mut e.influence_culture_boost,
            2,
            |game, p, info| {
                let cost = &info.attempt.range_boost_cost;
                if cost.is_free() {
                    info.roll_boost_cost = range_boost_cost(game, info, p.index);
                    return None;
                }

                Some(vec![PaymentRequest::mandatory(
                    cost.clone(),
                    &format!("Pay {cost} to increase the range of the influence"),
                )])
            },
            |game, s, info| {
                info.roll_boost_cost = range_boost_cost(game, info, s.player_index);
            },
        )
        .add_payment_request_listener(
            |e| &mut e.influence_culture_boost,
            0,
            roll_boost_payment,
            |game, s, info| roll_boost_paid(game, s.player_index, &s.choice[0], info),
        )
        .build()
}

fn roll_boost_paid(
    game: &mut Game,
    player_index: usize,
    payment: &ResourcePile,
    boost: &mut InfluenceCultureBoostInfo,
) {
    let info = &mut boost.attempt;
    let attempt = current_turn_log_without_redo(game)
        .actions
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

    let player = info.player(player_index);
    if payment.is_empty() {
        player.log(game, "Declined to pay to increase the dice roll");
        attempt_failed(game, player_index, attempt.selected_structure.position);
        return;
    }

    player.log(game, "pays to increase the dice roll");
    influence_culture(game, player_index, info);
}

fn roll_boost_payment(
    game: &mut Game,
    p: &EventPlayer,
    info: &mut InfluenceCultureBoostInfo,
) -> Option<Vec<PaymentRequest>> {
    let cost = &info.roll_boost_cost;
    if cost.is_free() {
        return None;
    }

    let roll = info.roll;
    if !p.get(game).can_afford(cost) {
        p.log(
            game,
            &format!("rolls a {roll} and does not have enough resources to increase the roll"),
        );
        info.attempt.info.execute(game);
        attempt_failed(game, p.index, info.attempt.position);
        return None;
    }

    info.attempt.info.execute(game);
    p.log(
        game,
        &format!(
            "rolls a {roll} and now has the option to pay {cost} to \
            increase the dice roll and proceed with the cultural influence",
        ),
    );

    Some(vec![PaymentRequest::optional(
        cost.clone(),
        &format!("Pay {cost} to increase the dice roll"),
    )])
}

fn range_boost_cost(
    game: &mut Game,
    info: &mut InfluenceCultureBoostInfo,
    player_index: usize,
) -> PaymentOptions {
    let a = &info.attempt;
    let p = a.player(player_index);

    let roll = game.next_dice_roll().value + a.roll_boost;
    info.roll = roll;
    let success = roll >= INFLUENCE_MIN_ROLL;
    if success {
        p.log(
            game,
            &format!("Cultural influence succeeded (rolls {roll})"),
        );
        a.info.execute(game);
        influence_culture(game, player_index, a);
        return PaymentOptions::free();
    }

    if (a.starting_city_position == a.position) || a.prevent_boost {
        p.log(game, &format!("Cultural influence failed (rolls {roll})"));
        a.info.execute(game);
        attempt_failed(game, player_index, a.position);
        return PaymentOptions::free();
    }

    PaymentOptions::resources(
        game.player(player_index),
        p.origin.clone(),
        ResourcePile::culture_tokens(INFLUENCE_MIN_ROLL - roll),
    )
}

fn influence_distance(game: &Game, src: Position, dst: Position) -> u8 {
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
    .map_or(u8::MAX, |(_path, len)| len as u8)
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
    add_action_cost: bool,
    barbarian_takeover: bool,
    target_player: usize,
) -> Result<InfluenceCultureAttemptInfo, String> {
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

    let (start, range_boost) = affordable_start_city(
        game,
        player_index,
        target_city,
        action_type,
        add_action_cost,
    )?;

    let origin = influence_event_origin(action_type, attacker);
    let mut info = Ok(InfluenceCultureAttemptInfo::new(
        PaymentOptions::resources(
            attacker,
            origin.clone(),
            ResourcePile::culture_tokens(range_boost),
        ),
        ActionInfo::new(attacker, origin),
        target_city_position,
        structure.clone(),
        start,
        barbarian_takeover,
        target_player,
    ));
    attacker.trigger_event(
        |e| &e.on_influence_culture_attempt,
        &mut info,
        target_city,
        game,
    );

    info = Ok(info?);

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
) -> Vec<(
    SelectedStructure,
    Result<InfluenceCultureAttemptInfo, String>,
)> {
    game.players
        .iter()
        .flat_map(|p| {
            p.cities
                .iter()
                .flat_map(|city| {
                    structures(city)
                        .into_iter()
                        .map(|s| {
                            let result = influence_culture_boost_cost(
                                game,
                                player,
                                &s,
                                action_type,
                                true,
                                false,
                                city.player_index,
                            );
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

fn influence_culture(game: &mut Game, influencer_index: usize, info: &InfluenceCultureAttemptInfo) {
    let city_position = info.position;
    let city_owner = game.get_any_city(city_position).player_index;
    let new = &info.player(influencer_index);
    match info.structure {
        Structure::CityCenter => {
            let city = lose_city(game, &info.player(city_owner), city_position);
            gain_city(game, new, city);
        }
        Structure::Building(b) => gain_building(game, new, b, city_position),
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

/// Returns the position of the starting city and the cost to boost the influence range.
///
/// # Errors
/// This function returns an error if no starting city is available or
/// if the player can't afford the boost.
///
/// # Panics
/// This function panics in an inconsistent state
pub fn affordable_start_city(
    game: &Game,
    player_index: usize,
    target_city: &City,
    action_type: &PlayingActionType,
    add_action_cost: bool,
) -> Result<(Position, u8), String> {
    if target_city.player_index == player_index {
        Ok((target_city.position, 0))
    } else {
        let player = game.player(player_index);

        let available = &player.resources;
        let mut tokens = available.culture_tokens;
        let mut action_cost = ResourcePile::empty();
        if add_action_cost {
            // either none (action cost and boost cost) or both can use Colosseum
            action_cost = action_type.payment_options(game, player_index).default;
            let c = action_cost.culture_tokens;
            if c > 0 {
                tokens -= c;
            }
        }
        if player.wonders_owned.contains(Wonder::Colosseum) {
            tokens += available.mood_tokens;
            let m = action_cost.mood_tokens;
            if m > 0 {
                tokens -= m;
            }
        }

        let mut start = player
            .cities
            .iter()
            .filter_map(|c| (!c.influenced()).then_some((c.position, c.size())))
            .collect_vec();
        if player.has_special_advance(SpecialAdvance::HellenisticCulture) {
            let extra = game
                .players
                .iter()
                .flat_map(|p| {
                    p.cities.iter().filter_map(|c| {
                        let t = (c.position, c.size());
                        (!c.pieces.buildings(Some(player.index)).is_empty() && !start.contains(&t))
                            .then_some(t)
                    })
                })
                .collect_vec();
            start.extend(extra);
        }
        start
            .iter()
            .filter_map(|&(position, size)| {
                let min_cost = position
                    .distance(target_city.position)
                    .saturating_sub(size as u32) as u8;

                if min_cost > tokens {
                    // avoid unnecessary calculations
                    return None;
                }

                let distance = influence_distance(game, position, target_city.position);
                let boost_cost = distance.saturating_sub(size as u8);
                if boost_cost > tokens {
                    return None;
                }
                Some((position, boost_cost))
            })
            .min_by_key(|(_, boost)| *boost)
            .ok_or("No starting city available".to_string())
    }
}

#[must_use]
pub fn available_influence_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_modified_available(game, player, &PlayingActionType::InfluenceCultureAttempt)
}

pub(crate) fn influence_event_origin(
    action_type: &PlayingActionType,
    player: &Player,
) -> EventOrigin {
    custom_action_modifier_event_origin(influence_base_origin(), action_type, player)
}

pub(crate) fn influence_base_origin() -> EventOrigin {
    EventOrigin::Ability("Influence Culture".to_string())
}
