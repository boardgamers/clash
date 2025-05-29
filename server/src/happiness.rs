use crate::city::MoodState;
use crate::content::custom_actions::{CustomActionType, happiness_modifiers};
use crate::game::Game;
use crate::leader::leader_position;
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player::{CostTrigger, Player};
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
        happiness_modifiers(),
    )
}

#[must_use]
pub fn happiness_city_restriction(player: &Player, action: &PlayingActionType) -> Option<Position> {
    match action {
        PlayingActionType::Custom(custom)
            if custom == &CustomActionType::StatesmanIncreaseHappiness =>
        {
            Some(leader_position(player))
        }
        _ => None,
    }
}

pub(crate) fn increase_happiness(
    game: &mut Game,
    player_index: usize,
    happiness_increases: &[(Position, u8)],
    payment: Option<ResourcePile>,
    action_type: &PlayingActionType,
) -> Result<(), String> {
    let trigger = game.execute_cost_trigger();
    let player = &mut game.players[player_index];
    let restriction = happiness_city_restriction(player, action_type);
    let mut angry_activations = vec![];
    let mut step_sum = 0;
    for &(city_position, steps) in happiness_increases {
        if steps == 0 {
            continue;
        }
        if restriction.is_some_and(|r| r != city_position) {
            return Err(format!(
                "Cannot increase happiness in city {city_position}, \
                 only in {restriction:?} with {action_type:?}"
            ));
        }

        let city = player.get_city(city_position);
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
    Ok(())
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
