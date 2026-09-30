use crate::ability_initializer::{AbilityInitializerSetup, once_per_turn_ability};
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building::Temple;
use crate::content::ability::Ability;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::persistent_events::ResourceRewardRequest;
use crate::game::GameOptions;
use crate::payment::PaymentConversion;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

pub(crate) fn spirituality(options: &GameOptions) -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Spirituality,
        "Spirituality",
        options,
        vec![myths(), rituals(), priesthood(), state_religion()],
    )
}

fn myths() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Myths,
        "Myths",
        "When an event would lower the mood of your cities, you may pay 1 mood token per city to prevent its mood loss, even if the event would make it Angry directly. This does not apply to Pirates.",
    )
    .with_advance_bonus(MoodToken)
    .with_unlocked_building(Temple)
}

pub(crate) fn use_temple() -> Ability {
    Ability::builder("Temple", "")
        .add_resource_request(
            |event| &mut event.construct,
            1,
            |_game, p, building| {
                if building.building == Temple {
                    return Some(ResourceRewardRequest::new(
                        p.reward_options().tokens(1),
                        "Select Temple bonus".to_string(),
                    ));
                }
                None
            },
        )
        .build()
}

fn rituals() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Rituals,
        "Rituals",
        "During an Increase Happiness action, you may replace any number of mood tokens with resources at a rate of 1 resource per mood token.",
    )
    .with_advance_bonus(CultureToken)
    .add_transient_event_listener(
        |event| &mut event.happiness_cost,
        0,
        |cost, (), (), p| {
            for r in &[
                ResourceType::Food,
                ResourceType::Wood,
                ResourceType::Ore,
                ResourceType::Ideas,
                ResourceType::Gold,
            ] {
                cost.info.add_log(p, "Can pay with any resource");
                cost.cost.conversions.push(PaymentConversion::unlimited(
                    ResourcePile::mood_tokens(1),
                    ResourcePile::of(*r, 1),
                ));
            }
        },
    )
}

fn priesthood() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Priesthood,
        "Priesthood",
        "Once per turn, you may research one Science advance without paying resources. The research action cost and prerequisites still apply.",
    )
    .add_transient_event_listener(
        |event| &mut event.advance_cost,
        2,
        |i, &advance, game, p| {
            if game
                .cache
                .get_advance_group(AdvanceGroup::Science)
                .advances
                .iter()
                .any(|a| a.advance == advance)
            {
                once_per_turn_ability(
                    p,
                    i,
                    &(),
                    &(),
                    |i| &mut i.info.info,
                    |i, (), (), p| {
                        i.set_zero_resources(p);
                    },
                );
            }
        },
    )
}

fn state_religion() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::StateReligion,
        "State Religion",
        "Once per turn, when constructing a Temple, do not pay any Food.",
    )
    .with_advance_bonus(MoodToken)
    .add_transient_event_listener(
        |event| &mut event.building_cost,
        0,
        |i, &b, _, p| {
            if matches!(b, Temple) {
                once_per_turn_ability(
                    p,
                    i,
                    &b,
                    &(),
                    |i| &mut i.info.info,
                    |i, _, (), p| {
                        i.cost.conversions.push(PaymentConversion::limited(
                            ResourcePile::of(ResourceType::Food, 1),
                            ResourcePile::empty(),
                            1,
                        ));
                        i.info.add_log(p, "Reduce the food cost to 0");
                    },
                );
            }
        },
    )
}
