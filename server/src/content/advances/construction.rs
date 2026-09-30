use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::custom_actions::CustomActionType;
use crate::game::GameOptions;
use crate::payment::PaymentConversion;
use crate::playing_actions::PlayingActionType;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use crate::wonder::draw_wonder_card;

pub(crate) fn construction(options: &GameOptions) -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Construction,
        "Construction",
        options,
        vec![
            mining(),
            engineering(),
            sanitation(),
            city_planning(), // balance
            roads(),
        ],
    )
}

fn mining() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Mining,
        "Mining",
        "Your cities may Collect ore from Mountain spaces",
    )
}

fn engineering() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Engineering,
        "Engineering",
        "Immediately draw 1 wonder card. You may construct wonders in your happy cities by activating the city and paying the wonder cost.",
    )
    .add_once_initializer(draw_wonder_card)
}

fn sanitation() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Sanitation,
        "Sanitation",
        "During each Recruit action, you may pay 1 mood token instead of resources for one Settler. Other units use their normal costs. You ignore Pestilence and Epidemics events.",
    )
    .with_advance_bonus(MoodToken)
    .add_transient_event_listener(
        |event| &mut event.recruit_cost,
        1,
        |cost, units, _, p| {
            if units.settlers > 0 {
                // insert at beginning so that it's preferred over gold
                cost.info
                    .add_log(p, "Reduce the cost of 1 Settler to 1 mood token");

                cost.cost.conversions.insert(
                    0,
                    PaymentConversion::limited(
                        UnitType::cost(&UnitType::Settler),
                        ResourcePile::mood_tokens(1),
                        1,
                    ),
                );
            }
        },
    )
}

fn roads() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Roads,
        "Roads",
        "When moving land units to or from one of your cities, you may pay 1 food and 1 ore per unit or group to move up to 2 spaces and ignore terrain penalties. Roads cannot be used to explore, embark or disembark.",
    )
    .with_advance_bonus(CultureToken)
}

fn city_planning() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::CityPlanning,
        "City Planning",
        "Once per turn, as a free action, you may pay 1 idea and 1 wood to get a free Construct action.",
    )
        .replaces(Advance::Sanitation)
    .add_action_modifier(CustomActionType::CityPlanning, |cost| cost.once_per_turn().free_action().resources(ResourcePile::ideas(1) + ResourcePile::wood(1)), PlayingActionType::Construct)
}
