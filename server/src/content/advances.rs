use super::custom_actions::CustomActionType::*;
use crate::{
    ability_initializer::AbilityInitializerSetup,
    advance::{Advance, AdvanceBonus::*},
    resource_pile::ResourcePile,
};

pub fn get_technologies() -> Vec<Advance> {
    vec![
        Advance::builder(
            "Storage",
            "Your maximum food limit is increased from 2 to 7",
        )
        .add_one_time_ability_initializer(|game, player_index| {
            game.players[player_index].resource_limit.food = 7
        })
        .add_ability_deinitializer(|game, player_index| {
            game.players[player_index].resource_limit.food = 2
        })
        .with_advance_bonus(MoodToken)
        .build(),
        Advance::builder(
            "Engineering",
            "ā Immediately draw 1 wonder\nā May Construct wonder happy cities",
        )
        .add_one_time_ability_initializer(|game, player| game.draw_wonder_card(player))
        .add_custom_action(ConstructWonder)
        .build(),
        Advance::builder(
            "Philosophy",
            "ā Immediately gain 1 idea\nā Gain 1 idea after getting a Science advance",
        )
        .add_one_time_ability_initializer(|game, player_index| {
            game.players[player_index].gain_resources(ResourcePile::ideas(1))
        })
        .add_player_event_listener(
            |event| &mut event.on_advance,
            |player, advance, _| {
                if advance == "Math"
                    || advance == "Astronomy"
                    || advance == "Medicine"
                    || advance == "Metallurgy"
                {
                    player.gain_resources(ResourcePile::ideas(2))
                }
            },
            0,
        )
        .with_advance_bonus(MoodToken)
        .build(),
        Advance::builder(
            "Math",
            "Engineering and Roads can be bought at no food cost",
        )
        .add_player_event_listener(
            |event| &mut event.advance_cost,
            |cost, advance, _| {
                if advance == "Engineering" || advance == "Roads" {
                    *cost = 0;
                }
            },
            0,
        )
        .with_advance_bonus(CultureToken)
        .with_unlocked_building("Observatory")
        .build(),
        Advance::builder(
            "Astronomy",
            "Navigation and Cartography can be bought at no food cost",
        )
        .add_player_event_listener(
            |event| &mut event.advance_cost,
            |cost, advance, _| {
                if advance == "Navigation" || advance == "Cartography" {
                    *cost = 0;
                }
            },
            0,
        )
        .with_required_advance("Math")
        .with_advance_bonus(CultureToken)
        .build(),
    ]
}

pub fn get_advance_by_name(name: &str) -> Option<Advance> {
    get_technologies()
        .into_iter()
        .find(|advance| advance.name == name)
}
