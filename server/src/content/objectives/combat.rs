use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{CivilCardMatch, CivilCardOpportunity};
use crate::content::advances::warfare::draft_cost;
use crate::objective_card::{Objective, objective_is_ready};

pub(crate) fn combat_objectives() -> Vec<Objective> {
    vec![conqueror(), warmonger()]
}

pub(crate) fn conqueror() -> Objective {
    let name = "Conqueror";
    Objective::builder(
        name,
        "You conquered a city with at least 1 Army unit or Fortress this turn.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_round_end,
        2,
        |game, player, _, e| {
            let c = &e.combat;
            if let Some(r) = &e.final_result {
                if let Some(winner) = r.winner() {
                    let p = c.player(winner);
                    if p == player && c.defender_city(game).is_some() {
                        objective_is_ready(game.player_mut(player), name);
                    }
                }
            }
        },
    )
    .build()
}

pub(crate) fn warmonger() -> Objective {
    let name = "Warmonger";
    Objective::builder(
        name,
        "You've completed your second combat this turn against different armies or cities, \
        once of which you've won.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_round_end,
        2,
        |game, player, _, e| {
            let c = &e.combat;
            if let Some(r) = &e.final_result {
                if let Some(winner) = r.winner() {
                    let p = c.player(winner);
                    if p == player && c.defender_city(game).is_some() {
                        objective_is_ready(game.player_mut(player), name);
                    }
                }
            }
        },
    )
    .build()
}
