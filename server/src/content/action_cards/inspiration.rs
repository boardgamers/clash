use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{ActionCard, ActionCardBuilder};
use crate::advance::gain_advance_without_payment;
use crate::city::MoodState;
use crate::content::action_cards::spy::spy;
use crate::content::action_cards::synergies::teachable_advances;
use crate::content::advances;
use crate::content::persistent_events::{AdvanceRequest, PaymentRequest, PositionRequest};
use crate::content::tactics_cards::{
    TacticsCardFactory, encircled, heavy_resistance, high_ground, high_morale, peltasts, siege,
    surprise, wedge_formation,
};
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::playing_actions::ActionCost;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
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
        ActionCost::free(),
        |_game, player, _| {
            player.resources.culture_tokens >= 1 && !possible_advances(player).is_empty()
        },
    )
    .tactics_card(tactics_card)
    .add_advance_request(
        |e| &mut e.play_action_card,
        0,
        |game, player, _| Some(AdvanceRequest::new(possible_advances(game.player(player)))),
        |game, sel, _| {
            let advance = sel.choice.clone();
            gain_advance_without_payment(
                game,
                &advance,
                sel.player_index,
                ResourcePile::culture_tokens(1),
                false,
            );
            let name = &sel.player_name;
            game.add_info_log_item(&format!(
                "{name} gained {advance} for 1 culture token using the Advance action card.",
            ));
        },
    )
    .build()
}

fn possible_advances(player: &Player) -> Vec<String> {
    advances::get_all()
        .iter()
        .filter(|a| player.can_advance_free(a))
        .map(|a| a.name.clone())
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
        |game, player, _| {
            Some(AdvanceRequest::new(possible_inspiration_advances(
                game,
                game.player(player),
            )))
        },
        |game, sel, _| {
            let advance = sel.choice.clone();
            gain_advance_without_payment(
                game,
                &advance,
                sel.player_index,
                ResourcePile::empty(),
                false,
            );
            let name = &sel.player_name;
            game.add_info_log_item(&format!(
                "{name} gained {advance} for free using Inspiration.",
            ));
        },
    )
    .build()
}

pub(crate) fn possible_inspiration_advances(game: &Game, player: &Player) -> Vec<String> {
    let players = players_in_range2(game, player)
        .iter()
        .map(|&i| game.player(i))
        .collect_vec();

    players
        .iter()
        .flat_map(|p| teachable_advances(p, player))
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
    .requirement_land_battle_won()
    .tactics_card(tactics_card);

    b = increase_mood(b, 2, false);
    b = b.add_payment_request_listener(
        |e| &mut e.play_action_card,
        1,
        |game, player, _| {
            if cities_where_mood_can_increase(game.player(player)).is_empty() {
                return None;
            }

            Some(vec![PaymentRequest::new(
                PaymentOptions::resources(ResourcePile::mood_tokens(1)),
                "Pay 1 mood token to increase the mood in a city by 1",
                true,
            )])
        },
        |game, s, a| {
            if s.choice[0].is_empty() {
                game.add_info_log_item(&format!("{} did not pay 1 mood token", s.player_name));
            } else {
                game.add_info_log_item(&format!("{} paid 1 mood token", s.player_name));
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
        move |game, player, a| {
            if need_payment && a.answer.is_none() {
                return None;
            }
            let choices = cities_where_mood_can_increase(game.player(player));
            let needed = 1..=1;
            Some(PositionRequest::new(
                choices,
                needed,
                "Select a city to increase the mood by 1",
            ))
        },
        |game, s, _| {
            let pos = s.choice[0];
            let player = s.player_index;
            game.add_info_log_item(&format!(
                "{} selected city {} to increase the mood by 1",
                s.player_name, pos
            ));
            game.player_mut(player)
                .get_city_mut(pos)
                .increase_mood_state();
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
        |game, player, name, _| {
            let p = game.player_mut(player);
            let pile = ResourcePile::ideas(academies(p));
            p.gain_resources(pile.clone());
            game.add_info_log_item(&format!("{name} gained {pile} (1 for each Academy)"));
        },
    )
    .build()
}

fn academies(player: &Player) -> u32 {
    player
        .cities
        .iter()
        .filter(|c| c.pieces.academy.is_some())
        .count() as u32
}

fn great_ideas(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Great Ideas",
        "After capturing a city or winning a land battle: Gain 2 ideas.",
        ActionCost::free(),
        |_game, player, _| player.resources.ideas < player.resource_limit.ideas,
    )
    .requirement_land_battle_won()
    .tactics_card(tactics_card)
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, player, name, _| {
            let p = game.player_mut(player);
            let pile = ResourcePile::ideas(2);
            p.gain_resources(pile.clone());
            game.add_info_log_item(&format!("{name} gained {pile} for Great Ideas"));
        },
    )
    .build()
}
