use crate::ability_initializer::AbilityInitializerSetup;
use crate::content::advances::warfare::draft_cost;
use crate::game::Game;
use crate::log::{ActionLogItem, ActionLogPlayer};
use crate::map::get_map_setup;
use crate::objective_card::{Objective, objective_is_ready};
use crate::player::Player;
use crate::position::Position;
use itertools::Itertools;
use crate::advance::Advance;

pub(crate) fn draft() -> Objective {
    let name = "Draft";
    Objective::builder(name, "You've recruited twice using Draft this turn.")
        .add_simple_persistent_event_listener(
            |event| &mut event.recruit,
            2,
            |game, player, _, r| {
                let p = game.player_mut(player);
                // Draft is just a cost conversion
                let used_draft = r.units.infantry > 0
                    && r.payment.mood_tokens >= draft_cost(p)
                    && p.has_advance(Advance::Draft);
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
        |game, player, _, p| {
            if home_position(game, game.player(player)).distance(*p) >= 5 {
                objective_is_ready(game.player_mut(player), name);
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
        let wonders = last_round(game)
            .iter()
            .filter_map(|p| {
                p.items
                    .iter()
                    .find_map(|i| i.wonder_built.as_ref().map(|n| (n, p.index)))
            })
            .collect_vec();

        wonders.len() == 1 && wonders[0].1 == player.index
    })
    .add_simple_persistent_event_listener(
        |event| &mut event.play_wonder_card,
        0,
        |game, player, _, _| {
            objective_is_ready(game.player_mut(player), name);
        },
    )
    .build()
}

pub(crate) fn last_player_round(game: &Game, player: usize) -> Vec<&ActionLogItem> {
    last_round(game)
        .iter()
        .filter(|p| p.index == player)
        .flat_map(|p| p.items.iter())
        .collect()
}

fn last_round(game: &Game) -> Vec<&ActionLogPlayer> {
    game.action_log
        .last()
        .and_then(|a| a.rounds.last())
        .iter()
        .flat_map(|r| r.players.iter())
        .collect_vec()
}

pub(crate) fn home_position(game: &Game, player: &Player) -> Position {
    let setup = get_map_setup(game.human_players_count());
    let h = &setup.home_positions[player.index];
    h.block.tiles(&h.position, h.position.rotation)[0].0
}
