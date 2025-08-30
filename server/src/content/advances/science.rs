use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::advance::Bonus::CultureToken;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building;
use crate::content::ability::Ability;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::persistent_events::ResourceRewardRequest;
use crate::game::GameOptions;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

pub(crate) fn science(options: &GameOptions) -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Science,
        "Science",
        options,
        vec![math(), astronomy(), medicine(), metallurgy()],
    )
}

fn math() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Math,
        "Math",
        "Engineering and Roads can be bought at no food cost",
    )
    .add_transient_event_listener(
        |event| &mut event.advance_cost,
        1,
        |i, &a, _, p| {
            if a == Advance::Engineering || a == Advance::Roads {
                i.set_zero_resources(p);
            }
        },
    )
    .with_advance_bonus(CultureToken)
    .with_unlocked_building(Building::Observatory)
}

pub fn use_observatory() -> Ability {
    Ability::builder("Observatory", "Gain 1 action card")
        .add_simple_persistent_event_listener(
            |event| &mut event.construct,
            4,
            |game, p, b| {
                if b.building == Building::Observatory {
                    gain_action_card_from_pile(game, p);
                    p.log(game, "Observatory gained 1 action card");
                }
            },
        )
        .build()
}

fn astronomy() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Astronomy,
        "Astronomy",
        "Navigation and Cartography can be bought at no food cost",
    )
    .add_transient_event_listener(
        |event| &mut event.advance_cost,
        0,
        |i, &a, _, p| {
            if a == Advance::Navigation || a == Advance::Cartography {
                i.set_zero_resources(p);
            }
        },
    )
    .with_advance_bonus(CultureToken)
}

fn medicine() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Medicine,
        "Medicine",
        "After recruiting, gain one of the paid resources back",
    )
    .with_advance_bonus(CultureToken)
    .add_resource_request(
        |event| &mut event.recruit,
        0,
        |_game, p, recruit| {
            let types: Vec<ResourceType> = ResourceType::all()
                .into_iter()
                .filter(|r| recruit.payment.get(r) > 0 && r.is_resource())
                .collect();

            if types.is_empty() {
                return None;
            }

            Some(ResourceRewardRequest::new(
                p.reward_options().sum(1, &types),
                "Select resource to gain back".to_string(),
            ))
        },
    )
}

fn metallurgy() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Metallurgy,
        "Metallurgy",
        "If you have the Steel Weapons Advance, \
        you no longer have to pay 1 ore to activate it against enemies without Steel Weapons. \
        If you collect at least 2 ore, you may replace 1 ore with 1 gold",
    )
    .with_advance_bonus(CultureToken)
    .add_bool_request(
        |event| &mut event.collect,
        0,
        |_game, _p, i| {
            (i.total.ore >= 2).then_some("Do you want to convert 1 ore to 1 gold?".to_string())
        },
        move |game, s, _| {
            if s.choice {
                s.player().lose_resources(game, ResourcePile::ore(1));
                s.player().gain_resources(game, ResourcePile::gold(1));
            } else {
                s.log(game, "Did not convert ore to gold");
            }
        },
    )
}
