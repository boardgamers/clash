use super::custom_actions::CustomActionType::*;
use crate::{
    ability_initializer::AbilityInitializerSetup,
    advance::{Advance, AdvanceBonus::*},
    map::Terrain::*,
    resource_pile::ResourcePile,
};

pub fn get_all_advances() -> Vec<Advance> {
    vec![
        //Agriculture

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
        .build(),

        Advance::builder(
            "Irrigation",
            "● Your cities may Collect food from Barren spaces\n● Ignore Famine events",
        )
        .add_collect_option(Barren, ResourcePile::food(1))
        .with_advance_bonus(MoodToken)
        .build(),

        //Construction

        Advance::builder(
            "Engineering",
            "● Immediately draw 1 wonder\n● May Construct wonder happy cities",
        )
        .add_one_time_ability_initializer(|game, player| game.draw_wonder_card(player))
        .add_custom_action(ConstructWonder)
        .build(),

        //Maritime

        Advance::builder("Fishing", "Your cities may Collect food from one Sea space")
            .add_collect_option(Water, ResourcePile::food(1))
            .with_advance_bonus(MoodToken)
            .build(),

        //Education

        Advance::builder(
            "Philosophy",
            "● Immediately gain 1 idea\n● Gain 1 idea after getting a Science advance",
        )
        .add_one_time_ability_initializer(|game, player_index| {
            game.players[player_index].gain_resources(ResourcePile::ideas(1));
        })
        .add_ability_undo_deinitializer(|game, player_index| {
            game.players[player_index].loose_resources(ResourcePile::ideas(1));
        })
        .add_player_event_listener(
            |event| &mut event.on_advance,
            |player, advance, _| {
                if advance == "Math"
                    || advance == "Astronomy"
                    || advance == "Medicine"
                    || advance == "Metallurgy"
                {
                    player.gain_resources(ResourcePile::ideas(2));
                }
            },
            0,
        )
        .add_player_event_listener(
            |event| &mut event.on_undo_advance,
            |player, advance, _| {
                if advance == "Math"
                    || advance == "Astronomy"
                    || advance == "Medicine"
                    || advance == "Metallurgy"
                {
                    player.loose_resources(ResourcePile::ideas(2));
                }
            },
            0,
        )
        .with_advance_bonus(MoodToken)
        .build(),

        //Science

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
    get_all_advances()
        .into_iter()
        .find(|advance| advance.name == name)
}

pub fn get_leading_government_advance(government: &str) -> Option<Advance> {
    get_all_advances().into_iter().find(|advance| {
        advance
            .government
            .as_ref()
            .is_some_and(|value| value.as_str() == government)
    })
}

pub fn get_government_advances(government: &str) -> Vec<Advance> {
    let leading_government =
        get_leading_government_advance(government).expect("government should exist");
    let mut government_advances = get_all_advances()
        .into_iter()
        .filter(|advance| {
            advance
                .required_advance
                .as_ref()
                .is_some_and(|required_advance| required_advance == &leading_government.name)
        })
        .collect::<Vec<Advance>>();
    government_advances.sort_by_key(|advance| {
        advance
            .government_tier
            .expect("advance should be a government advance")
    });
    government_advances.push(leading_government);
    government_advances.reverse();
    government_advances
}
