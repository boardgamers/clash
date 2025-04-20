use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::content::advances::{AdvanceGroup, advance_group_builder};
use crate::payment::PaymentConversion;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use crate::wonder::draw_wonder_card;

pub(crate) fn construction() -> AdvanceGroup {
    advance_group_builder(
        "Construction",
        vec![mining(), engineering(), sanitation(), roads()],
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
        "Immediately draw 1 wonder card. May Construct wonders in happy cities",
    )
    .add_one_time_ability_initializer(draw_wonder_card)
}

fn sanitation() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Sanitation,
        "Sanitation",
        "When Recruiting, you may spend 1 mood token to pay for 1 Settler. \
        Ignore Pestilence and Epidemics events.",
    )
    .with_advance_bonus(MoodToken)
    .add_transient_event_listener(
        |event| &mut event.recruit_cost,
        1,
        |cost, units, _| {
            if units.settlers > 0 {
                // insert at beginning so that it's preferred over gold
                cost.info
                    .log
                    .push("Sanitation reduced the cost of 1 Settler to 1 mood token".to_string());

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
        "When moving from or to a city, you may pay 1 food and 1 ore \
    to extend the range of a group of land units by 1 and ignore terrain effects. \
    May not be used to embark, disembark, or explore",
    )
    .with_advance_bonus(CultureToken)
}
