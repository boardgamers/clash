//! Rank collection cities without choosing resources or spending an action.
use crate::collect::{
    PositionCollection, apply_total_collect, collect_event_origin, possible_resource_collections,
};
use crate::content::civilizations::{aztecs, china, japan};
use crate::game::Game;
use crate::player::CostTrigger;
use crate::playing_actions::PlayingActionType;
use crate::resource_pile::ResourcePile;
use serde_json::{Value, json};
use std::collections::HashMap;

// Keep a representative of each distinct yield, capacity and range use. Rice
// Cultivation is the collection-total effect that also depends on source tiles.
// The actual bonuses are still evaluated by the normal collection engine.
#[derive(Clone, Default, Hash, PartialEq, Eq)]
struct YieldKey {
    used: u8,
    ranged: u8,
    rice: u8,
    pile: ResourcePile,
}

pub(super) fn potentials(game: &Game, seat: usize, kind: &PlayingActionType) -> Value {
    let p = game.player(seat);
    let origin = collect_event_origin(kind, p);
    let cities = p.cities.iter().filter(|city| {
        crate::collect::available_collect_actions_for_city(game, seat, city.position).contains(kind)
    });
    let values = cities
        .map(|city| {
            let info = possible_resource_collections(
                game,
                city.position,
                seat,
                &origin,
                CostTrigger::NoModifiers,
            );
            let mut states =
                HashMap::from([(YieldKey::default(), Vec::<PositionCollection>::new())]);
            for (position, piles) in &info.choices {
                let ranged = u8::from(
                    position.distance(city.position) > 1
                        && !aztecs::tribute_city(game, p, city.position, *position),
                );
                let rice = u8::from(china::rice_cultivation_tile(
                    game,
                    seat,
                    city.position,
                    *position,
                ));
                // Enumerate this tile's choices, including mixed resources when
                // Focused Collection permits collecting repeatedly from one tile.
                let mut tile = HashMap::from([(YieldKey::default(), vec![])]);
                for pile in piles {
                    let previous = tile.clone();
                    for (key, selected) in previous {
                        for times in 1..=info
                            .max_per_tile
                            .min(info.max_selection)
                            .saturating_sub(key.used)
                        {
                            let selection =
                                PositionCollection::new(*position, pile.clone()).times(times);
                            let next = YieldKey {
                                used: key.used + times,
                                ranged: key.ranged + ranged,
                                rice: (key.rice + rice).min(2),
                                pile: key.pile.clone() + selection.total(),
                            };
                            if next.ranged > info.max_range2_tiles {
                                continue;
                            }
                            let mut choices = selected.clone();
                            choices.push(selection);
                            tile.entry(next).or_insert(choices);
                        }
                    }
                }
                let mut next_states = states.clone();
                for (key, selected) in &states {
                    for (extra, choices) in tile.iter().filter(|(key, _)| key.used > 0) {
                        let next = YieldKey {
                            used: key.used + extra.used,
                            ranged: key.ranged + extra.ranged,
                            rice: (key.rice + extra.rice).min(2),
                            pile: key.pile.clone() + extra.pile.clone(),
                        };
                        if next.used > info.max_selection || next.ranged > info.max_range2_tiles {
                            continue;
                        }
                        next_states
                            .entry(next)
                            .or_insert_with(|| selected.iter().chain(choices).cloned().collect());
                    }
                }
                states = next_states;
            }
            let amount = states
                .values()
                .filter_map(|selected| {
                    let collection = apply_total_collect(selected, p, info.clone(), game).ok()?;
                    let total = collection.total.clone()
                        + japan::pottery_bonus(game, p, collection.total.food);
                    Some(total.amount())
                })
                .max()
                .unwrap_or(0);
            json!({"position":city.position,"amount":amount})
        })
        .collect::<Vec<_>>();
    json!(values)
}
