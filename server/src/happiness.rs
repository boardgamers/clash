use crate::city::MoodState;
use crate::content::custom_actions::CustomActionType;
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::CostInfo;
use crate::playing_actions::{
    base_or_custom_available, PlayingActionType,
};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

#[must_use]
pub fn available_happiness_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::IncreaseHappiness,
        &CustomActionType::VotingIncreaseHappiness,
    )
}

pub(crate) fn increase_happiness(
    game: &mut Game,
    player_index: usize,
    happiness_increases: &[(Position, u32)],
    payment: Option<ResourcePile>,
) {
    let player = &mut game.players[player_index];
    let mut angry_activations = vec![];
    let mut step_sum = 0;
    for &(city_position, steps) in happiness_increases {
        let city = player.get_city(city_position);
        if steps == 0 {
            continue;
        }
        step_sum += steps * city.size() as u32;

        if city.mood_state == MoodState::Angry {
            angry_activations.push(city_position);
        }
        let city = player.get_city_mut(city_position);
        for _ in 0..steps {
            city.increase_mood_state();
        }
    }

    if let Some(r) = payment {
        happiness_cost(player, step_sum, Some(&r)).pay(game, &r);
    }
}

#[must_use]
pub fn happiness_cost(
    p: &Player,
    city_size_steps: u32, // for each city: size * steps in that city
    execute: Option<&ResourcePile>,
) -> CostInfo {
    let payment_options = PaymentOptions::sum(city_size_steps, &[ResourceType::MoodTokens]);
    p.trigger_cost_event(|e| &e.happiness_cost, &payment_options, &(), &(), execute)
}
