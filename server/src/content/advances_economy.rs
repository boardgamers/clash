use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Market;
use crate::content::advances::{advance_group_builder, AdvanceGroup, CURRENCY};
use crate::content::custom_phase_actions::CustomPhaseResourceRewardRequest;
use crate::content::trade_routes::{trade_route_log, trade_route_reward};

pub(crate) fn economy() -> AdvanceGroup {
    advance_group_builder(
        "Economy",
        vec![
            Advance::builder("Bartering", "todo")
                .with_advance_bonus(MoodToken)
                .with_unlocked_building(Market),
            trade_routes(),
            Advance::builder(
                CURRENCY,
                // also for Taxation
                "You may collect gold instead of food for Trade Routes",
            )
            .with_advance_bonus(CultureToken),
        ],
    )
}

fn trade_routes() -> AdvanceBuilder {
    Advance::builder(
        "Trade Routes",
        "At the beginning of your turn, you gain 1 food for every trade route you can make, to a maximum of 4. A trade route is made between one of your Settlers or Ships and a non-Angry enemy player city within 2 spaces (without counting through unrevealed Regions). Each Settler or Ship can only be paired with one enemy player city. Likewise, each enemy player city must be paired with a different Settler or Ship. In other words, to gain X food you must have at least X Units (Settlers or Ships), each paired with X different enemy cities.")
        .add_resource_reward_request_listener(
            |event| &mut event.on_turn_start,
            0,
            |game, _player_index, ()| {
                trade_route_reward(game).map(|(reward, _routes)| {
                    CustomPhaseResourceRewardRequest {
                        reward,
                        name: "Collect trade routes reward".to_string(),
                    }
                })
            },
            |game, player_index, _player_name, p, selected| {
                let (_, routes) =
                    trade_route_reward(game).expect("No trade route reward");
                trade_route_log(game, player_index, &routes, p, selected)
            },
        )
}
