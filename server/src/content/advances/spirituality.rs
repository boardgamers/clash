use crate::ability_initializer::{AbilityInitializerSetup, once_per_turn_advance};
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building::Temple;
use crate::content::advances::{AdvanceGroup, advance_group_builder, get_group};
use crate::content::persistent_events::ResourceRewardRequest;
use crate::payment::{PaymentConversion, PaymentOptions};
use crate::resource::ResourceType;
use crate::resource::ResourceType::{CultureTokens, MoodTokens};
use crate::resource_pile::ResourcePile;

pub(crate) fn spirituality() -> AdvanceGroup {
    advance_group_builder(
        "Spirituality",
        vec![myths(), rituals(), priesthood(), state_religion()],
    )
}

fn myths() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Myths,
        "Myths",
        "Whenever an Event card asks you have to reduce the mood in a city, \
        you may pay 1 mood token instead of reducing the mood (does not apply for Pirates).",
    )
    .with_advance_bonus(MoodToken)
    .with_unlocked_building(Temple)
    .add_resource_request(
        |event| &mut event.construct,
        1,
        |_game, _player_index, building| {
            if matches!(building, Temple) {
                return Some(ResourceRewardRequest::new(
                    PaymentOptions::sum(1, &[MoodTokens, CultureTokens]),
                    "Select Temple bonus".to_string(),
                ));
            }
            None
        },
        |_game, p, _| {
            vec![format!(
                "{} selected {} as a reward for constructing a Temple",
                p.player_name, p.choice
            )]
        },
    )
}

fn rituals() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Rituals,
        "Rituals",
        "When you perform the Increase Happiness Action \
        you may spend any Resources as a substitute for mood tokens. This is done at a 1:1 ratio",
    )
    .with_advance_bonus(CultureToken)
    .add_transient_event_listener(
        |event| &mut event.happiness_cost,
        0,
        |cost, (), ()| {
            for r in &[
                ResourceType::Food,
                ResourceType::Wood,
                ResourceType::Ore,
                ResourceType::Ideas,
                ResourceType::Gold,
            ] {
                cost.info.log.push(
                    "Rituals allows spending any resource as a substitute for mood tokens"
                        .to_string(),
                );
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
        "Once per turn, a science advance is free",
    )
    .add_transient_event_listener(
        |event| &mut event.advance_cost,
        2,
        |i, &advance, ()| {
            if get_group("Science")
                .advances
                .iter()
                .any(|a| a.advance == advance)
            {
                once_per_turn_advance(
                    Advance::Priesthood,
                    i,
                    &(),
                    &(),
                    |i| &mut i.info.info,
                    |i, (), ()| {
                        i.set_zero();
                        i.info
                            .log
                            .push("Priesthood reduced the cost to 0".to_string());
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
        |event| &mut event.construct_cost,
        0,
        |i, &b, _| {
            if matches!(b, Temple) {
                once_per_turn_advance(
                    Advance::StateReligion,
                    i,
                    &b,
                    &(),
                    |i| &mut i.info.info,
                    |i, _, ()| {
                        i.cost.conversions.push(PaymentConversion::limited(
                            ResourcePile::of(ResourceType::Food, 1),
                            ResourcePile::empty(),
                            1,
                        ));
                        i.info
                            .log
                            .push("State Religion reduced the food cost to 0".to_string());
                    },
                );
            }
        },
    )
}
