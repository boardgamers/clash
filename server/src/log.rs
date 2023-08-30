#![allow(clippy::if_not_else)]

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    action::{Action, CombatAction},
    content::advances,
    game::Game,
    map::Terrain,
    playing_actions::PlayingAction,
    resource_pile::ResourcePile,
    status_phase::{
        ChangeGovernmentType, CompleteObjectives, DetermineFirstPlayer, FreeAdvance, RaseSize1City,
        StatusPhaseAction, StatusPhaseState,
    },
    unit::{MovementAction, Units},
    utils,
};

#[derive(Serialize, Deserialize, Clone)]
pub enum ActionLogItem {
    Playing(String),
    StatusPhase(String),
    Movement(String),
    CulturalInfluenceResolution(String),
}

impl ActionLogItem {
    #[must_use]
    pub fn as_playing_action(&self) -> Option<&str> {
        if let Self::Playing(v) = self {
            Some(v)
        } else {
            None
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if variant does'nt match with it's contents
    #[must_use]
    pub fn as_action(&self) -> Action {
        match self {
            Self::Playing(action) => Action::Playing(serde_json::from_str::<PlayingAction>(action).expect("data should be a serialized playing action")),
            Self::StatusPhase(action) => Action::StatusPhase(serde_json::from_str::<StatusPhaseAction>(action).expect("data should be a serialized status phase action")),
            Self::Movement(action) => Action::Movement(serde_json::from_str::<MovementAction>(action).expect("data should be a serialized movement action")),
            Self::CulturalInfluenceResolution(action) => Action::CulturalInfluenceResolution(serde_json::from_str::<bool>(action).expect("data should be a serialized boolean representing cultural influence resolution confirmation action")),
        }
    }
}

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
        Action::CulturalInfluenceResolution(action) => format!("{} {}", game.players[game.current_player_index].get_name(), match action {
            true => format!("payed {} culture tokens to increased the dice roll and proceed with the cultural influence", game.dice_roll_log.last().expect("there should have been at least one dice roll before a cultural influence resolution action")),
            false => String::from("declined to increase the dice roll"),
        }),
        Action::Combat(action) => format_combat_action_log_item(action, game),
        Action::Undo | Action::Redo => {
            panic!("undoing or redoing actions should not be written to the log")
        }
    }
}

fn format_playing_action_log_item(action: &PlayingAction, game: &Game) -> String {
    let player = &game.players[game.current_player_index];
    let player_name = player.get_name();
    match action {
        PlayingAction::Advance { advance, payment } => format!("{player_name} payed {payment} to get the {advance} advance"),
        PlayingAction::FoundCity { settler } => format!("{player_name} founded a city at {}", player.get_unit(*settler).expect("The player should have the settler").position),
        PlayingAction::Construct { city_position, city_piece, payment, port_position, temple_bonus } => format!("{player_name} payed {payment} to construct a {city_piece:?} in the city at {city_position}{}{}{}", if let Some(port_position) = port_position {
            let adjacent_water_tiles = city_position.neighbors().iter().filter(|neighbor| game.map.tiles.get(neighbor).is_some_and(|terrain| terrain == &Terrain::Water)).count();
            if adjacent_water_tiles > 1 {
                format!(" at the water tile {port_position}")
            } else {
                String::new()
            }
        } else { String::new() }, if player.get_city(*city_position).expect("there should be a city at the given position").is_activated() { format!(" making it {:?}", player.get_city(*city_position).expect("there should be a city at the given position").mood_state.clone() - 1) } else { String::new() }, if let Some(temple_bonus) = temple_bonus {
            format!(" and chooses to get {temple_bonus}")
        } else { String::new() }),
        PlayingAction::Collect { city_position, collections } => format!("{player_name} collects {}{} in the city at {city_position}{}", utils::format_list(&collections.iter().map(|(_, collection)| collection.to_string()).collect::<Vec<String>>(), "nothing"), if collections.len() > 1 && collections.iter().permutations(2).unique().any(|permutation| permutation[0].1.has_common_resource(&permutation[1].1)) { format!(" for a total of {}", collections.iter().map(|(_, collection)| collection.clone()).sum::<ResourcePile>()) } else { String::new() }, if player.get_city(*city_position).expect("there should be a city at the given position").is_activated() { format!(" making it {:?}", player.get_city(*city_position).expect("there should be a city at the given position").mood_state.clone() - 1) } else { String::new() }),
        PlayingAction::Recruit { units, city_position, payment, leader_index, replaced_units } => format!("{player_name} payed {payment} to recruit {}{} in the city at {city_position}{}{}", units.iter().cloned().collect::<Units>(), leader_index.map_or(String::new(), |leader_index| format!(" {} {} as his leader", if player.available_leaders.len() > 1 { "choosing" } else { "getting" }, &player.available_leaders[leader_index].name)), if player.get_city(*city_position).expect("there should be a city at the given position").is_activated() { format!(" making it {:?}", player.get_city(*city_position).expect("there should be a city at the given position").mood_state.clone() - 1) } else { String::new() }, format_args!("{}{}", match replaced_units.len() { 0 => "", 1 => " and replaces the unit at ", _ => " and replaces units at " }, utils::format_list(&replaced_units.iter().map(|unit_id| player.get_unit(*unit_id).expect("the player should have the replaced units").position.to_string()).unique().collect(), ""))),
        PlayingAction::MoveUnits => format!("{player_name} used a move units action"),
        PlayingAction::IncreaseHappiness { happiness_increases } => {
            let happiness_increases = happiness_increases.iter().filter_map(|(position, steps)| if *steps > 0 { Some(format!("the city at {position} by {steps} steps, making it {:?}", player.get_city(*position).expect("player should have a city at this position").mood_state.clone() + *steps)) } else { None }).collect::<Vec<String>>();
            format!("{player_name} increased happiness in {}", utils::format_list(&happiness_increases, "no city"))
        },
        PlayingAction::InfluenceCultureAttempt { starting_city_position, target_player_index, target_city_position, city_piece } => format!("{player_name} tried to influence culture the {city_piece:?} in the city at {target_city_position} by {}{}", if target_player_index == &game.current_player_index { String::from("himself")} else { game.players[*target_player_index].get_name() }, if starting_city_position != target_city_position { format!(" with the city at {starting_city_position}")} else { String::new() }),
        PlayingAction::Custom(action) => action.format_log_item(game, &player_name),
        PlayingAction::EndTurn => format!("{player_name} ended his turn{}", match game.actions_left {
            0 => String::new(),
            actions_left => format!(" with {actions_left} actions left"),
        }),
    }
}

