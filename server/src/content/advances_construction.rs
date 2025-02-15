use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::content::advances::{advance_group_builder, AdvanceGroup, ROADS};
use crate::content::custom_actions::CustomActionType::ConstructWonder;
use crate::game::Game;
use crate::payment::PaymentConversion;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;

pub(crate) fn construction() -> AdvanceGroup {
    advance_group_builder(
        "Construction",
        vec![
            Advance::builder("Mining", "Your cities may Collect ore from Mountain spaces"),
            engineering(),
            sanitation(),
            Advance::builder(ROADS, "When moving from or to a city, you may pay 1 food and 1 ore to extend the range of a group of land units by 1 and ignore terrain effects. May not be used to embark, disembark, or explore")
                .with_advance_bonus(CultureToken)
        ],
    )
}

fn engineering() -> AdvanceBuilder {
    Advance::builder(
        "Engineering",
        "Immediately draw 1 wonder card. May Construct wonders in happy cities",
    )
    .add_one_time_ability_initializer(Game::draw_wonder_card)
    .add_custom_action(ConstructWonder)
}

fn sanitation() -> AdvanceBuilder {
    Advance::builder(
        "Sanitation",
        "When Recruiting, you may spend 1 mood token to pay for 1 Settler.",
    )
    .with_advance_bonus(MoodToken)
    .add_player_event_listener(
        |event| &mut event.recruit_cost,
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
        1,
    )
}
