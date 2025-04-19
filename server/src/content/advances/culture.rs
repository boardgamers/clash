use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{AdvanceInfo, AdvanceBuilder, Advance};
use crate::city::{City, MoodState};
use crate::city_pieces::Building::Obelisk;
use crate::content::advances::{AdvanceGroup, advance_group_builder};
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::PaymentRequest;
use crate::happiness::increase_happiness;
use crate::payment::PaymentOptions;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::wonder::draw_wonder_card;
use std::vec;

pub(crate) fn culture() -> AdvanceGroup {
    advance_group_builder("Culture", vec![arts(), sports(), monuments(), theaters()])
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
    .add_custom_action(CustomActionType::ArtsInfluenceCultureAttempt)
}

const SPORTS_DESC: &str = "As an action, you may spend \
        1 or 2 culture tokens to increase the happiness of a city by 1 or 2, respectively";

fn sports() -> AdvanceBuilder {
    AdvanceInfo::builder(
                Advance::Sports,
        "Sports", SPORTS_DESC)
        .with_advance_bonus(MoodToken)
        .add_custom_action(CustomActionType::Sports)
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

const THEATERS_DESC: &str = "Once per turn, as a free action, you may convert 1 culture token \
        into 1 mood token, or 1 mood token into 1 culture token";

fn theaters() -> AdvanceBuilder {
    AdvanceInfo::builder(
                Advance::Theaters,
        "Theaters", THEATERS_DESC)
        .with_advance_bonus(MoodToken)
        .add_custom_action(CustomActionType::Theaters)
}

#[must_use]
pub fn sports_options(city: &City) -> Option<PaymentOptions> {
    match city.mood_state {
        MoodState::Happy => None,
        MoodState::Neutral => Some(PaymentOptions::sum(1, &[ResourceType::CultureTokens])),
        MoodState::Angry => Some(PaymentOptions::single_type(
            ResourceType::CultureTokens,
            1..=2,
        )),
    }
}

pub(crate) fn use_sports() -> Builtin {
    Builtin::builder("Sports", SPORTS_DESC)
        .add_payment_request_listener(
            |event| &mut event.custom_action,
            0,
            |game, _player_index, a| {
                let options = sports_options(game.get_any_city(a.city.expect("city not found")))
                    .expect("Invalid options for sports");
                Some(vec![PaymentRequest::new(
                    options,
                    "Each culture token increases the happiness by 1 step",
                    false,
                )])
            },
            |game, s, a| {
                let position = a.city.expect("city not found");
                let steps = s.choice[0].culture_tokens;
                increase_happiness(
                    game,
                    s.player_index,
                    &[(position, steps)],
                    None,
                );
                game.add_info_log_item(
                    &format!(
                        "{} used Sports to increase the happiness of {} by {steps} steps, making it {:?}",
                        s.player_name,
                        position,
                        game.get_any_city(position).mood_state
                    ),
                );
            },
        )
        .build()
}

#[must_use]
pub fn theaters_options() -> PaymentOptions {
    PaymentOptions::sum(1, &[ResourceType::CultureTokens, ResourceType::MoodTokens])
}

pub(crate) fn use_theaters() -> Builtin {
    Builtin::builder("Theaters", THEATERS_DESC)
        .add_payment_request_listener(
            |event| &mut event.custom_action,
            0,
            |_game, _player_index, _| {
                Some(vec![PaymentRequest::new(
                    theaters_options(),
                    "Convert 1 culture token into 1 mood token, or 1 mood token into 1 culture token",
                    false,
                )])
            },
            |game, s, _| {
                let reward = theater_opposite(&s.choice[0]);
                game.players[s.player_index].gain_resources(reward.clone());
                game.add_info_log_item(
                    &format!(
                        "{} used Theaters to convert {} into {}",
                        s.player_name,
                        s.choice[0],
                        reward
                    ),
                );
            },
        )
        .build()
}

fn theater_opposite(payment: &ResourcePile) -> ResourcePile {
    if payment.mood_tokens > 0 {
        ResourcePile::culture_tokens(1)
    } else {
        ResourcePile::mood_tokens(1)
    }
}
