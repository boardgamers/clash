use crate::city::{MoodState, increase_mood_state};
use crate::content::custom_actions::{CustomActionType, custom_action_modifier_event_origin};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::leader::{Leader, leader_position};
use crate::payment::PaymentOptions;
use crate::player::{CostTrigger, Player};
use crate::player_events::CostInfo;
use crate::playing_actions::{PlayingActionType, base_or_custom_available};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct IncreaseHappiness {
    pub happiness_increases: Vec<(Position, u8)>,
    pub payment: ResourcePile,
    pub action_type: PlayingActionType,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lawgiver: bool,
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
            lawgiver: false,
        }
    }
}

#[must_use]
pub fn available_happiness_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(game, player, &PlayingActionType::IncreaseHappiness)
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

pub(crate) fn execute_increase_happiness(
    game: &mut Game,
    player_index: usize,
    happiness_increases: &[(Position, u8)],
    payment: &ResourcePile,
    already_paid: bool,
    lawgiver: bool,
    action_type: &PlayingActionType,
    origin: &EventOrigin,
) -> Result<(), String> {
    let trigger = game.execute_cost_trigger();
    let restriction = happiness_city_restriction(game.player(player_index), action_type);
    for (index, &(city_position, steps)) in happiness_increases.iter().enumerate() {
        if steps == 0 {
            continue;
        }
        if restriction.is_some_and(|r| r != city_position) {
            return Err(format!(
                "Cannot increase happiness in city {city_position}, \
                 only in {restriction:?} with {action_type:?}"
            ));
        }

        let city = game
            .player(player_index)
            .try_get_city(city_position)
            .ok_or("Choose your city")?;
        let max = match city.mood_state {
            MoodState::Happy => 0,
            MoodState::Neutral => 1,
            MoodState::Angry => 2,
        };
        if steps > max
            || happiness_increases[..index]
                .iter()
                .any(|(p, _)| *p == city_position)
        {
            return Err("Choose valid happiness increases".to_string());
        }
    }

    let cost = if !already_paid {
        Some(happiness_cost_for_cities(
            game,
            player_index,
            happiness_increases,
            lawgiver,
            trigger,
            action_type,
            origin,
        )?)
    } else {
        None
    };
    for &(city_position, steps) in happiness_increases {
        if steps > 0 {
            increase_mood_state(game, city_position, steps, origin);
        }
    }
    if let Some(cost) = cost {
        cost.pay(game, payment);
    }

    Ok(())
}

#[must_use]
pub(crate) fn lawgiver_city(player: &Player) -> Option<Position> {
    player
        .units
        .iter()
        .find(|u| u.unit_type == Leader::Hammurabi.unit_type())
        .and_then(|u| player.try_get_city(u.position).map(|c| c.position))
}

pub(crate) fn happiness_cost_for_cities(
    game: &Game,
    player: usize,
    cities: &[(Position, u8)],
    lawgiver: bool,
    execute: CostTrigger,
    action_type: &PlayingActionType,
    origin: &EventOrigin,
) -> Result<CostInfo, String> {
    let p = game.player(player);
    let lawgiver_position = if lawgiver {
        Some(lawgiver_city(p).ok_or("Lawgiver needs Hammurabi in your city")?)
    } else {
        None
    };
    let mut used_lawgiver = false;
    let mut size_steps = 0;
    for &(position, steps) in cities {
        let city = p.try_get_city(position).ok_or("Choose your city")?;
        if Some(position) == lawgiver_position {
            let to_happy = match city.mood_state {
                MoodState::Happy => 0,
                MoodState::Neutral => 1,
                MoodState::Angry => 2,
            };
            if steps == 0 || steps != to_happy {
                return Err("Lawgiver makes Hammurabi's city happy".into());
            }
            used_lawgiver = true;
        } else {
            size_steps += city.size() as u8 * steps;
        }
    }
    if lawgiver && !used_lawgiver {
        return Err("Select Hammurabi's city for Lawgiver".into());
    }
    let mut cost = happiness_cost(player, size_steps, execute, action_type, game, origin);
    if lawgiver {
        cost.cost.default += ResourcePile::culture_tokens(1);
        cost.info.add_log(
            &crate::events::EventPlayer::from_player(
                player,
                game,
                EventOrigin::LeaderAbility("Lawgiver".into()),
            ),
            "Make Hammurabi's city happy for 1 culture token",
        );
    }
    Ok(cost)
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
