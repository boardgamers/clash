use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::MoodToken;
use crate::advance::{Advance, AdvanceBuilder};
use crate::collect::{CollectContext, CollectInfo};
use crate::content::advances::{advance_group_builder, AdvanceGroup, IRRIGATION, ROADS};
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
    Advance::builder(
        "Farming",
        "Your cities may Collect food from Grassland and wood from Forest spaces",
    )
}

fn storage() -> AdvanceBuilder {
    Advance::builder(
        "Storage",
        "Your maximum food limit is increased from 2 to 7",
    )
    .add_one_time_ability_initializer(|game, player_index| {
        game.players[player_index].resource_limit.food = 7;
    })
    .add_ability_undo_deinitializer(|game, player_index| {
        game.players[player_index].resource_limit.food = 2;
    })
    .with_advance_bonus(MoodToken)
}

fn irrigation() -> AdvanceBuilder {
    Advance::builder(
        IRRIGATION,
        "Your cities may Collect food from Barren spaces, Ignore Famine events",
    )
    .add_player_event_listener(
        |event| &mut event.terrain_collect_options,
        |m, (), ()| {
            m.insert(Barren, HashSet::from([ResourcePile::food(1)]));
        },
        0,
    )
    .with_advance_bonus(MoodToken)
}

fn husbandry() -> AdvanceBuilder {
    Advance::builder(
        "Husbandry",
        "During a Collect Resources Action, you may collect from a Land space that is 2 Land spaces away, rather than 1. If you have the Roads Advance you may collect from two Land spaces that are 2 Land spaces away. This Advance can only be used once per turn.",
    )
        .with_advance_bonus(MoodToken)
        .add_once_per_turn_listener(
            |event| &mut event.collect_options,
            |i| &mut i.info.info,
            husbandry_collect,
            0,
        )
}

fn husbandry_collect(i: &mut CollectInfo, c: &CollectContext, game: &Game) {
    let player = &game.players[c.player_index];
    let allowed = if player.has_advance(ROADS) { 2 } else { 1 };

    if c.used
        .iter()
        .filter(|(pos, _)| pos.distance(c.city_position) == 2)
        .count()
        == allowed
    {
        return;
    }

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
