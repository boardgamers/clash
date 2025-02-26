use crate::city_pieces::Building;
use crate::game::GameState::Playing;
use crate::game::{Game, GameState};
use crate::payment::PaymentOptions;
use crate::player_events::{ActionInfo, InfluenceCultureInfo, InfluenceCulturePossible};
use crate::playing_actions::{roll_boost_cost, InfluenceCultureAttempt, PlayingAction};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::undo::UndoContext;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CulturalInfluenceResolution {
    pub roll_boost_cost: ResourcePile,
    pub target_player_index: usize,
    pub target_city_position: Position,
    pub city_piece: Building,
}

pub(crate) fn influence_culture_attempt(
    game: &mut Game,
    player_index: usize,
    c: &InfluenceCultureAttempt,
) {
    let starting_city_position = c.starting_city_position;
    let target_player_index = c.target_player_index;
    let target_city_position = c.target_city_position;
    let city_piece = c.city_piece;
    let info = influence_culture_boost_cost(
        game,
        player_index,
        starting_city_position,
        target_player_index,
        target_city_position,
        city_piece,
    );
    if matches!(info.possible, InfluenceCulturePossible::Impossible) {
        panic!("Impossible to influence culture");
    }

    let self_influence = starting_city_position == target_city_position;

    // currectly, there is no way to have different costs for this
    game.players[player_index].lose_resources(info.range_boost_cost.default);
    let roll = game.get_next_dice_roll().value + info.roll_boost;
    let success = roll >= 5;
    if success {
        game.add_to_last_log_item(&format!(" and succeeded (rolled {roll})"));
        info.info.execute(game);
        influence_culture(
            game,
            player_index,
            target_player_index,
            target_city_position,
            city_piece,
        );
        return;
    }

    if self_influence || matches!(info.possible, InfluenceCulturePossible::NoBoost) {
        game.add_to_last_log_item(&format!(" and failed (rolled {roll})"));
        info.info.execute(game);
        return;
    }
    if let Some(roll_boost_cost) = PaymentOptions::resources(roll_boost_cost(roll))
        .first_valid_payment(&game.players[player_index].resources)
    {
        game.add_to_last_log_item(&format!(" and rolled a {roll}"));
        info.info.execute(game);
        game.add_info_log_item(&format!("{} now has the option to pay {roll_boost_cost} to increase the dice roll and proceed with the cultural influence", game.players[player_index].get_name()));
        game.state = GameState::CulturalInfluenceResolution(CulturalInfluenceResolution {
            roll_boost_cost,
            target_player_index,
            target_city_position,
            city_piece,
        });
    } else {
        game.add_to_last_log_item(&format!(
            " but rolled a {roll} and has not enough culture tokens to increase the roll "
        ));
        info.info.execute(game);
    }
}

pub(crate) fn execute_cultural_influence_resolution_action(
    game: &mut Game,
    action: bool,
    roll_boost_cost: ResourcePile,
    target_player_index: usize,
    target_city_position: Position,
    city_piece: Building,
    player_index: usize,
) {
    game.state = Playing;
    if !action {
        return;
    }
    game.players[player_index].lose_resources(roll_boost_cost.clone());
    game.push_undo_context(UndoContext::InfluenceCultureResolution { roll_boost_cost });
    influence_culture(
        game,
        player_index,
        target_player_index,
        target_city_position,
        city_piece,
    );
}

pub(crate) fn undo_cultural_influence_resolution_action(game: &mut Game, action: bool) {
    let cultural_influence_attempt_action = game.action_log[game.action_log_index - 1].action.playing_ref().expect("any log item previous to a cultural influence resolution action log item should a cultural influence attempt action log item");
    let PlayingAction::InfluenceCultureAttempt(c) = cultural_influence_attempt_action else {
        panic!("any log item previous to a cultural influence resolution action log item should a cultural influence attempt action log item");
    };

    let city_piece = c.city_piece;
    let target_player_index = c.target_player_index;
    let target_city_position = c.target_city_position;

    let Some(UndoContext::InfluenceCultureResolution { roll_boost_cost }) = game.pop_undo_context()
    else {
        panic!("when undoing a cultural influence resolution action, the game should have stored influence culture resolution context")
    };

    game.state = GameState::CulturalInfluenceResolution(CulturalInfluenceResolution {
        roll_boost_cost: roll_boost_cost.clone(),
        target_player_index,
        target_city_position,
        city_piece,
    });
    if !action {
        return;
    }
    game.players[game.current_player_index].gain_resources_in_undo(roll_boost_cost);
    undo_influence_culture(game, target_player_index, target_city_position, city_piece);
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
    starting_city_position: Position,
    target_player_index: usize,
    target_city_position: Position,
    city_piece: Building,
) -> InfluenceCultureInfo {
    let starting_city = game.get_city(player_index, starting_city_position);

    let range_boost =
        influence_distance(game, starting_city_position, target_city_position, &[], 0)
            .saturating_sub(starting_city.size() as u32);

    let self_influence = starting_city_position == target_city_position;
    let target_city = game.get_city(target_player_index, target_city_position);
    let target_city_owner = target_city.player_index;
    let target_building_owner = target_city.pieces.building_owner(city_piece);
    let attacker = game.get_player(player_index);
    let defender = game.get_player(target_player_index);
    let start_city_is_eligible = !starting_city.influenced() || self_influence;

    let mut info = InfluenceCultureInfo::new(
        PaymentOptions::resources(ResourcePile::culture_tokens(range_boost)),
        ActionInfo::new(attacker),
    );
    let _ =
        attacker
            .events
            .on_influence_culture_attempt
            .get()
            .trigger(&mut info, target_city, game);
    info.is_defender = true;
    let _ =
        defender
            .events
            .on_influence_culture_attempt
            .get()
            .trigger(&mut info, target_city, game);

    if !matches!(city_piece, Building::Obelisk)
        && starting_city.player_index == player_index
        && info.is_possible(range_boost)
        && attacker.can_afford(&info.range_boost_cost)
        && start_city_is_eligible
        && !game.successful_cultural_influence
        && attacker.is_building_available(city_piece, game)
        && target_city_owner == target_player_index
        && target_building_owner.is_some_and(|o| o != player_index)
    {
        return info;
    }
    info.set_impossible();
    info
}

///
///
/// # Panics
///
/// Panics if the influenced player does not have the influenced city
/// This function assumes the action is legal
pub fn influence_culture(
    game: &mut Game,
    influencer_index: usize,
    influenced_player_index: usize,
    city_position: Position,
    building: Building,
) {
    game.players[influenced_player_index]
        .get_city_mut(city_position)
        .expect("influenced player should have influenced city")
        .pieces
        .set_building(building, influencer_index);
    game.successful_cultural_influence = true;

    game.trigger_command_event(
        influencer_index,
        |e| &mut e.on_influence_culture_success,
        &(),
    );
}

///
///
/// # Panics
///
/// Panics if the influenced player does not have the influenced city
pub fn undo_influence_culture(
    game: &mut Game,
    influenced_player_index: usize,
    city_position: Position,
    building: Building,
) {
    game.players[influenced_player_index]
        .get_city_mut(city_position)
        .expect("influenced player should have influenced city")
        .pieces
        .set_building(building, influenced_player_index);
    game.successful_cultural_influence = false;
}