fn format_movement_action_log_item(action: &MovementAction, game: &Game) -> String {
    let player = &game.players[game.current_player_index];
    let player_name = player.get_name();
    match action {
        MovementAction::Move {
            units,
            destination: _,
        } if units.is_empty() => {
            format!("\t{player_name} used a movement actions but moved no units")
        }
        MovementAction::Move { units, destination } => format!(
            "\t{player_name} moved {} from {} to {}",
            units
                .iter()
                .map(|unit| player
                    .get_unit(*unit)
                    .expect("the player should have moved units")
                    .unit_type
                    .clone())
                .collect::<Units>(),
            player
                .get_unit(units[0])
                .expect("the player should have moved units")
                .position,
            destination
        ),
        MovementAction::Stop => format!("\t{player_name} ended the movement action"),
    }
}

fn format_status_phase_action_log_item(action: &StatusPhaseAction, game: &Game) -> String {
    let player_name = game.players[game.current_player_index].get_name();
    match action.phase {
        StatusPhaseState::CompleteObjectives => {
            let completed_objectives = serde_json::from_str::<CompleteObjectives>(&action.data)
                .expect("status phase data should match with it's phase")
                .objectives;
            format!(
                "{player_name} completed {}",
                utils::format_list(&completed_objectives, "no objectives")
            )
        }
        StatusPhaseState::FreeAdvance => {
            let advance = serde_json::from_str::<FreeAdvance>(&action.data)
                .expect("status phase data should match with it's phase")
                .advance;
            format!("{player_name} advanced {advance}")
        }
        StatusPhaseState::RaseSize1City => {
            let city = serde_json::from_str::<RaseSize1City>(&action.data)
                .expect("status phase data should match with it's phase")
                .city;
            format!(
                "{player_name} {}",
                match city {
                    Some(city) => format!("rased the city at {city} and gained 1 gold"),
                    None => String::from("did not rase a city"),
                }
            )
        }
        StatusPhaseState::ChangeGovernmentType => {
            let new_government = serde_json::from_str::<ChangeGovernmentType>(&action.data)
                .expect("status phase data should match with it's phase")
                .new_government;
            format!(
                "{player_name} {}",
                match new_government {
                    Some(new_government_advance) => format!(
                        "changed his government from {} to {}",
                        game.players[game.current_player_index]
                            .government()
                            .expect("player should have a government before changing it"),
                        advances::get_advance_by_name(&new_government_advance)
                            .expect("new government advance should exist")
                            .government
                            .expect("advance should be a government advance")
                    ),
                    None => String::from("did not change his government"),
                }
            )
        }
        StatusPhaseState::DetermineFirstPlayer => {
            let player_index = serde_json::from_str::<DetermineFirstPlayer>(&action.data)
                .expect("status phase data should match with it's phase")
                .player_index;
            format!(
                "{player_name} choose {}",
                if player_index == game.starting_player_index {
                    format!(
                        "{} to remain the staring player",
                        if player_index != game.current_player_index {
                            game.players[player_index].get_name()
                        } else {
                            String::new()
                        }
                    )
                } else {
                    format!(
                        "{} as the new starting player",
                        if player_index == game.current_player_index {
                            String::from("himself")
                        } else {
                            game.players[player_index].get_name()
                        }
                    )
                }
            )
        }
    }
}

fn format_combat_action_log_item(action: &CombatAction, game: &Game) -> String {
    match action {
        CombatAction::PlayActionCard(card) => format!(""),
        CombatAction::Retreat(action) => format!(""),
    }
}
