use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::card::{HandCard, HandCardLocation};
use crate::content::advances::warfare::draft_cost;
use crate::game::Game;
use crate::log::{ActionLogAction, ActionLogEntry, ActionLogTurn, TurnType};
use crate::map::capital_city_position;
use crate::objective_card::{Objective, objective_is_ready};
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
        "You founded a city this at least 5 spaces away from your starting city position.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.found_city,
        0,
        |game, player, p| {
            if capital_city_position(game, player.get(game)).distance(*p) >= 5 {
                objective_is_ready(player.get_mut(game), name);
            }
        },
    )
    .build()
}

pub(crate) fn terror_regime() -> Objective {
    // is handled explicitly
    Objective::builder("Terror Regime", "At least 4 cities are Angry.").build()
}

pub(crate) fn magnificent_culture() -> Objective {
    let name = "Magnificent Culture";
    Objective::builder(
        name,
        "You just built a wonder OR \
        you built have built the only wonder in the last round.",
    )
    .status_phase_check(|game, player| {
        last_round(game).iter().any(|p| {
            p.actions.iter().any(|a| {
                a.items.iter().any(|i| {
                    matches!(&i.entry, ActionLogEntry::HandCard {
                        card: HandCard::Wonder(_),
                        from: HandCardLocation::Hand(p) ,
                        to: HandCardLocation::PlayToKeep,
                    } if *p == player.index)
                })
            })
        })
    })
    .add_simple_persistent_event_listener(
        |event| &mut event.play_wonder_card,
        0,
        |game, player, _| {
            objective_is_ready(player.get_mut(game), name);
        },
    )
    .build()
}

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
