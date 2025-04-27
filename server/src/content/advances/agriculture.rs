use crate::ability_initializer::{AbilityInitializerSetup, once_per_turn_advance};
use crate::advance::Bonus::MoodToken;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::collect::{CollectContext, CollectInfo};
use crate::content::advances::{AdvanceGroup, advance_group_builder};
use crate::game::Game;
use crate::map::Terrain::Barren;
use crate::resource_pile::ResourcePile;
use std::collections::HashSet;

pub(crate) fn agriculture() -> AdvanceGroup {
    advance_group_builder(
        "Agriculture",
        vec![farming(), storage(), irrigation(), husbandry()],
    )
}

fn farming() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Farming,
        "Farming",
        "Your cities may Collect food from Grassland and wood from Forest spaces",
    )
}

fn storage() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Storage,
        "Storage",
        "Your maximum food limit is increased from 2 to 7",
    )
    .add_one_time_ability_initializer(|game, player_index| {
        game.players[player_index].resource_limit.food = 7;
    })
    .with_reset_collect_stats()
    .with_advance_bonus(MoodToken)
}

fn irrigation() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Irrigation,
        "Irrigation",
        "Your cities may Collect food from Barren spaces, Ignore Famine events",
    )
    .with_reset_collect_stats()
    .add_transient_event_listener(
        |event| &mut event.terrain_collect_options,
        0,
        |m, (), ()| {
            m.insert(Barren, HashSet::from([ResourcePile::food(1)]));
        },
    )
    .with_advance_bonus(MoodToken)
}

fn husbandry() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Husbandry,
        "Husbandry",
        "During a Collect Resources Action, \
        you may collect from a Land space that is 2 Land spaces away, rather than 1. \
        If you have the Roads Advance you may collect from two Land spaces that are 2 Land \
        spaces away. This Advance can only be used once per turn.",
    )
    .with_advance_bonus(MoodToken)
    .add_transient_event_listener(
        |event| &mut event.collect_options,
        0,
        |i, c, game| {
            once_per_turn_advance(
                Advance::Husbandry,
                i,
                c,
                game,
                |i| &mut i.info.info,
                husbandry_collect,
            );
        },
    )
}

fn husbandry_collect(i: &mut CollectInfo, c: &CollectContext, game: &Game) {
    let player = &game.players[c.player_index];
    let allowed = if player.has_advance(Advance::Roads) {
        2
    } else {
        1
    };
    i.max_per_tile = allowed;
    
    i.info.log.push(format!(
        "Husbandry allows collecting {allowed} resources from 2 land spaces away"
    ));

    game.map
        .tiles
        .iter()
        .filter(|(pos, t)| pos.distance(c.city_position) == 2 && t.is_land())
        .for_each(|(pos, t)| {
            i.choices
                .insert(*pos, c.terrain_options.get(t).cloned().unwrap_or_default());
        });
}
