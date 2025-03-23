#![allow(clippy::if_not_else)]

use crate::content::action_cards::get_civil_card;
use crate::cultural_influence::influence_culture_boost_cost;
use crate::game::ActionLogItem;
use crate::player::Player;
use crate::playing_actions::{
    Collect, Construct, IncreaseHappiness, InfluenceCultureAttempt, Recruit,
};
use crate::{
    action::Action,
    game::Game,
    playing_actions::PlayingAction,
    position::Position,
    resource_pile::ResourcePile,
    unit::{MovementAction, Units},
    utils,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LogSliceOptions {
    pub player: Option<usize>,
    pub start: usize,
    pub end: Option<usize>,
}

///
///
/// # Panics
///
/// Panics if an undo or redo action is given
///
/// this is called before the action is executed
#[must_use]
pub fn format_action_log_item(action: &Action, game: &Game) -> Vec<String> {
    match action {
        Action::Playing(action) => vec![format_playing_action_log_item(action, game)],
        Action::Movement(action) => vec![format_movement_action_log_item(action, game)],
        Action::Response(_) => {
            // is done in the event handler itself
            vec![]
        }
        Action::Undo | Action::Redo => {
            panic!("undoing or redoing actions should not be written to the log")
        }
    }
}

fn format_playing_action_log_item(action: &PlayingAction, game: &Game) -> String {
    let player = &game.players[game.active_player()];
    let player_name = player.get_name();
    match action {
        PlayingAction::Advance { advance, payment } => {
            format!("{player_name} paid {payment} to get the {advance} advance")
        }
        PlayingAction::FoundCity { settler } => format!(
            "{player_name} founded a city at {}",
            player.get_unit(*settler).position
        ),
        PlayingAction::Construct(c) => format_construct_log_item(game, player, &player_name, c),
        PlayingAction::Collect(c) => format_collect_log_item(player, &player_name, c),
        PlayingAction::Recruit(r) => format_recruit_log_item(player, &player_name, r),
        PlayingAction::IncreaseHappiness(i) => format_happiness_increase(player, &player_name, i),
        PlayingAction::InfluenceCultureAttempt(c) => {
            format_cultural_influence_attempt_log_item(game, player.index, &player_name, c)
        }
        PlayingAction::Custom(action) => action.format_log_item(game, player, &player_name),
        PlayingAction::ActionCard(a) => format!(
            "{player_name} played the action card {}",
            get_civil_card(*a).name
        ),
        PlayingAction::EndTurn => format!(
            "{player_name} ended their turn{}",
            match game.actions_left {
                0 => String::new(),
                actions_left => format!(" with {actions_left} actions left"),
            }
        ),
    }
}

pub(crate) fn format_cultural_influence_attempt_log_item(
    game: &Game,
    player_index: usize,
    player_name: &str,
    c: &InfluenceCultureAttempt,
) -> String {
    let target_player_index = c.target_player_index;
    let target_city_position = c.target_city_position;
    let starting_city_position = c.starting_city_position;
    let city_piece = c.city_piece;
    let player = if target_player_index == game.active_player() {
        String::from("themselves")
    } else {
        game.player_name(target_player_index)
    };
    let city = if starting_city_position != target_city_position {
        format!(" with the city at {starting_city_position}")
    } else {
        String::new()
    };
    let range_boost_cost = influence_culture_boost_cost(
        game,
        player_index,
        starting_city_position,
        target_player_index,
        target_city_position,
        city_piece,
    )
    .range_boost_cost;
    // this cost can't be changed by the player
    let cost = if !range_boost_cost.is_free() {
        format!(" and paid {} to boost the range", range_boost_cost.default)
    } else {
        String::new()
    };
    format!("{player_name} tried to influence culture the {city_piece:?} in the city at {target_city_position} by {player}{city}{cost}")
}

///
/// # Panics
///
/// Panics if the city does not exist
#[must_use]
pub fn format_happiness_increase(
    player: &Player,
    player_name: &str,
    i: &IncreaseHappiness,
) -> String {
    let happiness_increases = i
        .happiness_increases
        .iter()
        .filter_map(|(position, steps)| {
            if *steps > 0 {
                Some(format_city_happiness_increase(player, *position, *steps))
            } else {
                None
            }
        })
        .collect_vec();
    format!(
        "{player_name} paid {} to increase happiness in {}",
        i.payment,
        utils::format_and(&happiness_increases, "no city")
    )
}

pub(crate) fn format_city_happiness_increase(
    player: &Player,
    position: Position,
    steps: u32,
) -> String {
    format!(
        "the city at {position} by {steps} steps, making it {:?}",
        player.get_city(position).mood_state.clone() + steps
    )
}

fn format_recruit_log_item(player: &Player, player_name: &String, r: &Recruit) -> String {
    let leader_name = r.leader_name.clone();
    let city_position = &r.city_position;
    let units = &r.units;
    let payment = &r.payment;
    let replaced_units = &r.replaced_units;
    let leader_str = leader_name.map_or(String::new(), |leader_name| {
        format!(
            " {} {} as their leader",
            if player.available_leaders.len() > 1 {
                "choosing"
            } else {
                "getting"
            },
            &leader_name
        )
    });
    let mood = format_mood_change(player, *city_position);
    let replace_str = match replaced_units.len() {
        0 => "",
        1 => " and replaces the unit at ",
        _ => " and replaces units at ",
    };
    let replace_pos = utils::format_and(
        &replaced_units
            .iter()
            .map(|unit_id| player.get_unit(*unit_id).position.to_string())
            .unique()
            .collect_vec(),
        "",
    );
    format!(
        "{player_name} paid {payment} to recruit {units}{leader_str} in the city at {city_position}{mood}{replace_str}{replace_pos}"
    )
}

pub(crate) fn format_collect_log_item(player: &Player, player_name: &str, c: &Collect) -> String {
    let collections = &c.collections;
    let res = utils::format_and(
        &collections
            .iter()
            .map(|(_, collection)| collection.to_string())
            .collect_vec(),
        "nothing",
    );
    let total = if collections.len() > 1
        && collections
            .iter()
            .permutations(2)
            .unique()
            .any(|permutation| permutation[0].1.has_common_resource(&permutation[1].1))
    {
        format!(
            " for a total of {}",
            collections
                .iter()
                .map(|(_, collection)| collection.clone())
                .sum::<ResourcePile>()
        )
    } else {
        String::new()
    };
    let city_position = c.city_position;
    let mood = format_mood_change(player, city_position);
    format!("{player_name} collects {res}{total} in the city at {city_position}{mood}")
}

fn format_construct_log_item(
    game: &Game,
    player: &Player,
    player_name: &String,
    c: &Construct,
) -> String {
    let port_pos = if let Some(port_position) = c.port_position {
        let adjacent_water_tiles = c
            .city_position
            .neighbors()
            .iter()
            .filter(|neighbor| game.map.is_sea(**neighbor))
            .count();
        if adjacent_water_tiles > 1 {
            format!(" at the water tile {port_position}")
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let city_piece = c.city_piece;
    let payment = &c.payment;
    let city_position = c.city_position;

    let mood = format_mood_change(player, city_position);
    format!("{player_name} paid {payment} to construct a {city_piece:?} in the city at {city_position}{port_pos}{mood}")
}

fn format_mood_change(player: &Player, city_position: Position) -> String {
    if player.get_city(city_position).is_activated() {
        format!(
            " making it {:?}",
            player.get_city(city_position).mood_state.clone() - 1
        )
    } else {
        String::new()
    }
}

fn format_movement_action_log_item(action: &MovementAction, game: &Game) -> String {
    let player = &game.players[game.active_player()];
    let player_name = player.get_name();
    match action {
        MovementAction::Move(m) if m.units.is_empty() => {
            format!("{player_name} used a movement actions but moved no units")
        }
        MovementAction::Move(m) => {
            let units_str = m
                .units
                .iter()
                .map(|unit| player.get_unit(*unit).unit_type)
                .collect::<Units>();
            let start = player.get_unit(m.units[0]).position;
            let start_is_water = game.map.is_sea(start);
            let dest = m.destination;
            let t = game
                .map
                .get(dest)
                .expect("the destination position should be on the map");
            let (verb, suffix) = if start_is_water {
                if t.is_unexplored() || t.is_water() {
                    ("sailed", "")
                } else {
                    ("disembarked", "")
                }
            } else if t.is_water() {
                ("embarked", "")
            } else if start.is_neighbor(dest) {
                ("marched", "")
            } else {
                ("marched", " on roads")
            };
            let payment = &m.payment;
            let cost = if payment.is_empty() {
                String::new()
            } else {
                format!(" for {payment}")
            };
            format!("{player_name} {verb} {units_str} from {start} to {dest}{suffix}{cost}",)
        }
        MovementAction::Stop => format!("{player_name} ended the movement action"),
    }
}

#[must_use]
pub fn current_turn_log(game: &Game) -> Vec<ActionLogItem> {
    let from = game.action_log[0..game.action_log_index]
        .iter()
        .rposition(|item| matches!(item.action, Action::Playing(PlayingAction::EndTurn)))
        .unwrap_or(0);
    game.action_log[from..game.action_log_index].to_vec()
}
