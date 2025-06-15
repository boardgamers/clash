use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::advance::{Advance, gain_advance_without_payment};
use crate::city::{increase_mood_state, MoodState};
use crate::content::action_cards::spy::spy;
use crate::content::action_cards::synergies::teachable_advances;
use crate::content::persistent_events::{AdvanceRequest, PaymentRequest, PositionRequest};
use crate::content::tactics_cards::{
    TacticsCardFactory, encircled, heavy_resistance, high_ground, high_morale, peltasts, siege,
    surprise, wedge_formation,
};
use crate::game::Game;
use crate::player::Player;
use crate::playing_actions::ActionCost;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use std::sync::Arc;
use std::vec;

pub(crate) fn inspiration_action_cards() -> Vec<ActionCard> {
    vec![
        advance(1, peltasts),
        advance(2, encircled),
        inspiration(3, encircled),
        inspiration(4, peltasts),
        hero_general(5, wedge_formation),
        hero_general(6, high_morale),
        spy(7, heavy_resistance),
        spy(8, high_morale),
        ideas(9, high_ground),
        ideas(10, surprise),
        great_ideas(11, siege),
        great_ideas(12, high_ground),
    ]
}

fn advance(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Advance",
        "Pay 1 culture token: Gain 1 advance without changing the Game Event counter.",
        ActionCost::cost(ResourcePile::culture_tokens(1)),
        |game, player, _| !possible_advances(player, game).is_empty(),
    )
    .tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| Some(AdvanceRequest::new(possible_advances(p.get(game), game))),
        |game, s, _| {
            let advance = s.choice;
            gain_advance_without_payment(
                game,
                advance,
                s.player_index,
                ResourcePile::culture_tokens(1),
                false,
            );
            s.log(
                game,
                &format!("Gain {} using the Advance action card.", advance.name(game)),
            );
        },
    )
    .build()
}

fn possible_advances(player: &Player, game: &Game) -> Vec<Advance> {
    game.cache
        .get_advances()
        .iter()
        .filter(|a| player.can_advance_free(a.advance, game))
        .map(|a| a.advance)
        .collect()
}

fn inspiration(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Inspiration",
        "Gain 1 advance for free (without changing the Game Event counter) \
        that a player owns who has a unit or city within range 2 of your units or cities.",
        ActionCost::free(),
        |game, player, _| !possible_inspiration_advances(game, player).is_empty(),
    )
    .tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            Some(AdvanceRequest::new(possible_inspiration_advances(
                game,
                p.get(game),
            )))
        },
        |game, s, _| {
            let advance = s.choice;
            gain_advance_without_payment(
                game,
                advance,
                s.player_index,
                ResourcePile::empty(),
                false,
            );
            s.log(
                game,
                &format!("Gain {} for free using Inspiration.", advance.name(game)),
            );
        },
    )
    .build()
}

pub(crate) fn possible_inspiration_advances(game: &Game, player: &Player) -> Vec<Advance> {
    let players = players_in_range2(game, player)
        .iter()
        .map(|&i| game.player(i))
        .collect_vec();

    players
        .iter()
        .flat_map(|p| teachable_advances(p, player, game))
        .collect()
}

fn players_in_range2(game: &Game, player: &Player) -> Vec<usize> {
    let my = player_positions(player);

    game.players
        .iter()
        .filter(|p| {
            if p.index == player.index || !p.is_human() {
                return false;
            }
            let other = player_positions(p);
            other
                .iter()
                .any(|pos| my.iter().any(|my_pos| my_pos.distance(*pos) <= 2))
        })
        .map(|p| p.index)
        .collect()
}

pub(crate) fn player_positions(player: &Player) -> Vec<Position> {
    player
        .units
        .iter()
        .map(|u| u.position)
        .chain(player.cities.iter().map(|c| c.position))
        .collect()
}

fn hero_general(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    let mut b = ActionCard::builder(
        id,
        "Hero General",
        "If you won a land battle this turn: Increase the mood in a city by 1. \
        You may pay 1 mood token to increase the mood in a city by 1.",
        ActionCost::free(),
        |_game, player, _| !cities_where_mood_can_increase(player).is_empty(),
    )
    .combat_requirement(Arc::new(|s, p| {
        s.is_winner(p.index) && s.is_battle() && s.battleground.is_land()
    }))
    .tactics_card(tactics_card);

    b = increase_mood(b, 2, false);
    b = b.add_payment_request_listener(
        |e| &mut e.play_action_card,
        1,
        |game, p, _| {
            let player = p.get(game);
            if cities_where_mood_can_increase(player).is_empty() {
                return None;
            }

            Some(vec![PaymentRequest::optional(
                p.payment_options()
                    .resources(player, ResourcePile::mood_tokens(1)),
                "Pay 1 mood token to increase the mood in a city by 1",
            )])
        },
        |_game, s, a| {
            if !s.choice[0].is_empty() {
                a.answer = Some(true);
            }
        },
    );
    b = increase_mood(b, 0, true);

    b.build()
}

fn increase_mood(b: ActionCardBuilder, priority: i32, need_payment: bool) -> ActionCardBuilder {
    b.add_position_request(
        |e| &mut e.play_action_card,
        priority,
        move |game, p, a| {
            if need_payment && a.answer.is_none() {
                return None;
            }
            let choices = cities_where_mood_can_increase(p.get(game));
            let needed = 1..=1;
            Some(PositionRequest::new(
                choices,
                needed,
                "Select a city to increase the mood by 1",
            ))
        },
        |game, s, _| {
            increase_mood_state(game, s.choice[0], 1, &s.origin);
        },
    )
}

fn cities_where_mood_can_increase(player: &Player) -> Vec<Position> {
    player
        .cities
        .iter()
        .filter(|c| c.mood_state != MoodState::Happy)
        .map(|c| c.position)
        .collect()
}

fn ideas(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Ideas",
        "Gain 1 idea per Academy you own.",
        ActionCost::free(),
        |_game, player, _| academies(player) > 0,
    )
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, p, _| {
            p.gain_resources(game, ResourcePile::ideas(academies(p.get(game))));
        },
    )
    .build()
}

fn academies(player: &Player) -> u8 {
    player
        .cities
        .iter()
        .filter(|c| c.pieces.academy.is_some())
        .count() as u8
}

fn great_ideas(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Great Ideas",
        "You captured a city or won a land battle this turn: Gain 2 ideas.",
        ActionCost::free(),
        |_game, player, _| player.resources.ideas < player.resource_limit.ideas,
    )
    .combat_requirement(Arc::new(|s, p| {
        s.is_winner(p.index) && s.battleground.is_land()
    }))
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, player, _| {
            player.gain_resources(game, ResourcePile::ideas(2));
        },
    )
    .build()
}
