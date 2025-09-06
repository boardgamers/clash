use crate::city::{MoodState, increase_mood_state};
use crate::content::custom_actions::{
    PlayingActionModifier, SpecialAction, custom_action_modifier_event_origin,
};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::leader::leader_position;
use crate::payment::PaymentOptions;
use crate::player::{CostTrigger, Player};
use crate::player_events::CostInfo;
use crate::playing_actions::{PlayingActionType, base_or_modified_available};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct IncreaseHappiness {
    pub happiness_increases: Vec<(Position, u8)>,
    pub payment: ResourcePile,
    pub action_type: PlayingActionType,
}

impl IncreaseHappiness {
    #[must_use]
    pub fn new(
        happiness_increases: Vec<(Position, u8)>,
        payment: ResourcePile,
        action_type: PlayingActionType,
    ) -> Self {
        Self {
            happiness_increases,
            payment,
            action_type,
        }
    }
}

#[must_use]
pub fn available_happiness_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_modified_available(game, player, &PlayingActionType::IncreaseHappiness)
}

#[must_use]
pub fn happiness_city_restriction(player: &Player, action: &PlayingActionType) -> Option<Position> {
    match action {
        PlayingActionType::Special(SpecialAction::Modifier(m))
            if m == &PlayingActionModifier::StatesmanIncreaseHappiness =>
        {
            Some(leader_position(player))
        }
        _ => None,
    }
}

pub(crate) fn execute_increase_happiness(
    game: &mut Game,
    player_index: usize,
    happiness_increases: &[(Position, u8)],
    payment: &ResourcePile,
    already_paid: bool,
    action_type: &PlayingActionType,
    origin: &EventOrigin,
) -> Result<(), String> {
    let trigger = game.execute_cost_trigger();
    let restriction = happiness_city_restriction(game.player(player_index), action_type);
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

        let city = game.player(player_index).get_city(city_position);
        step_sum += steps * city.size() as u8;

        if city.mood_state == MoodState::Angry {
            angry_activations.push(city_position);
        }
        increase_mood_state(game, city_position, steps, origin);
    }

    if !already_paid {
        happiness_cost(player_index, step_sum, trigger, action_type, game, origin)
            .pay(game, payment);
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
    origin: &EventOrigin,
) -> CostInfo {
    let p = game.player(player);
    let mut payment_options = PaymentOptions::sum(
        p,
        origin.clone(),
        city_size_steps,
        &[ResourceType::MoodTokens],
    );
    // either none or both can use Colosseum
    payment_options.default += action_type.payment_options(game, player).default;

    p.trigger_cost_event(
        |e| &e.happiness_cost,
        CostInfo::new(p, payment_options),
        &(),
        &(),
        execute,
    )
}

pub(crate) fn happiness_event_origin(
    action_type: &PlayingActionType,
    player: &Player,
) -> EventOrigin {
    custom_action_modifier_event_origin(happiness_base_event_origin(), action_type, player)
}

pub(crate) fn happiness_base_event_origin() -> EventOrigin {
    EventOrigin::Ability("Increase Happiness".to_string())
}
