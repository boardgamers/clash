use serde::{Deserialize, Serialize};

use crate::{
    action::Action,
    content::advances,
    game::Game,
    map::Terrain,
    playing_actions::PlayingAction,
    resource_pile::ResourcePile,
    status_phase::{
        ChangeGovernmentType, CompleteObjectives, DetermineFirstPlayer, FreeAdvance, RaseSize1City,
        StatusPhaseAction,
    },
    utils,
};

#[derive(Serialize, Deserialize, Clone)]
pub enum ActionLogItem {
    Playing(String),
    StatusPhase(String),
    CulturalInfluenceResolution(String),
}

impl ActionLogItem {
    pub fn as_playing_action(&self) -> Option<&str> {
        if let Self::Playing(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_action(&self) -> Action {
        match self {
            Self::Playing(action) => Action::Playing(serde_json::from_str::<PlayingAction>(action).expect("data should be a serialized playing action")),
            Self::StatusPhase(action) => Action::StatusPhase(serde_json::from_str::<StatusPhaseAction>(action).expect("data should be a serialized status phase action")),
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

pub fn format_action_log_item(action: &Action, game: &Game) -> String {
    match action {
        Action::Playing(action) => format_playing_action_log_item(action, game),
        Action::StatusPhase(action) => format_status_phase_action_log_item(action, game),
        Action::CulturalInfluenceResolution(action) => format!("{} {}", game.players[game.current_player_index].get_name(), match action {
            true => format!("payed {} culture tokens to increased the dice roll and proceed with the cultural influence", game.dice_roll_log.last().expect("there should have been at least one dice roll before a cultural influence resolution action")),
            false => String::from("declined to increase the dice roll"),
        }),
        Action::Undo | Action::Redo => {
            panic!("undoing or redoing actions should not be written to the log")
        }
    }
}

fn format_playing_action_log_item(action: &PlayingAction, game: &Game) -> String {
    let player_name = game.players[game.current_player_index].get_name();
    match action {
        PlayingAction::Advance { advance, payment } => format!("{player_name} payed {payment} to get the {advance} advance"),
        PlayingAction::Construct { city_position, city_piece, payment, port_position, temple_bonus } => format!("{player_name} payed {payment} to construct {city_piece:?} in the city at {city_position}{}{}", if let Some(port_position) = port_position {
            let adjacent_water_tiles = city_position.neighbors().iter().filter(|neighbor| game.map.tiles.get(neighbor).is_some_and(|terrain| terrain == &Terrain::Water)).count();
            if adjacent_water_tiles > 1 {
                format!(" at the water tile {port_position}")
            } else {
                String::new()
            }
        } else { String::new() }, if let Some(temple_bonus) = temple_bonus {
            format!(" and chooses to get {temple_bonus}")
        } else { String::new() }),
        PlayingAction::Collect { city_position, collections } => format!("{player_name} collects {}{} in the city at {city_position}", utils::format_list(&collections.iter().map(|(_, collection)| collection.to_string()).collect::<Vec<String>>(), "nothing"), if collections.len() > 1 { format!(" for a total of {}", collections.iter().map(|(_, collection)| collection.clone()).sum::<ResourcePile>()) } else { String::new() }),
        PlayingAction::IncreaseHappiness { happiness_increases } => {
            let happiness_increases = happiness_increases.iter().filter_map(|(position, steps)| if *steps > 0 { Some(format!("the city at {position} by {steps} steps, making it {:?}", game.players[game.current_player_index].get_city(*position).expect("player should have a city at this position").mood_state.clone() + *steps)) } else { None }).collect::<Vec<String>>();
            format!("{player_name} increased happiness in {}", utils::format_list(&happiness_increases, "no city"))
        },
        PlayingAction::InfluenceCultureAttempt { starting_city_position, target_player_index, target_city_position, city_piece } => format!("{player_name} tried to influence culture the {city_piece:?} in the city at {target_city_position} by {}{}", if target_player_index == &game.current_player_index { String::from("himself")} else { game.players[*target_player_index].get_name() }, if starting_city_position != target_city_position { format!(" with the city at {starting_city_position}")} else { String::new() }),
        PlayingAction::Custom(action) => action.format_log_item(game, player_name),
        PlayingAction::EndTurn => format!("{player_name} ended his turn{}", match game.actions_left {
            0 => String::new(),
            actions_left => format!(" with {actions_left} actions left"),
        }),
    }
}

fn format_status_phase_action_log_item(action: &StatusPhaseAction, game: &Game) -> String {
    let player_name = game.players[game.current_player_index].get_name();
    match action.phase {
        crate::status_phase::StatusPhaseState::CompleteObjectives => {
            let completed_objectives = serde_json::from_str::<CompleteObjectives>(&action.data)
                .expect("status phase data should match with it's phase")
                .objectives;
            format!(
                "{player_name} completed {}",
                utils::format_list(&completed_objectives, "no objectives")
            )
        }
        crate::status_phase::StatusPhaseState::FreeAdvance => {
            let advance = serde_json::from_str::<FreeAdvance>(&action.data)
                .expect("status phase data should match with it's phase")
                .advance;
            format!("{player_name} advanced {advance}")
        }
        crate::status_phase::StatusPhaseState::RaseSize1City => {
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
        crate::status_phase::StatusPhaseState::ChangeGovernmentType => {
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
        crate::status_phase::StatusPhaseState::DetermineFirstPlayer => {
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
