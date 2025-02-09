use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Temple;
use crate::content::advances::{advance_group_builder, AdvanceGroup};
use crate::content::custom_phase_actions::CustomPhaseResourceRewardRequest;
use crate::payment::{PaymentConversion, PaymentOptions};
use crate::resource::ResourceType;
use crate::resource::ResourceType::{CultureTokens, MoodTokens};
use crate::resource_pile::ResourcePile;

pub(crate) fn spirituality() -> AdvanceGroup {
    advance_group_builder(
        "Spirituality",
        vec![
            myths(),
            rituals(),
            Advance::builder("State Religion", "not implemented"),
        ],
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
                format!("{player_name} selected {p} as a reward for constructing a Temple",)
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
                    cost.conversions.push(PaymentConversion::unlimited(vec![ResourcePile::mood_tokens(1)], ResourcePile::of(*r, 1)));
                }
            },
            0,
        )
}
