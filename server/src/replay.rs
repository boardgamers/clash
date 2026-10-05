use crate::action::{Action, try_execute_action};
use crate::consts::NON_HUMAN_PLAYERS;
use crate::game::{CivSetupOption, Game, GameOptions};
use crate::game_api::log_length;
use crate::game_setup::{GameSetupBuilder, setup_game};
use crate::log::{ActionLogAge, TurnType};
use crate::player::PlayerSettings;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ReplayGameData {
    #[serde(default)]
    options: GameOptions,
    seed: String,
    log: Vec<ActionLogAge>,
    log_index: usize,
    players: Vec<ReplayPlayerData>,
    #[serde(default)]
    dropped_players: Vec<usize>,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ReplayPlayerData {
    id: usize,
    #[serde(default)]
    name: Option<String>,
    civilization: String,
    #[serde(default)]
    settings: PlayerSettings,
}

/// Reconstruct a current-format save up to the requested history length.
///
/// # Errors
///
/// Returns an error for invalid history, an unavailable target, or an illegal
/// recorded action. No partial result is returned.
#[must_use]
pub fn replay(data: ReplayGameData, to: Option<usize>) -> Result<Game, String> {
    let turns: Vec<_> = data
        .log
        .iter()
        .flat_map(|age| age.rounds.iter().flat_map(|round| &round.turns))
        .collect();
    let last = turns
        .last()
        .ok_or("Cannot replay an empty action history")?;
    if data.log_index > last.actions.len() {
        return Err("Invalid replay history cursor".to_string());
    }
    let actions: Vec<_> = turns
        .iter()
        .enumerate()
        .flat_map(|(i, turn)| {
            let limit = if i + 1 == turns.len() {
                data.log_index
            } else {
                turn.actions.len()
            };
            turn.actions.iter().take(limit)
        })
        .collect();
    if actions.is_empty() {
        return Err("Cannot replay an empty action history".to_string());
    }
    let to = to.unwrap_or(actions.len());
    if to > actions.len() {
        return Err(format!("Replay target {to} is outside the action history"));
    }
    let player_amount = data
        .players
        .len()
        .checked_sub(NON_HUMAN_PLAYERS)
        .filter(|n| (2..=4).contains(n))
        .ok_or("Invalid replay player count")?;
    if data.players.iter().enumerate().any(|(i, p)| p.id != i) {
        return Err("Invalid replay player indices".to_string());
    }
    let mut builder = GameSetupBuilder::new(player_amount)
        .seed(data.seed)
        .options(data.options.clone());
    if data.options.civilization == CivSetupOption::Random {
        let civilizations = (0..player_amount)
            .map(|player| {
                turns
                    .iter()
                    .find_map(|turn| match &turn.turn_type {
                        TurnType::Setup(setup) if setup.player == player => {
                            setup.civilization.clone()
                        }
                        _ => None,
                    })
                    .ok_or("Replay history is missing a setup civilization")
            })
            .collect::<Result<Vec<_>, _>>()?;
        builder = builder.assigned_civilizations(civilizations);
    }
    let mut game = setup_game(&builder.build());
    if data.options.civilization == CivSetupOption::DraftThree {
        // Draft locks are private and absent from the action log; setup turns
        // record their final choices. Re-deal before locking to preserve the RNG.
        for player in 0..player_amount {
            let civilization = turns
                .iter()
                .find_map(|turn| match &turn.turn_type {
                    TurnType::Setup(setup) if setup.player == player => setup.civilization.clone(),
                    _ => None,
                })
                .ok_or("Replay history is missing a draft civilization")?;
            game = try_execute_action(game, Action::ChooseCivilization(civilization), player)?;
        }
    }
    for player in &data.players {
        if let Some(name) = &player.name {
            game.players[player.id].set_name(name.clone());
        }
    }
    for (i, item) in actions.into_iter().take(to).enumerate() {
        if item.action == Action::StartTurn {
            continue;
        }
        game = try_execute_action(game, item.action.clone(), item.player)
            .map_err(|e| format!("Failed to replay move {}: {e}", i + 1))?;
    }
    if log_length(&game) != to {
        return Err(format!(
            "Replay target {to} is not a complete action boundary"
        ));
    }
    for player in data.players {
        game.players[player.id].settings = player.settings;
    }
    game.dropped_players = data.dropped_players;
    Ok(game)
}
