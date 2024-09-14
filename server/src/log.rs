#![allow(clippy::if_not_else)]

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

#[derive(Clone)]
pub enum ActionLogItem {
    Playing(PlayingAction),
    StatusPhase(StatusPhaseAction),
    Movement(MovementAction),
    CulturalInfluenceResolution(bool),
    Combat(CombatAction),
    PlaceSettler(Position),
}

impl ActionLogItem {
    #[must_use]
    pub fn as_playing_action(&self) -> Option<PlayingAction> {
        if let Self::Playing(v) = self {
            Some(v.clone())
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
    pub fn as_action(self) -> Action {
        match self {
            Self::Playing(action) => Action::Playing(action),
            Self::StatusPhase(action) => Action::StatusPhase(action),
            Self::Movement(action) => Action::Movement(action),
            Self::CulturalInfluenceResolution(action) => {
                Action::CulturalInfluenceResolution(action)
            }
            Self::Combat(action) => Action::Combat(action),
            Self::PlaceSettler(action) => Action::PlaceSettler(action),
        }
    }
}

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
        Action::CulturalInfluenceResolution(action) => format!("{} {}", game.players[game.active_player()].get_name(), match action {
            true => format!("payed {} culture tokens to increased the dice roll and proceed with the cultural influence", game.dice_roll_log.last().expect("there should have been at least one dice roll before a cultural influence resolution action") / 2 + 1),
            false => String::from("declined to increase the dice roll"),
        }),
        Action::Combat(action) => format_combat_action_log_item(action, game),
        Action::PlaceSettler(position) => format!("{} placed a settler in the city at {position}", game.players[game.state.settler_placer().expect("the game should be in the place settler state")].get_name()),
        Action::Undo | Action::Redo => {
            panic!("undoing or redoing actions should not be written to the log")
        }
    }
}

fn format_playing_action_log_item(action: &PlayingAction, game: &Game) -> String {
    let player = &game.players[game.active_player()];
    let player_name = player.get_name();
    "".to_string()
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
        StatusPhaseAction::RaseSize1City(city) => {
            format!(
                "{player_name} {}",
                match city {
                    Some(city) => format!("rased the city at {city} and gained 1 gold"),
                    None => String::from("did not rase a city"),
                }
            )
        }
        StatusPhaseAction::ChangeGovernmentType(new_government) => {
            format!(
                "{player_name} {}",
                match new_government {
                    Some(new_government_advance) => format!(
                        "changed his government from {} to {} - additional advances: {}",
                        game.players[game.active_player()]
                            .government()
                            .expect("player should have a government before changing it"),
                        new_government_advance.new_government,
                        new_government_advance.additional_advances.join(", ")
                    ),
                    None => String::from("did not change his government"),
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
            card.as_ref()
                .map_or(String::from("did not play a tactics card"), |card| format!(
                    "played the {card} tactics card"
                ))
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
