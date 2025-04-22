use crate::ability_initializer::AbilityInitializerSetup;
use crate::combat_stats::CombatStats;
use crate::content::advances::trade_routes::find_trade_route_for_unit;
use crate::game::Game;
use crate::log::current_player_turn_log;
use crate::objective_card::{Objective, objective_is_ready};
use crate::player::Player;
use crate::unit::{Unit, UnitType};
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
                .fighter_losses(s.battleground)
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
                s.attacker.fighters(b).amount() + s.defender.fighters(b).amount();
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
                s.player(player).fighters(b).amount() < s.opponent(player).fighters(b).amount();
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
        7,
        |game, player, _, e| {
            let s = &e.combat.stats;
            let b = s.battleground;
            let o = s.opponent(player);
            let fewer_fighters =
                s.player(player).fighters(b).amount() < o.fighters(b).amount();
            let fewer_warfare = warfare_advances(game.player(player), game) < warfare_advances(game.player(o.player), game);
            if (fewer_fighters && s.is_winner(player)) || fewer_warfare {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}

fn warfare_advances(player: &Player, game: &Game) -> usize {
    game.cache
        .get_advance_group("Warfare")
        .advances
        .iter()
        .filter(|a| player.has_advance(a.advance))
        .count()
}

pub(crate) fn legendary_battle() -> Objective {
    let name = "Legendary Battle";
    Objective::builder(
        name,
        "You fought a battle against a city of size 5 with at least 1 wonder.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        8,
        |game, player, _, e| {
            let s = &e.combat.stats;
            if let Some(city) = game.try_get_any_city(s.position) {
                let fighters = s.player(player).fighters(s.battleground).amount() >= 3;
                let city_size = city.size() >= 5;
                let wonders = !city.pieces.wonders.is_empty();
                let is_attacker = s.attacker.player == player;
                if fighters && city_size && wonders && is_attacker {
                    objective_is_ready(game.player_mut(player), name);
                }
            }
        },
    )
    .build()
}

pub(crate) fn scavenger() -> Objective {
    let name = "Scavenger";
    Objective::builder(
        name,
        "You killed a settler or ship that could have been used for Trade Routes.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        9,
        |game, player, _, e| {
            let s = &e.combat.stats;
            let o = s.opponent(player);
            let opponent = game.player(o.player);
            let units = &o.losses;
            if units.settlers > 0 || units.ships > 0 {
                // just the position and type matter
                let unit = Unit::new(o.player, s.position, UnitType::Settler, 0);
                if !find_trade_route_for_unit(game, opponent, &unit).is_empty() {
                    objective_is_ready(game.player_mut(player), name);
                }
            }
        },
    )
    .build()
}

pub(crate) fn aggressor() -> Objective {
    let name = "Aggressor";
    Objective::builder(
        name,
        "You won a land battle as attacker in which you killed at least 2 army units.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        10,
        |game, player, _, e| {
            let s = &e.combat.stats;
            if s.is_winner(player)
                && s.battleground.is_land()
                && s.opponent(player).fighter_losses(s.battleground).amount() >= 2
            {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}

pub(crate) fn barbarian_conquest() -> Objective {
    let name = "Barbarian Conquest";
    Objective::builder(
        name,
        "You captured a barbarian city with at least 2 army units.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        11,
        |game, player, _, e| {
            let s = &e.combat.stats;
            if s.is_winner(player)
                && s.battleground.is_city()
                && !s.opponent_is_human(player, game)
                && s.defender.present.amount() >= 2
            {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}

pub(crate) fn resistance() -> Objective {
    let name = "Resistance";
    Objective::builder(
        name,
        "You captured a barbarian city with at least 2 army units.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        12,
        |game, player, _, e| {
            let s = &e.combat.stats;
            if s.is_winner(player)
                && !s.opponent_is_human(player, game)
                && s.battleground.is_city()
                && s.opponent(player).losses.amount() >= 2
            {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}

pub(crate) fn great_commander() -> Objective {
    let name = "Great Commander";
    Objective::builder(
        name,
        "You won a battle with a leader where you did not have more army units than your opponent.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        13,
        |game, player, _, e| {
            let s = &e.combat.stats;
            let b = s.battleground;
            let o = s.opponent(player);
            let not_more_fighters = s.player(player).fighters(b).amount() <= o.fighters(b).amount();

            if s.is_winner(player) && s.player(player).present.leaders > 0 && not_more_fighters {
                objective_is_ready(game.player_mut(player), name);
            }
        },
    )
    .build()
}

pub(crate) fn brutus() -> Objective {
    let name = "Brutus";
    Objective::builder(name, "You killed a leader and 2 army units in a battle.")
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_end,
            14,
            |game, player, _, e| {
                let s = &e.combat.stats;
                let l = &s.opponent(player).losses;

                if l.leaders > 0
                    && l.clone()
                        .to_vec()
                        .iter()
                        .filter(|u| u.is_army_unit() || u.is_ship())
                        .count()
                        >= 3
                {
                    objective_is_ready(game.player_mut(player), name);
                }
            },
        )
        .build()
}
