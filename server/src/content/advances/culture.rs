use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city::{City, MoodState};
use crate::city_pieces::Building::Obelisk;
use crate::content::ability::{Ability, AbilityBuilder};
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::custom_actions::{
    CustomActionActionExecution, CustomActionExecution, CustomActionType, any_non_happy,
};
use crate::content::persistent_events::PaymentRequest;
use crate::happiness::execute_increase_happiness;
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player::{Player, gain_resources};
use crate::playing_actions::PlayingActionType;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::wonder::{Wonder, draw_wonder_card};
use std::sync::Arc;
use std::vec;

pub(crate) fn culture() -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Culture,
        "Culture",
        vec![arts(), sports(), monuments(), theaters()],
    )
}

fn arts() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Arts,
        "Arts",
        "Once per turn, as a free action, you may spend \
            1 culture token to get an influence culture action",
    )
    .with_advance_bonus(CultureToken)
    .with_unlocked_building(Obelisk)
    .add_action_modifier(
        CustomActionType::ArtsInfluenceCultureAttempt,
        |c| {
            c.once_per_turn()
                .free_action()
                .resources(ResourcePile::culture_tokens(1))
        },
        PlayingActionType::InfluenceCultureAttempt,
    )
}

const SPORTS_DESC: &str = "As an action, you may spend \
        1 or 2 culture tokens to increase the happiness of a city by 1 or 2, respectively";

fn sports() -> AdvanceBuilder {
    AdvanceInfo::builder(Advance::Sports, "Sports", SPORTS_DESC)
        .with_advance_bonus(MoodToken)
        .add_custom_action_execution(
            CustomActionType::Sports,
            |c| c.any_times().action().no_resources(),
            CustomActionExecution::Action(CustomActionActionExecution::new(
                use_sports(),
                Arc::new(|_game, p| can_use_sports(p)),
                Some(Arc::new(|game, city| {
                    let p = game.player(city.player_index);
                    sports_options(p, city).is_some_and(|c| c.can_afford(&p.resources))
                })),
            )),
        )
}

fn monuments() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Monuments,
        "Monuments",
        "Immediately draw 1 wonder card. \
        Your cities with wonders may not be the target of influence culture attempts",
    )
    .add_one_time_ability_initializer(draw_wonder_card)
    .with_advance_bonus(CultureToken)
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        1,
        |r, city, _| {
            if let Ok(info) = r {
                if info.is_defender && !city.pieces.wonders.is_empty() {
                    *r = Err(
                        "Monuments prevent influence culture attempts on cities with wonders"
                            .to_string(),
                    );
                }
            }
        },
    )
}

fn theaters() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Theaters,
        "Theaters",
        "Once per turn, as a free action, you may convert 1 culture token \
        into 1 mood token, or 1 mood token into 1 culture token",
    )
    .with_advance_bonus(MoodToken)
    .add_custom_action(
        CustomActionType::Theaters,
        |c| c.once_per_turn().free_action().no_resources(),
        use_theaters,
        |_game, p| p.resources.culture_tokens > 0 || p.resources.mood_tokens > 0,
    )
}

#[must_use]
pub fn sports_options(player: &Player, city: &City) -> Option<PaymentOptions> {
    match city.mood_state {
        MoodState::Happy => None,
        MoodState::Neutral => Some(PaymentOptions::sum(
            player,
            PaymentReason::CustomAction,
            1,
            &[ResourceType::CultureTokens],
        )),
        MoodState::Angry => Some(PaymentOptions::single_type(
            player,
            PaymentReason::CustomAction,
            ResourceType::CultureTokens,
            1..=2,
        )),
    }
}

fn can_use_sports(p: &Player) -> bool {
    if !any_non_happy(p) {
        return false;
    }
    if p.resources.culture_tokens > 0 {
        return true;
    }
    p.wonders_owned.contains(Wonder::Colosseum) && p.resources.mood_tokens > 0
}

fn use_sports() -> Ability {
    Ability::builder("Sports", SPORTS_DESC)
        .add_payment_request_listener(
            |event| &mut event.custom_action,
            0,
            |game, player_index, a| {
                let p = game.player(player_index);
                let options = sports_options(p, p.get_city(a.action.city.expect("city not found")))
                    .expect("Invalid options for sports");
                Some(vec![PaymentRequest::mandatory(
                    options,
                    "Each culture token increases the happiness by 1 step",
                )])
            },
            |game, s, a| {
                let position = a.action.city.expect("city not found");
                let pile = s.choice[0].clone();
                let steps = pile.amount();
                execute_increase_happiness(
                    game,
                    s.player_index,
                    &[(position, steps)],
                    pile,
                    true,
                    &a.action.action.playing_action_type(),
                )
                .expect("Failed to increase happiness");
            },
        )
        .build()
}

fn use_theaters(b: AbilityBuilder) -> AbilityBuilder {
    b.add_payment_request_listener(
        |event| &mut event.custom_action,
        0,
        |game, player, _| {
            Some(vec![PaymentRequest::mandatory(
                PaymentOptions::sum(
                    game.player(player),
                    PaymentReason::CustomAction,
                    1,
                    &[ResourceType::CultureTokens, ResourceType::MoodTokens],
                ),
                "Convert 1 culture token into 1 mood token, or 1 mood token into 1 culture token",
            )])
        },
        |game, s, _| {
            gain_resources(
                game,
                s.player_index,
                theater_opposite(&s.choice[0]),
                |name, pile| {
                    format!(
                        "{name} used Theaters to convert {} into {pile}",
                        s.choice[0],
                    )
                },
            );
        },
    )
}

fn theater_opposite(payment: &ResourcePile) -> ResourcePile {
    if payment.mood_tokens > 0 {
        ResourcePile::culture_tokens(1)
    } else {
        ResourcePile::mood_tokens(1)
    }
}
