use crate::ability_initializer::AbilityInitializerSetup;
use crate::combat_stats::CombatStats;
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::objective_card::{objective_is_ready, Objective};
use itertools::Itertools;
use crate::content::advances;
use crate::player::Player;

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
            let s = &e.combat.stats;
            if s.is_winner(player) && s.battleground.is_city() && s.opponent_is_human(player, game)
            {
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
            let s = &e.combat.stats;
            let army_units_killed_by_you: u8 = s
                .opponent(player)
                .losses
                .clone()
                .into_iter()
                .filter_map(|(u, loss)| u.is_army_unit().then_some(loss))
                .sum();
            if s.battleground.is_land() && army_units_killed_by_you >= 3 {
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
            let s = &e.combat.stats;
            let b = s.battleground;
            let army_units_present: u8 =
                s.attacker.fighters(b).sum() + s.defender.fighters(b).sum();
            if s.battleground.is_land() && army_units_present >= 6 {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}

pub(crate) fn defiance() -> Objective {
    let name = "Defiance";
    Objective::builder(
        name,
        "You won a battle against a human player despite having fewer units than them.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        5,
        |game, player, _, e| {
            let s = &e.combat.stats;
            let b = s.battleground;
            let fewer_fighters =
                s.player(player).fighters(b).sum() < s.opponent(player).fighters(b).sum();
            if fewer_fighters && s.opponent_is_human(player, game) && s.is_winner(player) {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}

pub(crate) fn naval_assault() -> Objective {
    let name = "Naval Assault";
    Objective::builder(
        name,
        "You captured a city with army units that disembarked from a ship.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.capture_undefended_position,
        0,
        |game, p, _, s| eval_naval_assault(game, p, s),
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        6,
        |game, p, _, e| {
            eval_naval_assault(game, p, &e.combat.stats);
        },
    )
    .build()
}

fn eval_naval_assault(game: &mut Game, player: usize, s: &CombatStats) {
    if s.disembarked
        && s.opponent_is_human(player, game)
        && s.is_winner(player)
        && s.battleground.is_city()
    {
        objective_is_ready(game.player_mut(player), "Naval Assault");
    }
}

pub(crate) fn bold() -> Objective {
    let name = "Bold";
    Objective::builder(
        name,
        "You won a battle against a player that has more Warfare advances - or despite having fewer units than them.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        5,
        |game, player, _, e| {
            let s = &e.combat.stats;
            let b = s.battleground;
            let o = s.opponent(player);
            let fewer_fighters =
                s.player(player).fighters(b).sum() < o.fighters(b).sum();
            let fewer_warfare = warfare_advances(game.player(player)) < warfare_advances(game.player(o.player));
            if (fewer_fighters && s.is_winner(player)) || fewer_warfare {
                objective_is_ready(game.player_mut(player), name);
            } 
        },
    )
    .build()
}

fn warfare_advances(player: &Player) -> usize {
    advances::get_group("Warfare")
        .advances
        .iter()
        .filter(|a| player.has_advance(&a.name))
        .count()
}