use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::CultureToken;
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Observatory;
use crate::content::advances::{advance_group_builder, AdvanceGroup, METALLURGY};

pub(crate) fn science() -> AdvanceGroup {
    advance_group_builder(
        "Science",
        vec![
            math(),
            astronomy(),
            // part of metallurgy is not implemented
            Advance::builder(
                METALLURGY,
                "If you have the Steel Weapons Advance, you no longer have to pay 1 ore to activate it against enemies without Steel Weapons.")
                .with_advance_bonus(CultureToken),
        ],
    )
}

fn astronomy() -> AdvanceBuilder {
    Advance::builder(
        "Astronomy",
        "Navigation and Cartography can be bought at no food cost",
    )
    .add_player_event_listener(
        |event| &mut event.advance_cost,
        |i, a, ()| {
            if a.name == "Navigation" || a.name == "Cartography" {
                i.set_zero();
                i.log.push(". Astronomy reduced the cost to 0".to_string());
            }
        },
        0,
    )
    .with_advance_bonus(CultureToken)
}

fn math() -> AdvanceBuilder {
    Advance::builder(
        "Math",
        "Engineering and Roads can be bought at no food cost",
    )
    .add_player_event_listener(
        |event| &mut event.advance_cost,
        |i, a, ()| {
            if a.name == "Engineering" || a.name == "Roads" {
                i.log.push(". Math reduced the cost to 0".to_string());
                i.set_zero();
            }
        },
        0,
    )
    .with_advance_bonus(CultureToken)
    .with_unlocked_building(Observatory)
}
