use crate::ability_initializer::{AbilityInitializerSetup, once_per_turn_advance};
use crate::action_card::gain_action_card_from_pile;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building;
use crate::content::ability::Ability;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::persistent_events::PaymentRequest;
use crate::objective_card::draw_objective_card_from_pile;
use crate::resource::gain_resources;
use crate::resource_pile::ResourcePile;

pub(crate) fn education() -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Education,
        "Education",
        vec![
            writing(),
            public_education(),
            free_education(),
            philosophy(),
        ],
    )
}

fn writing() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Writing,
        "Writing",
        "Gain 1 action and 1 objective card",
    )
    .with_advance_bonus(CultureToken)
    .with_unlocked_building(Building::Academy)
    .add_once_initializer(move |game, player| {
        gain_action_card_from_pile(game, player);
        // can't gain objective card directly, because the "combat_end" listener might
        // currently being processed ("teach us now")
        player.get_mut(game).gained_objective = draw_objective_card_from_pile(game, player);
    })
}

pub(crate) fn use_academy() -> Ability {
    Ability::builder("Academy", "")
        .add_simple_persistent_event_listener(
            |event| &mut event.construct,
            3,
            |game, p, b| {
                if b.building == Building::Academy {
                    p.gain_resources(game, ResourcePile::ideas(2));
                }
            },
        )
        .build()
}

fn public_education() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::PublicEducation,
        "Public Education",
        "Once per turn, when you collect resources in a city with an Academy, gain 1 idea",
    )
    .with_advance_bonus(MoodToken)
    .add_transient_event_listener(
        |event| &mut event.collect_total,
        1,
        |i, game, _, _| {
            let city = game.get_any_city(i.city);
            if city.pieces.academy.is_some() {
                once_per_turn_advance(
                    Advance::PublicEducation,
                    i,
                    &(),
                    &(),
                    |i| &mut i.info.info,
                    |i, (), ()| {
                        i.total += ResourcePile::ideas(1);
                        i.info
                            .log
                            .push("Public Education gained 1 idea".to_string());
                    },
                );
            }
        },
    )
}

fn free_education() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::FreeEducation,
        "Free Education",
        "After you buy an Advance by paying for it with at least 1 gold or 1 idea, \
        you may pay an extra 1 idea to gain 1 mood token",
    )
    .with_advance_bonus(MoodToken)
    .add_payment_request_listener(
        |e| &mut e.advance,
        1,
        |game, p, i| {
            if i.advance == Advance::FreeEducation {
                None
            } else if i.payment.has_at_least(&ResourcePile::gold(1))
                || i.payment.has_at_least(&ResourcePile::ideas(1))
            {
                Some(vec![PaymentRequest::optional(
                    p.payment_options()
                        .resources(p.get(game), ResourcePile::ideas(1)),
                    "Pay extra 1 idea for a mood token",
                )])
            } else {
                None
            }
        },
        |game, s, _| {
            let payment = &s.choice[0];
            if payment.is_empty() {
                return;
            }
            s.player()
                .gain_resources(game, ResourcePile::mood_tokens(1));
        },
    )
}

fn philosophy() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Philosophy,
        "Philosophy",
        "Immediately gain 1 idea after getting a Science advance",
    )
    .add_once_initializer(move |game, player| {
        gain_resources(
            game,
            player.index,
            ResourcePile::ideas(1),
            player.origin.clone(),
        );
    })
    .add_simple_persistent_event_listener(
        |event| &mut event.advance,
        0,
        |game, p, advance| {
            if game
                .cache
                .get_advance_group(AdvanceGroup::Science)
                .advances
                .iter()
                .any(|a| a.advance == advance.advance)
            {
                p.gain_resources(game, ResourcePile::ideas(1));
            }
        },
    )
    .with_advance_bonus(MoodToken)
}
