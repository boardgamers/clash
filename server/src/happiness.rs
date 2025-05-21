use crate::city::MoodState;
use crate::content::custom_actions::CustomActionType;
use crate::game::Game;
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player::CostTrigger;
use crate::player_events::CostInfo;
use crate::playing_actions::{PlayingActionType, base_or_custom_available};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

#[must_use]
pub fn available_happiness_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::IncreaseHappiness,
        CustomActionType::VotingIncreaseHappiness,
    )
}

pub(crate) fn increase_happiness(
    game: &mut Game,
    player_index: usize,
    happiness_increases: &[(Position, u8)],
    payment: Option<ResourcePile>,
    action_type: &PlayingActionType,
) {
    let trigger = game.execute_cost_trigger();
    let player = &mut game.players[player_index];
    let mut angry_activations = vec![];
    let mut step_sum = 0;
    for &(city_position, steps) in happiness_increases {
        let city = player.get_city(city_position);
        if steps == 0 {
            continue;
        }
        step_sum += steps * city.size() as u8;

        if city.mood_state == MoodState::Angry {
            angry_activations.push(city_position);
        }
        let city = player.get_city_mut(city_position);
        for _ in 0..steps {
            city.increase_mood_state();
        }
    }

    if let Some(r) = payment {
        happiness_cost(player_index, step_sum, trigger, action_type, game).pay(game, &r);
    }
}

#[must_use]
pub fn happiness_cost(
    player: usize,
    city_size_steps: u8, // for each city: size * steps in that city
    execute: CostTrigger,
    action_type: &PlayingActionType,
    game: &Game,
) -> CostInfo {
    let p = game.player(player);
    let mut payment_options = PaymentOptions::sum(
        p,
        PaymentReason::IncreaseHappiness,
        city_size_steps,
        &[ResourceType::MoodTokens],
    );
    // either none or both can use Colosseum
    payment_options.default += action_type.cost(game).payment_options(p).default;

    p.trigger_cost_event(|e| &e.happiness_cost, &payment_options, &(), &(), execute)
}
