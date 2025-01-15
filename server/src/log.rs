#![allow(clippy::if_not_else)]

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::action::PlayActionCard;
use crate::player::Player;
use crate::playing_actions::{
    Collect, Construct, IncreaseHappiness, InfluenceCultureAttempt, Recruit,
};
use crate::status_phase::{ChangeGovernmentType, RazeSize1City};
use crate::{
    action::{Action, CombatAction},
    game::Game,
    map::Terrain,
    playing_actions::PlayingAction,
    position::Position,
    resource_pile::ResourcePile,
    status_phase::StatusPhaseAction,
    unit::{MovementAction, Units},
    utils,
};

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
pub fn format_action_log_item(action: &Action, game: &Game) -> String {
    match action {
        Action::Playing(action) => format_playing_action_log_item(action, game),
        Action::StatusPhase(action) => format_status_phase_action_log_item(action, game),
        Action::Movement(action) => format_movement_action_log_item(action, game),
        Action::CulturalInfluenceResolution(action) => {
            format_cultural_influence_resolution_log_item(game, *action)
        }
        Action::Combat(action) => format_combat_action_log_item(action, game),
        Action::PlaceSettler(position) => format_place_settler_log_item(game, *position),
        Action::ExploreResolution(_rotation) => format_explore_action_log_item(game),
        Action::Undo | Action::Redo => {
            panic!("undoing or redoing actions should not be written to the log")
        }
    }
}

fn format_explore_action_log_item(game: &Game) -> String {
    let player = game.players[game.active_player()].get_name();
    format!("{player} chose the orientation of the newly explored tiles")
}

fn format_place_settler_log_item(game: &Game, position: Position) -> String {
    let player = game.players[game
        .state
        .settler_placer()
        .expect("the game should be in the place settler state")]
    .get_name();
    format!("{player} placed a settler in the city at {position}")
}

fn format_cultural_influence_resolution_log_item(game: &Game, success: bool) -> String {
    let player = game.players[game.active_player()].get_name();
    let outcome = if success {
        let price = game.dice_roll_log.last()
            .expect("there should have been at least one dice roll before a cultural influence resolution action");
        format!("paid {} culture tokens to increase the dice roll and proceed with the cultural influence", price / 2 + 1)
    } else {
        String::from("declined to increase the dice roll")
    };
    format!("{player} {outcome}")
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
            player
                .get_unit(*settler)
                .expect("The player should have the settler")
                .position
        ),
        PlayingAction::Construct(c) => format_construct_log_item(game, player, &player_name, c),
        PlayingAction::Collect(c) => format_collect_log_item(player, &player_name, c),
        PlayingAction::Recruit(r) => format_recruit_log_item(player, &player_name, r),
        PlayingAction::MoveUnits => format!("{player_name} used a move units action"),
        PlayingAction::IncreaseHappiness(i) => format_happiness_increase(player, &player_name, i),
        PlayingAction::InfluenceCultureAttempt(c) => {
            format_cultural_influence_attempt_log_item(game, &player_name, c)
        }
        PlayingAction::Custom(action) => action.format_log_item(game, player, &player_name),
        PlayingAction::EndTurn => format!(
            "{player_name} ended his turn{}",
            match game.actions_left {
                0 => String::new(),
                actions_left => format!(" with {actions_left} actions left"),
            }
        ),
    }
}

fn format_cultural_influence_attempt_log_item(
    game: &Game,
    player_name: &String,
    c: &InfluenceCultureAttempt,
) -> String {
    let target_player_index = c.target_player_index;
    let target_city_position = c.target_city_position;
    let starting_city_position = c.starting_city_position;
    let city_piece = c.city_piece;
    let player = if target_player_index == game.active_player() {
        String::from("himself")
    } else {
        game.players[target_player_index].get_name()
    };
    let city = if starting_city_position != target_city_position {
        format!(" with the city at {starting_city_position}")
    } else {
        String::new()
    };
    format!("{player_name} tried to influence culture the {city_piece:?} in the city at {target_city_position} by {player}{city}")
}

