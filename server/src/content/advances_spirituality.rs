use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Temple;
use crate::content::advances::{advance_group_builder, get_group, AdvanceGroup};
use crate::content::custom_phase_actions::CustomPhaseResourceRewardRequest;
use crate::payment::{PaymentConversion, PaymentOptions};
use crate::resource::ResourceType;
use crate::resource::ResourceType::{CultureTokens, MoodTokens};
use crate::resource_pile::ResourcePile;

pub(crate) fn spirituality() -> AdvanceGroup {
    advance_group_builder(
        "Spirituality",
        vec![myths(), rituals(), priesthood(), stata_religion()],
    )
}

fn myths() -> AdvanceBuilder {
    Advance::builder("Myths", "not implemented")
        .with_advance_bonus(MoodToken)
        .with_unlocked_building(Temple)
        .add_resource_reward_request_listener(
            |event| &mut event.on_construct,
            1,
            |_game, _player_index, building| {
                if matches!(building, Temple) {
                    return Some(CustomPhaseResourceRewardRequest {
                        reward: PaymentOptions::sum(1, &[MoodTokens, CultureTokens]),
                        name: "Select Temple bonus".to_string(),
                    });
                }
                None
            },
            |_game, _player_index, player_name, p, _selected| {
                format!("{player_name} selected {p} as a reward for constructing a Temple")
            },
        )
}

fn rituals() -> AdvanceBuilder {
    Advance::builder("Rituals", "When you perform the Increase Happiness Action you may spend any Resources as a substitute for mood tokens. This is done at a 1:1 ratio")
        .with_advance_bonus(CultureToken)
        .add_player_event_listener(
            |event| &mut event.happiness_cost,
            |cost, (), ()| {
                for r in &[
                    ResourceType::Food,
                    ResourceType::Wood,
                    ResourceType::Ore,
                    ResourceType::Ideas,
                    ResourceType::Gold,
                ] {
                    cost.log.push(". Rituals allows spending any resource as a substitute for mood tokens".to_string());
                    cost.cost.conversions.push(PaymentConversion::unlimited(vec![ResourcePile::mood_tokens(1)], ResourcePile::of(*r, 1)));
                }
            },
            0,
        )
}

fn priesthood() -> AdvanceBuilder {
    let name = "Priesthood";
    Advance::builder(name, "Once per turn, a science advance is free").add_player_event_listener(
        |event| &mut event.advance_cost,
        |i, advance, ()| {
            if !i.info.contains_key(name)
                && get_group("Science").advances.iter().any(|a| a == advance)
            {
                i.set_zero();
                i.log.push(". Priesthood reduced the cost to 0".to_string());
                i.info.insert(name.to_string(), "used".to_string());
            }
        },
        0,
    )
}

fn stata_religion() -> AdvanceBuilder {
    let name = "State Religion";
    Advance::builder(
        name,
        "Once per turn, when constructing a Temple,
            do not pay any Food.",
    )
    .with_advance_bonus(MoodToken)
    .add_player_event_listener(
        |event| &mut event.construct_cost,
        |i, _city, b| {
            if !i.info.contains_key(name) && matches!(b, Temple) {
                i.cost.conversions.push(PaymentConversion::limited(
                    vec![ResourcePile::of(ResourceType::Food, 1)],
                    ResourcePile::empty(),
                    1,
                ));
                i.info.insert(name.to_string(), "used".to_string());
                i.log
                    .push(". State Religion reduced the food cost to 0".to_string());
            }
        },
        0,
    )
}
