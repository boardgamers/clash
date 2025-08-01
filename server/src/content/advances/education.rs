use crate::ability_initializer::{AbilityInitializerSetup, once_per_turn_ability};
use crate::action_card::gain_action_card_from_pile;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::card::HandCardLocation;
use crate::city_pieces::Building;
use crate::content::ability::Ability;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::PaymentRequest;
use crate::game::GameOptions;
use crate::log::{self, ActionLogEntry};
use crate::objective_card::draw_objective_card_from_pile;
use crate::playing_actions::PlayingActionType;
use crate::resource::gain_resources;
use crate::resource_pile::ResourcePile;

pub(crate) fn education(options: &GameOptions) -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Education,
        "Education",
        options,
        vec![
            writing(),
            public_education(),
            free_education(),
            philosophy(),
            philosophy_patched(),
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
        if !game.is_update_patch() {
            gain_action_card_from_pile(game, player);
        }
        // can't gain objective card directly, because the "combat_end" listener might
        // currently being processed ("teach us now")
        player.get_mut(game).gained_objective = draw_objective_card_from_pile(game, player);
    })
    .add_transient_event_listener(
        |event| &mut event.after_action,
        1,
        |game, (), (), p| {
            if game.is_update_patch() {
                let count = log::current_log_action_mut(game)
                    .items
                    .iter()
                    .filter(|item| {
                        matches!(
                            item.entry,
                            ActionLogEntry::HandCard {
                                to: HandCardLocation::CompleteObjective(_),
                                ..
                            }
                        )
                    })
                    .count();
                p.gain_resources(game, ResourcePile::ideas(count as u8));
            }
        },
    )
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
        |i, game, _, p| {
            let city = game.get_any_city(i.city);
            if city.pieces.academy.is_some() {
                once_per_turn_ability(
                    p,
                    i,
                    &(),
                    &(),
                    |i| &mut i.info.info,
                    |i, (), (), p| {
                        i.total += ResourcePile::ideas(1);
                        i.info.add_log(p, "Gain 1 idea");
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
        "Immediately gain 1 idea. Gain 1 idea after getting a Science advance",
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

fn philosophy_patched() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Philosophy,
        "Philosophy",
        "As a free action, you may pay 3 ideas to get a free advance action",
    )
    .replaces(Advance::Philosophy)
    .add_action_modifier(
        CustomActionType::Philosophy,
        |cost| {
            cost.any_times()
                .free_action()
                .resources(ResourcePile::ideas(3))
        },
        PlayingActionType::Advance,
    )
    .with_advance_bonus(MoodToken)
}
