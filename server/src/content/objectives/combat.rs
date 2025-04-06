use crate::ability_initializer::AbilityInitializerSetup;
use crate::log::current_player_turn_log;
use crate::objective_card::{Objective, objective_is_ready};
use itertools::Itertools;

pub(crate) fn conqueror() -> Objective {
    let name = "Conqueror";
    Objective::builder(
        name,
        "You conquered a city with at least 1 Army unit or Fortress this turn.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        1,
        |game, player, _, e| {
            let stats = &e.combat.stats;
            if stats.is_winner(player) && stats.battleground.is_city() {
                objective_is_ready(game.player_mut(player), name);
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
        one of which you've won.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        2,
        |game, player, _, _e| {
            let stat = current_player_turn_log(game)
                .items
                .iter()
                .filter_map(|i| i.combat_stats.as_ref())
                .collect_vec();
            if stat.len() < 2
                || !stat.iter().any(|s| s.is_winner(player))
                // we just check that the combat position is different - not tracking the actual unit IDs
                || stat.iter().all(|s| s.position == stat[0].position)
            {
                return;
            }
            objective_is_ready(game.player_mut(player), name);
        },
    )
    .build()
}

pub(crate) fn general() -> Objective {
    let name = "General";
    Objective::builder(
        name,
        "You killed at least 3 enemy units in a single combat this turn.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        1,
        |game, player, _, e| {
            //todo
            let stats = &e.combat.stats;
            if stats.is_winner(player) && stats.battleground.is_city() {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}
