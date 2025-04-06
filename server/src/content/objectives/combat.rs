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
                .filter_map(|i| i.combat_stats.as_ref().filter(|s| s.battleground.is_land()))
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
        "You killed at least 3 enemy units in a single land combat this turn.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        3,
        |game, player, _, e| {
            let stats = &e.combat.stats;
            let army_units_killed_by_you: u8 = stats
                .opponent(player)
                .losses
                .clone()
                .into_iter()
                .filter_map(|(u, loss)| u.is_army_unit().then_some(loss))
                .sum();
            if stats.battleground.is_land() && army_units_killed_by_you >= 3 {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}

pub(crate) fn great_battle() -> Objective {
    let name = "Great Battle";
    Objective::builder(
        name,
        "You participated in a land battle with at least 6 army units",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        4,
        |game, player, _, e| {
            let stats = &e.combat.stats;
            let b = stats.battleground;
            let army_units_present: u8 =
                stats.attacker.fighters(b).sum() + stats.defender.fighters(b).sum();
            if stats.battleground.is_land() && army_units_present >= 6 {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}
