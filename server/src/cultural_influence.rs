use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::city::City;
use crate::city_pieces::Building;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::{
    CurrentEventType, PaymentRequest, SelectedStructure, Structure,
};
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::payment::PaymentOptions;
use crate::player_events::{ActionInfo, InfluenceCultureInfo, InfluenceCultureOutcome};
use crate::playing_actions::{roll_boost_cost, PlayingAction};
use crate::position::Position;
use crate::resource_pile::ResourcePile;

pub(crate) fn influence_culture_attempt(
    game: &mut Game,
    player_index: usize,
    c: &SelectedStructure,
) {
    let target_city_position = c.0;
    let target_city = game.get_any_city(target_city_position);

    let info = influence_culture_boost_cost(game, player_index, c);
    assert!(info.blockers.is_empty(), "Impossible to influence culture");
    let starting_city_position =
        start_city(game, player_index, target_city).expect("there should be a starting city");

    let self_influence = starting_city_position == target_city_position;

    // currently, there is no way to have different costs for this
    game.players[player_index].lose_resources(info.range_boost_cost.default.clone());
    let roll = game.get_next_dice_roll().value + info.roll_boost;
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
    let _ = game.trigger_current_event(
        &[player_index],
        |e| &mut e.on_influence_culture_resolution,
        roll_boost_cost,
        CurrentEventType::InfluenceCultureResolution,
    );
}

pub(crate) fn cultural_influence_resolution() -> Builtin {
    Builtin::builder(
        "Influence Culture",
        "Pay culture tokens to increase the dice roll",
    )
    .add_payment_request_listener(
        |e| &mut e.on_influence_culture_resolution,
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
                attempt_failed(game, s.player_index, a.0);
                return;
            }

            game.add_info_log_item(&format!(
                "{} paid {roll_boost_cost} to increase the dice roll and proceed \
                    with the cultural influence",
                s.player_name
            ));

            influence_culture(game, s.player_index, &a);
        },
    )
    .build()
}

fn influence_distance(
    game: &Game,
    src: Position,
    dst: Position,
    visited: &[Position],
    len: u32,
) -> u32 {
    if visited.contains(&src) {
        return u32::MAX;
    }
    let mut visited = visited.to_vec();
    visited.push(src);

    if src == dst {
        return len;
    }
    src.neighbors()
        .into_iter()
        .filter(|&p| game.map.is_sea(p) || game.map.is_land(p))
        .map(|n| influence_distance(game, n, dst, &visited, len + 1))
        .min()
        .expect("there should be a path")
}

#[must_use]
pub fn influence_culture_boost_cost(
    game: &Game,
    player_index: usize,
    selected: &SelectedStructure,
) -> InfluenceCultureInfo {
    let target_city_position = selected.0;
    let structure = &selected.1;
    let target_city = game.get_any_city(target_city_position);
    let target_player_index = target_city.player_index;

    let attacker = game.get_player(player_index);

    let Some(starting_city_position) = start_city(game, player_index, target_city) else {
        let mut i = InfluenceCultureInfo::new(
            PaymentOptions::resources(ResourcePile::empty()),
            ActionInfo::new(attacker),
            structure.clone(),
        );
        i.add_blocker("No starting city available");
        return i;
    };
    let starting_city = game.get_any_city(starting_city_position);

    let range_boost =
        influence_distance(game, starting_city_position, target_city_position, &[], 0)
            .saturating_sub(starting_city.size() as u32);

    let self_influence = starting_city_position == target_city_position;
    let target_city_owner = target_city.player_index;
    let target_owner = match structure {
        Structure::CityCenter => Some(target_city_owner),
        Structure::Building(b) => target_city.pieces.building_owner(*b),
        Structure::Wonder(_) => panic!("Wonder is not allowed here"),
    };

    let defender = game.get_player(target_player_index);
    let start_city_is_eligible = !starting_city.influenced() || self_influence;

    let mut info = InfluenceCultureInfo::new(
        PaymentOptions::resources(ResourcePile::culture_tokens(range_boost)),
        ActionInfo::new(attacker),
        structure.clone(),
    );
    let _ = attacker.trigger_event(
        |e| &e.on_influence_culture_attempt,
        &mut info,
        target_city,
        game,
    );
    info.is_defender = true;
    let _ = defender.trigger_event(
        |e| &e.on_influence_culture_attempt,
        &mut info,
        target_city,
        game,
    );

    if matches!(structure, Structure::Building(Building::Obelisk)) {
        info.add_blocker("Obelisk can't be influenced");
    }
    if info.prevent_boost && range_boost > 0 {
        info.add_blocker("Range boost not allowed");
    }
    if !attacker.can_afford(&info.range_boost_cost) {
        info.add_blocker("Not enough culture tokens");
    }

    if !start_city_is_eligible {
        info.add_blocker("Starting city is not eligible");
    }

    if game.successful_cultural_influence {
        info.add_blocker("Cultural influence already used");
    }

    if !structure.is_available(attacker, game) {
        info.add_blocker("Structure is not available");
    }

    if target_owner == Some(player_index) {
        info.add_blocker("Target is already owned");
    }

    info
}

fn influence_culture(game: &mut Game, influencer_index: usize, structure: &SelectedStructure) {
    let city_position = structure.0;
    let city_owner = game.get_any_city(city_position).player_index;
    match structure.1 {
        Structure::CityCenter => {
            let mut city = game
                .get_player_mut(city_owner)
                .take_city(city_position)
                .expect("city should be taken");
            city.player_index = influencer_index;
            game.get_player_mut(influencer_index).cities.push(city);
        }
        Structure::Building(b) => game
            .get_player_mut(city_owner)
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

fn start_city(game: &Game, player_index: usize, target_city: &City) -> Option<Position> {
    if target_city.player_index == player_index {
        Some(target_city.position)
    } else {
        //todo
        // Influence Culture may cross Sea spaces, but not unrevealed
        // Regions.

        // todo ✦ You may target Buildings in Barbarian BARBARIAN cities
        let player = game.get_player(player_index);
        let position = target_city.position;
        player
            .cities
            .iter()
            .filter(|c| !c.influenced())
            .min_by_key(|c| c.position.distance(position))
            .map(|c| c.position)
    }
}

pub(crate) fn format_cultural_influence_attempt_log_item(
    game: &Game,
    player_index: usize,
    player_name: &str,
    c: &SelectedStructure,
) -> String {
    let target_city_position = c.0;
    let target_city = game.get_any_city(target_city_position);
    let target_player_index = target_city.player_index;
    let starting_city_position =
        start_city(game, player_index, target_city).expect("there should be a starting city");

    let player = if target_player_index == game.active_player() {
        String::from("themselves")
    } else {
        game.player_name(target_player_index)
    };
    let city = if starting_city_position == target_city_position {
        String::new()
    } else {
        format!(" with the city at {starting_city_position}")
    };
    let range_boost_cost = influence_culture_boost_cost(game, player_index, c).range_boost_cost;
    // this cost can't be changed by the player
    let cost = if range_boost_cost.is_free() {
        String::new()
    } else {
        format!(" and paid {} to boost the range", range_boost_cost.default)
    };
    let city_piece = match c.1 {
        Structure::CityCenter => "City Center",
        Structure::Building(b) => b.name(),
        Structure::Wonder(_) => panic!("Wonder is not allowed here"),
    };
    format!("{player_name} tried to influence culture the {city_piece:?} in the city at {target_city_position} by {player}{city}{cost}")
}