///
/// # Panics
///
/// Panics if the city does not exist
#[must_use]
pub fn format_happiness_increase(
    player: &Player,
    player_name: &str,
    happiness_increases: &IncreaseHappiness,
) -> String {
    let happiness_increases = happiness_increases
        .happiness_increases
        .iter()
        .filter_map(|(position, steps)| {
            if *steps > 0 {
                Some(format!(
                    "the city at {position} by {steps} steps, making it {:?}",
                    player
                        .get_city(*position)
                        .expect("player should have a city at this position")
                        .mood_state
                        .clone()
                        + *steps
                ))
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    format!(
        "{player_name} increased happiness in {}",
        utils::format_list(&happiness_increases, "no city")
    )
}

fn format_recruit_log_item(player: &Player, player_name: &String, r: &Recruit) -> String {
    let leader_index = r.leader_index;
    let city_position = &r.city_position;
    let units = &r.units;
    let payment = &r.payment;
    let replaced_units = &r.replaced_units;
    let units_str = units.iter().cloned().collect::<Units>();
    let leader_str = leader_index.map_or(String::new(), |leader_index| {
        format!(
            " {} {} as his leader",
            if player.available_leaders.len() > 1 {
                "choosing"
            } else {
                "getting"
            },
            &player.available_leaders[leader_index].name
        )
    });
    let mood = format_mood_change(player, *city_position);
    let replace_str = match replaced_units.len() {
        0 => "",
        1 => " and replaces the unit at ",
        _ => " and replaces units at ",
    };
    let replace_pos = utils::format_list(
        &replaced_units
            .iter()
            .map(|unit_id| {
                player
                    .get_unit(*unit_id)
                    .expect("the player should have the replaced units")
                    .position
                    .to_string()
            })
            .unique()
            .collect::<Vec<String>>(),
        "",
    );
    format!(
        "{player_name} paid {payment} to recruit {units_str}{leader_str} in the city at {city_position}{mood}{replace_str}{replace_pos}"
    )
}

pub(crate) fn format_collect_log_item(player: &Player, player_name: &str, c: &Collect) -> String {
    let collections = &c.collections;
    let res = utils::format_list(
        &collections
            .iter()
            .map(|(_, collection)| collection.to_string())
            .collect::<Vec<String>>(),
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
            .filter(|neighbor| {
                game.map
                    .tiles
                    .get(neighbor)
                    .is_some_and(|terrain| terrain == &Terrain::Water)
            })
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
    let temple = if let Some(temple_bonus) = &c.temple_bonus {
        format!(" and chooses to get {temple_bonus}")
    } else {
        String::new()
    };
    format!("{player_name} paid {payment} to construct a {city_piece:?} in the city at {city_position}{port_pos}{mood}{temple}")
}

fn format_mood_change(player: &Player, city_position: Position) -> String {
    if player
        .get_city(city_position)
        .expect("there should be a city at the given position")
        .is_activated()
    {
        format!(
            " making it {:?}",
            player
                .get_city(city_position)
                .expect("there should be a city at the given position")
                .mood_state
                .clone()
                - 1
        )
    } else {
        String::new()
    }
}

fn format_movement_action_log_item(action: &MovementAction, game: &Game) -> String {
    let player = &game.players[game.active_player()];
    let player_name = player.get_name();
    match action {
        MovementAction::Move {
            units,
            destination: _,
        } if units.is_empty() => {
            format!("\t{player_name} used a movement actions but moved no units")
        }
        MovementAction::Move { units, destination } => {
            let units_str = units
                .iter()
                .map(|unit| {
                    player
                        .get_unit(*unit)
                        .expect("the player should have moved units")
                        .unit_type
                        .clone()
                })
                .collect::<Units>();
            let start = player
                .get_unit(units[0])
                .expect("the player should have moved units")
                .position;
            let start_is_water = game
                .map
                .tiles
                .get(&start)
                .expect("the start position should be on the map")
                .is_water();
            let destination_is_water = game
                .map
                .tiles
                .get(destination)
                .expect("the destination position should be on the map")
                .is_water();
            let verb = if start_is_water {
                if destination_is_water {
                    "sailed"
                } else {
                    "disembarked"
                }
            } else if destination_is_water {
                "embarked"
            } else {
                "marched"
            };
            format!("\t{player_name} {verb} {units_str} from {start} to {destination}",)
        }
        MovementAction::Stop => format!("\t{player_name} ended the movement action"),
    }
}

fn format_status_phase_action_log_item(action: &StatusPhaseAction, game: &Game) -> String {
    let player_name = game.players[game.active_player()].get_name();
    match action {
        StatusPhaseAction::CompleteObjectives(completed_objectives) => {
            format!(
                "{player_name} completed {}",
                utils::format_list(completed_objectives, "no objectives")
            )
        }
        StatusPhaseAction::FreeAdvance(advance) => {
            format!("{player_name} advanced {advance} for free")
        }
        StatusPhaseAction::RazeSize1City(city) => {
            format!(
                "{player_name} {}",
                match city {
                    RazeSize1City::Position(city) =>
                        format!("razed the city at {city} and gained 1 gold"),
                    RazeSize1City::None => String::from("did not rase a city"),
                }
            )
        }
        StatusPhaseAction::ChangeGovernmentType(new_government) => {
            format!(
                "{player_name} {}",
                match new_government {
                    ChangeGovernmentType::ChangeGovernment(new_government_advance) => format!(
                        "changed his government from {} to {} - additional advances: {}",
                        game.players[game.active_player()]
                            .government()
                            .expect("player should have a government before changing it"),
                        new_government_advance.new_government,
                        new_government_advance.additional_advances.join(", ")
                    ),
                    ChangeGovernmentType::KeepGovernment =>
                        String::from("did not change his government"),
                }
            )
        }
        StatusPhaseAction::DetermineFirstPlayer(player_index) => {
            format!(
                "{player_name} choose {}",
                if *player_index == game.starting_player_index {
                    format!(
                        "{} to remain the staring player",
                        if *player_index != game.active_player() {
                            game.players[*player_index].get_name()
                        } else {
                            String::new()
                        }
                    )
                } else {
                    format!(
                        "{} as the new starting player",
                        if *player_index == game.active_player() {
                            String::from("himself")
                        } else {
                            game.players[*player_index].get_name()
                        }
                    )
                }
            )
        }
    }
}

fn format_combat_action_log_item(action: &CombatAction, game: &Game) -> String {
    let player = &game.players[game.active_player()];
    let player_name = player.get_name();
    match action {
        CombatAction::PlayActionCard(card) => format!(
            "{player_name} {}",
            match card {
                PlayActionCard::Card(card) => format!("played the {card} tactics card"),
                PlayActionCard::None => String::from("did not play a tactics card"),
            }
        ),
        CombatAction::RemoveCasualties(casualties) => format!(
            "{player_name} removed {}",
            casualties
                .iter()
                .map(|unit| player
                    .get_unit(*unit)
                    .expect("the player should have units to be removed")
                    .unit_type
                    .clone())
                .collect::<Units>()
        ),
        CombatAction::Retreat(action) => format!(
            "{player_name} {}",
            if *action {
                "retreated ending the battle in a draw"
            } else {
                "decided not to retreat"
            }
        ),
    }
}

#[must_use]
pub fn current_turn_log(game: &Game) -> Vec<Action> {
    let from = game.action_log[0..game.action_log_index]
        .iter()
        .rposition(|action| matches!(action, Action::Playing(PlayingAction::EndTurn)))
        .unwrap_or(0);
    game.action_log[from..game.action_log_index]
        .iter()
        .filter(|action| !matches!(action, Action::StatusPhase(_)))
        .cloned()
        .collect()
}
