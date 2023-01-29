use super::custom_actions::CustomActionType::*;
use crate::{
    ability_initializer::AbilityInitializerSetup,
    advance::{Advance, AdvanceBonus::*},
};

pub fn get_technologies() -> Vec<Advance> {
    vec![
        Advance::builder(
            "Engineering",
            "● Immediately draw 1 wonder\n● May Construct wonder happy cities",
        )
        .add_ability_initializer(Box::new(|game, player| game.draw_wonder_card(player)))
        .add_custom_action(ConstructWonder)
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
    ]
}

pub fn get_advance_by_name(name: &str) -> Option<Advance> {
    get_technologies()
        .into_iter()
        .find(|advance| advance.name == name)
}
