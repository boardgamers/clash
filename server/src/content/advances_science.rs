use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::CultureToken;
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Observatory;
use crate::content::advances::{advance_group_builder, AdvanceGroup, METALLURGY};
use crate::content::custom_phase_actions::ResourceRewardRequest;
use crate::payment::PaymentOptions;
use crate::resource::ResourceType;

pub(crate) fn science() -> AdvanceGroup {
    advance_group_builder(
        "Science",
        vec![math(), astronomy(), medicine(), metallurgy()],
    )
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
                i.info.log.push("Math reduced the cost to 0".to_string());
                i.set_zero();
            }
        },
        0,
    )
    .with_advance_bonus(CultureToken)
    .with_unlocked_building(Observatory)
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
                i.info
                    .log
                    .push("Astronomy reduced the cost to 0".to_string());
            }
        },
        0,
    )
    .with_advance_bonus(CultureToken)
}

fn medicine() -> AdvanceBuilder {
    Advance::builder(
        "Medicine",
        "After recruiting, gain one of the paid resources back",
    )
    .with_advance_bonus(CultureToken)
    .add_resource_request(
        |event| &mut event.on_recruit,
        0,
        |_game, _player_index, recruit| {
            let types: Vec<ResourceType> = ResourceType::all()
                .into_iter()
                .filter(|r| recruit.payment.get(r) > 0 && r.is_resource())
                .collect();

            if types.is_empty() {
                return None;
            }

            Some(ResourceRewardRequest {
                reward: PaymentOptions::sum(1, &types),
                name: "Select resource to gain back".to_string(),
            })
        },
        |_game, _player_index, player_name, resource, selected| {
            let verb = if selected { "selected" } else { "gained" };
            format!("{player_name} {verb} {resource} for Medicine Advance")
        },
    )
}

fn metallurgy() -> AdvanceBuilder {
    Advance::builder(
        METALLURGY,
        "If you have the Steel Weapons Advance, you no longer have to pay 1 ore to activate it against enemies without Steel Weapons. If you collect at least 2 ore, replace 1 ore with 1 gold",)
        .with_advance_bonus(CultureToken)
        .add_player_event_listener(
            |event| &mut event.collect_total,
            |i, (),()| {
                if i.total.ore >= 2 {
                    i.total.ore -= 1;
                    i.total.gold += 1;
                    i.info.log.push("Metallurgy converted 1 ore to 1 gold".to_string());
                }
            },
            0,
        )
}
