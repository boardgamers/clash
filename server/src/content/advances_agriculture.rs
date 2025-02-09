use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::advance::Bonus::MoodToken;
use crate::advance::{Advance, AdvanceBuilder};
use crate::collect::CollectContext;
use crate::content::advances::{advance_group_builder, AdvanceGroup, IRRIGATION, ROADS};
use crate::game::Game;
use crate::map::Terrain::Barren;
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use std::collections::{HashMap, HashSet};

pub(crate) fn agriculture() -> AdvanceGroup {
    advance_group_builder(
        "Agriculture",
        vec![
            Advance::builder(
                "Farming",
                "Your cities may Collect food from Grassland and wood from Forest spaces",
            ),
            storage(),
            irrigation(),
            husbandry(),
        ],
    )
}

fn husbandry() -> AdvanceBuilder {
    Advance::builder(
        "Husbandry",
        "During a Collect Resources Action, you may collect from a Land space that is 2 Land spaces away, rather than 1. If you have the Roads Advance you may collect from two Land spaces that are 2 Land spaces away. This Advance can only be used once per turn.",
    )
        .with_advance_bonus(MoodToken)
        .add_player_event_listener(
            |event| &mut event.collect_options,
            husbandry_collect,
            0,
        )
        .add_once_per_turn_effect("Husbandry", is_husbandry_action)
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

fn is_husbandry_action(action: &Action) -> bool {
    match action {
        Action::Playing(PlayingAction::Collect(collect)) => collect
            .collections
            .iter()
            .any(|c| c.0.distance(collect.city_position) > 1),
        _ => false,
    }
}

fn husbandry_collect(
    options: &mut HashMap<Position, HashSet<ResourcePile>>,
    c: &CollectContext,
    game: &Game,
) {
    let player = &game.players[c.player_index];
    let allowed = if player
        .played_once_per_turn_effects
        .contains(&"Husbandry".to_string())
    {
        0
    } else if player.has_advance(ROADS) {
        2
    } else {
        1
    };

    if c.used
        .iter()
        .filter(|(pos, _)| pos.distance(c.city_position) == 2)
        .count()
        == allowed
    {
        return;
    }

    game.map
        .tiles
        .iter()
        .filter(|(pos, t)| pos.distance(c.city_position) == 2 && t.is_land())
        .for_each(|(pos, t)| {
            options.insert(*pos, c.terrain_options.get(t).cloned().unwrap_or_default());
        });
}
