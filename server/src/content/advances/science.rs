use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::advance::Bonus::CultureToken;
use crate::advance::{AdvanceInfo, AdvanceBuilder, Advance};
use crate::city_pieces::Building;
use crate::content::advances::{AdvanceGroup, METALLURGY, advance_group_builder};
use crate::content::persistent_events::ResourceRewardRequest;
use crate::payment::PaymentOptions;
use crate::resource::ResourceType;

pub(crate) fn science() -> AdvanceGroup {
    advance_group_builder(
        "Science",
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
        |i, a, ()| {
            if a.name == "Engineering" || a.name == "Roads" {
                i.info.log.push("Math reduced the cost to 0".to_string());
                i.set_zero();
            }
        },
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.construct,
        4,
        |game, player_index, _player_name, b| {
            if matches!(b, Building::Observatory) {
                gain_action_card_from_pile(game, player_index);
                game.add_info_log_item("Observatory gained 1 action card");
            }
        },
    )
    .with_advance_bonus(CultureToken)
    .with_unlocked_building(Building::Observatory)
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
        |i, a, ()| {
            if a.name == "Navigation" || a.name == "Cartography" {
                i.set_zero();
                i.info
                    .log
                    .push("Astronomy reduced the cost to 0".to_string());
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
        |_game, _player_index, recruit| {
            let types: Vec<ResourceType> = ResourceType::all()
                .into_iter()
                .filter(|r| recruit.payment.get(r) > 0 && r.is_resource())
                .collect();

            if types.is_empty() {
                return None;
            }

            Some(ResourceRewardRequest::new(
                PaymentOptions::sum(1, &types),
                "Select resource to gain back".to_string(),
            ))
        },
        |_game, s, _| {
            let verb = if s.actively_selected {
                "selected"
            } else {
                "gained"
            };
            vec![format!(
                "{} {verb} {} for Medicine Advance",
                s.player_name, s.choice
            )]
        },
    )
}

fn metallurgy() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Metallurgy,
        "Metallurgy",
        "If you have the Steel Weapons Advance, \
        you no longer have to pay 1 ore to activate it against enemies without Steel Weapons. \
        If you collect at least 2 ore, replace 1 ore with 1 gold",
    )
    .with_advance_bonus(CultureToken)
    .add_transient_event_listener(
        |event| &mut event.collect_total,
        0,
        |i, _, ()| {
            if i.total.ore >= 2 {
                i.total.ore -= 1;
                i.total.gold += 1;
                i.info
                    .log
                    .push("Metallurgy converted 1 ore to 1 gold".to_string());
            }
        },
    )
}
