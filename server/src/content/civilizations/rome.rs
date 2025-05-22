use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, gain_advance_without_payment};
use crate::civilization::Civilization;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::player::gain_resources;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo};

pub(crate) fn rome() -> Civilization {
    Civilization::new("Rome", vec![aqueduct(), roman_roads(), captivi()], vec![])
}

fn aqueduct() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Aqueduct,
        Advance::Engineering,
        "Aqueduct",
        "Ignore Famine events. \
                Sanitation cost is reduced to 0 resources or a free action",
    )
    .add_custom_action(CustomActionType::Aqueduct)
    .add_transient_event_listener(
        |event| &mut event.advance_cost,
        3,
        |i, &a, _| {
            if a == Advance::Sanitation {
                i.set_zero();
                i.info
                    .log
                    .push("Aqueduct reduced the cost to 0".to_string());
            }
        },
    )
    .build()
}

pub(crate) fn use_aqueduct() -> Builtin {
    Builtin::builder("Aqueduct", "Gain Sanitation as a free action")
        .add_simple_persistent_event_listener(
            |event| &mut event.custom_action,
            0,
            |game, player, name, a| {
                game.add_info_log_item(&format!(
                    "{name} uses Aqueduct to gain Sanitation as a free action",
                ));
                gain_advance_without_payment(
                    game,
                    Advance::Sanitation,
                    player,
                    a.payment.clone(),
                    true,
                );
            },
        )
        .build()
}

fn roman_roads() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::RomanRoads,
        Advance::Roads,
        "Roman Roads",
        "Roads distance is increased to 4 if travelling between your cities",
    )
    // is checked explicitly
    .build()
}

fn captivi() -> SpecialAdvanceInfo {
    // todo You may replace any resources with mood tokens when paying for buildings
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Captivi,
        Advance::Bartering,
        "Captivi",
        "Gain 1 gold and 1 mood token when you win a battle. \
        You may replace any resources with mood tokens when paying for buildings.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        20,
        |game, player, _, s| {
            if s.is_winner(player) && s.is_battle() {
                gain_resources(
                    game,
                    player,
                    ResourcePile::gold(1) + ResourcePile::mood_tokens(1),
                    |name, pile| format!("{name} gained {pile} for Captivi"),
                );
            }
        },
    )
    .add_transient_event_listener(
        |event| &mut event.building_cost,
        0,
        |i, &b, _| {
            i.cost.conversions.push(PaymentConversion::limited(
                ResourcePile::of(ResourceType::Food, 1),
                ResourcePile::empty(),
                1,
            ));
            i.info
                .log
                .push("State Religion reduced the food cost to 0".to_string());
        },
    )
    .build()
}
