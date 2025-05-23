use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, gain_advance_without_payment};
use crate::city::MoodState;
use crate::civilization::Civilization;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::PaymentRequest;
use crate::payment::{
    PaymentConversion, PaymentConversionType, PaymentOptions, PaymentReason, base_resources,
};
use crate::player::gain_resources;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};

pub(crate) fn rome() -> Civilization {
    Civilization::new(
        "Rome",
        vec![aqueduct(), roman_roads(), captivi(), provinces()],
        vec![],
    )
}

fn aqueduct() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Aqueduct,
        SpecialAdvanceRequirement::Advance(Advance::Engineering),
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
        SpecialAdvanceRequirement::Advance(Advance::Roads),
        "Roman Roads",
        "Roads distance is increased to 4 if travelling between your cities",
    )
    // is checked explicitly
    .build()
}

fn captivi() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Captivi,
        SpecialAdvanceRequirement::Advance(Advance::Bartering),
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
        2,
        |i, _b, _| {
            i.cost.conversions.push(PaymentConversion::new(
                base_resources(),
                ResourcePile::mood_tokens(1),
                PaymentConversionType::Unlimited,
            ));
            i.info
                .log
                .push("Captivi allows to replace resources with mood tokens".to_string());
        },
    )
    .build()
}

fn provinces() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Provinces,
        SpecialAdvanceRequirement::AnyGovernment,
        "Provinces",
        "You can recruit Cavalry units in any city \
        that is at least 3 spaces away from your capital. \
        Captured cities become Neutral instead of Angry - or Happy if you pay 1 culture token.",
    )
    .add_payment_request_listener(
        |event| &mut event.combat_end,
        21,
        |game, player, s| {
            s.captured_city(player, game)
                .then_some(vec![PaymentRequest::optional(
                    PaymentOptions::resources(
                        game.player(player),
                        PaymentReason::AdvanceAbility,
                        ResourcePile::culture_tokens(1),
                    ),
                    "Pay 1 culture token to make the city happy",
                )])
        },
        |game, c, s| {
            let pile = &c.choice[0];
            if pile.is_empty() {
                game.add_info_log_item("Provinces made the city Neutral instead of Angry");
            } else {
                game.add_info_log_item(&format!(
                    "Provinces made the city Happy instead of Angry for {pile}"
                ));
            };

            game.player_mut(s.attacker.player)
                .get_city_mut(s.defender.position)
                .set_mood_state(if pile.is_empty() {
                    MoodState::Neutral
                } else {
                    MoodState::Happy
                });
        },
    )
    .build()
}
