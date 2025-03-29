use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::{CultureToken, MoodToken};
use crate::advance::{Advance, AdvanceBuilder};
use crate::city::{City, MoodState};
use crate::city_pieces::Building::Obelisk;
use crate::content::advances::{advance_group_builder, AdvanceGroup};
use crate::content::custom_actions::CustomActionType;
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::playing_actions::increase_happiness;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::wonder::draw_wonder_card;
use std::vec;

pub(crate) fn culture() -> AdvanceGroup {
    advance_group_builder("Culture", vec![arts(), sports(), monuments(), theaters()])
}

fn arts() -> AdvanceBuilder {
    Advance::builder(
        "Arts",
        "Once per turn, as a free action, you may spend \
        1 culture token to get an influence culture action",
    )
    .with_advance_bonus(CultureToken)
    .with_unlocked_building(Obelisk)
    .add_custom_action(CustomActionType::ArtsInfluenceCultureAttempt)
}

fn sports() -> AdvanceBuilder {
    Advance::builder(
        "Sports",
        "As an action, you may spend \
        1 or 2 culture tokens to increase the happiness of a city by 1 or 2, respectively",
    )
    .with_advance_bonus(MoodToken)
    .add_custom_action(CustomActionType::Sports)
}

fn monuments() -> AdvanceBuilder {
    Advance::builder(
        "Monuments",
        "Immediately draw 1 wonder card. \
        Your cities with wonders may not be the target of influence culture attempts",
    )
    .add_one_time_ability_initializer(draw_wonder_card)
    .with_advance_bonus(CultureToken)
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        1,
        |info, city, _| {
            if info.is_defender && !city.pieces.wonders.is_empty() {
                info.add_blocker(
                    "Monuments prevent influence culture attempts on cities with wonders",
                );
            }
        },
    )
}

fn theaters() -> AdvanceBuilder {
    Advance::builder(
        "Theaters",
        "Once per turn, as a free action, you may convert 1 culture token \
        into 1 mood token, or 1 mood token into 1 culture token",
    )
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

pub(crate) fn execute_sports(
    game: &mut Game,
    player_index: usize,
    pos: Position,
    payment: &ResourcePile,
) {
    let options = sports_options(game.get_city(player_index, pos));
    game.players[player_index].pay_cost(&options.expect("sports not possible"), payment);
    increase_happiness(game, player_index, &[(pos, payment.culture_tokens)], None);
}

#[must_use]
pub fn theaters_options() -> PaymentOptions {
    PaymentOptions::sum(1, &[ResourceType::CultureTokens, ResourceType::MoodTokens])
}

pub(crate) fn execute_theaters(game: &mut Game, player_index: usize, payment: &ResourcePile) {
    game.players[player_index].gain_resources(theater_opposite(payment));
    game.players[player_index].pay_cost(&theaters_options(), payment);
}

fn theater_opposite(payment: &ResourcePile) -> ResourcePile {
    if payment.mood_tokens > 0 {
        ResourcePile::culture_tokens(1)
    } else {
        ResourcePile::mood_tokens(1)
    }
}
