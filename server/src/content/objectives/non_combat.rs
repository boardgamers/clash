use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::card::{HandCard, HandCardLocation};
use crate::content::advances::warfare::draft_cost;
use crate::game::Game;
use crate::log::{ActionLogAction, ActionLogEntry, ActionLogTurn, TurnType};
use crate::map::capital_city_position;
use crate::objective_card::{Objective, ObjectiveProgress, objective_is_ready};
use itertools::Itertools;

pub(crate) fn draft() -> Objective {
    let name = "Draft";
    Objective::builder(name, "You've recruited twice using Draft this turn.")
        .add_simple_persistent_event_listener(
            |event| &mut event.recruit,
            2,
            |game, player, r| {
                let p = player.get_mut(game);
                // Draft is just a cost conversion
                let used_draft = r.units.infantry > 0
                    && r.payment.mood_tokens >= draft_cost(p)
                    && p.can_use_advance(Advance::Draft);
                if used_draft {
                    if p.event_info.contains_key("Used Draft") {
                        objective_is_ready(p, name);
                    } else {
                        p.event_info
                            .insert("Used Draft".to_string(), "true".to_string());
                    }
                }
            },
        )
        .build()
}

pub(crate) fn city_founder() -> Objective {
    let name = "City Founder";
    Objective::builder(
        name,
        "Immediately after founding a city at least 5 spaces from your starting city position.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.found_city,
        0,
        |game, player, p| {
            if game
                .map
                .revealed_distance(capital_city_position(game, player.get(game)), *p)
                .is_some_and(|distance| distance >= 5)
            {
                objective_is_ready(player.get_mut(game), name);
            }
        },
    )
    .build()
}

pub(crate) fn terror_regime() -> Objective {
    // is handled explicitly
    Objective::builder("Terror Regime", "At least 4 of your cities are Angry.")
        .progress(|_, player| {
            vec![ObjectiveProgress::new(
                "Angry cities",
                player
                    .cities
                    .iter()
                    .filter(|c| c.mood_state == crate::city::MoodState::Angry)
                    .count(),
                4,
            )]
        })
        .build()
}

pub(crate) fn magnificent_culture() -> Objective {
    let name = "Magnificent Culture";
    Objective::builder(
        name,
        "Immediately after constructing the first Wonder in the game, or during the Status Phase if you were the only player to construct Wonders this Age.",
    )
    .status_phase_check(|game, player| {
        let builders = if wonderful_corrected(game) {
            wonder_builders(current_age_actions(game).into_iter())
        } else {
            wonder_builders(last_round(game).into_iter().flat_map(|turn| &turn.actions))
        };
        !builders.is_empty() && builders.iter().all(|p| *p == player.index)
    })
    .add_simple_persistent_event_listener(
        |event| &mut event.play_wonder_card,
        0,
        |game, player, _| {
            let builders = wonder_builders(crate::log::linear_action_entries(game).into_iter());
            let qualifies = if wonderful_corrected(game) {
                !builders.is_empty() && builders.iter().all(|p| *p == player.index)
            } else {
                builders == [player.index]
            };
            if qualifies {
                objective_is_ready(player.get_mut(game), name);
            }
        },
    )
    .build()
}

fn wonderful_corrected(game: &Game) -> bool {
    game.wonderful_eligibility_from
        .is_some_and(|from| crate::game_api::log_length(game) >= from)
}

fn current_age_actions(game: &Game) -> Vec<&ActionLogAction> {
    let turns: Vec<_> = game
        .log
        .last()
        .into_iter()
        .flat_map(|age| age.rounds.iter().flat_map(|round| &round.turns))
        .collect();
    turns
        .iter()
        .enumerate()
        .flat_map(|(i, turn)| {
            let limit = if i + 1 == turns.len() {
                game.log_index
            } else {
                turn.actions.len()
            };
            turn.actions.iter().take(limit)
        })
        .collect()
}

// Construction history remains valid even after a Wonder is captured or destroyed.
fn wonder_builders<'a>(actions: impl Iterator<Item = &'a ActionLogAction>) -> Vec<usize> {
    actions
        .flat_map(|action| &action.items)
        .filter_map(|item| match &item.entry {
            ActionLogEntry::HandCard {
                card: HandCard::Wonder(_),
                from: HandCardLocation::Hand(player),
                to: HandCardLocation::PlayToKeep,
            } => Some(*player),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "wonder_objective_tests.rs"]
mod wonder_objective_tests;

pub(crate) fn last_player_round(game: &Game, player: usize) -> Vec<&ActionLogAction> {
    last_round(game)
        .iter()
        .filter(|p| {
            if let TurnType::Player(i) = p.turn_type
                && i == player
            {
                true
            } else {
                false
            }
        })
        .flat_map(|p| p.actions.iter())
        .collect()
}

fn last_round(game: &Game) -> Vec<&ActionLogTurn> {
    game.log
        .last()
        .and_then(|a| a.rounds.last())
        .iter()
        .flat_map(|r| r.turns.iter())
        .collect_vec()
}
