use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::card::{HandCard, HandCardLocation, HandCardType, hand_cards, log_card_transfer};
use crate::content::persistent_events::{HandCardsRequest, PlayerRequest};
use crate::content::tactics_cards::TacticsCardFactory;
use crate::events::EventPlayer;
use crate::game::Game;
use crate::objective_card::{deinit_objective_card, init_objective_card};
use crate::player::Player;
use crate::utils::remove_element;
use crate::wonder::Wonder;
use itertools::Itertools;
use std::fmt::Debug;

pub(crate) fn spy(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Spy",
        "Look at all Wonder, Action, and Objective cards of another player. \
        You may swap one card of the same type.",
        |c| c.action().culture_tokens(1),
        |game, player, _| !players_with_cards(game, player.index).is_empty(),
    )
    .add_player_request(
        |e| &mut e.play_action_card,
        1,
        |game, player, _| {
            Some(PlayerRequest::new(
                players_with_cards(game, player.index),
                "Select a player to look at all Wonder, Action, and Objective cards of",
            ))
        },
        |game, s, a| {
            let p = s.choice;
            s.log(
                game,
                &format!(
                    "Decided to looked at all Wonder, Action, and Objective cards of {}",
                    game.player_name(p)
                ),
            );
            a.selected_player = Some(p);
        },
    )
    .add_hand_card_request(
        |e| &mut e.play_action_card,
        0,
        |game, player, a| {
            game.information_revealed(); // you've seen the cards

            let p = player.get(game);
            let other = game.player(a.selected_player.expect("player not found"));

            let all = HandCardType::get_all();
            let mut cards = hand_cards(other, &all, game);
            for t in all {
                if !hand_cards(other, &[t], game).is_empty() {
                    cards.extend(hand_cards(p, &[t], game));
                }
            }

            let secrets = get_swap_secrets(other, game);
            player.get_mut(game).secrets.extend(secrets);

            Some(HandCardsRequest::new(
                cards,
                // 1 is not allowed, but the framework can't check that
                // not can the framework validate the types are correct
                0..=2,
                "Select a Wonder, Action, or Objective card to swap",
            ))
        },
        |game, s, a| {
            game.information_revealed(); // can't undo swap - the other player saw your card

            let _ = swap_spy_cards(
                game,
                &s.choice,
                &s.player(),
                a.selected_player.expect("player not found"),
            )
            .map_err(|e| panic!("Failed to swap cards: {e}"));
        },
    )
    .tactics_card(tactics_card)
    .build()
}

fn players_with_cards(game: &Game, player: usize) -> Vec<usize> {
    game.players
        .iter()
        .filter(|p| p.index != player && has_any_card(p, game))
        .map(|p| p.index)
        .collect_vec()
}

fn swap_spy_cards(
    game: &mut Game,
    swap: &[HandCard],
    player: &EventPlayer,
    other: usize,
) -> Result<(), String> {
    if swap.is_empty() {
        player.log(game, "Decided not to swap a card");
        return Ok(());
    }

    if swap.len() != 2 {
        return Err("must select 2 cards".to_string());
    }

    let p = player.get(game);
    let o = game.player(other);
    let our_card = get_swap_card(swap, p, game)?;
    let other_card = get_swap_card(swap, o, game)?;

    let t = match our_card {
        HandCard::ActionCard(id) => {
            let HandCard::ActionCard(other_id) = other_card else {
                return Err("wrong card type".to_string());
            };
            swap_card(
                game,
                player,
                other,
                id,
                other_id,
                |p| &mut p.action_cards,
                |_, _, _| {}, // action cards are not initialized
                |_, _, _| {},
                HandCard::ActionCard,
            );
            "action"
        }
        HandCard::Wonder(name) => {
            let HandCard::Wonder(other_name) = other_card else {
                return Err("wrong card type".to_string());
            };
            swap_card(
                game,
                player,
                other,
                name,
                other_name,
                |p| &mut p.wonder_cards,
                |_, _, _| {}, // wonder cards on the hand are not initialized
                |_, _, _| {},
                HandCard::Wonder,
            );
            "wonder"
        }
        HandCard::ObjectiveCard(id) => {
            let HandCard::ObjectiveCard(other_id) = other_card else {
                return Err("wrong card type".to_string());
            };
            swap_card(
                game,
                player,
                other,
                id,
                other_id,
                |p| &mut p.objective_cards,
                init_objective_card,
                deinit_objective_card,
                HandCard::ObjectiveCard,
            );
            "objective"
        }
    };
    player.log(
        game,
        &format!(
            "Decided to swap an {t} card with {}",
            game.player_name(other)
        ),
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn swap_card<T: PartialEq + Ord + Debug + Copy>(
    game: &mut Game,
    player: &EventPlayer,
    other: usize,
    id: T,
    other_id: T,
    get_list: impl Fn(&mut Player) -> &mut Vec<T>,
    init: impl Fn(&mut Game, usize, T),
    deinit: impl Fn(&mut Game, usize, T),
    to_hand_card: impl Fn(T) -> HandCard + Copy,
) {
    let card = remove_element(get_list(player.get_mut(game)), &id)
        .unwrap_or_else(|| panic!("card not found {id:?}"));
    let o = game.player_mut(other);
    let other_card = remove_element(get_list(o), &other_id)
        .unwrap_or_else(|| panic!("other card not found {other_id:?}"));

    get_list(o).push(card);
    get_list(o).sort();
    let p = player.get_mut(game);
    get_list(p).push(other_card);
    get_list(p).sort();

    deinit(game, player.index, id);
    deinit(game, other, other_id);
    init(game, other, id);
    init(game, player.index, other_id);
    log_card_transfer(
        game,
        &to_hand_card(id),
        HandCardLocation::Hand(player.index),
        HandCardLocation::Hand(other),
        &player.origin,
    );
    log_card_transfer(
        game,
        &to_hand_card(other_id),
        HandCardLocation::Hand(other),
        HandCardLocation::Hand(player.index),
        &player.origin,
    );
}

fn get_swap_card(swap: &[HandCard], p: &Player, game: &Game) -> Result<HandCard, String> {
    hand_cards(p, &HandCardType::get_all(), game)
        .iter()
        .find(|c| swap.contains(c))
        .ok_or("card not found".to_string())
        .cloned()
}

fn has_any_card(p: &Player, game: &Game) -> bool {
    !hand_cards(p, &HandCardType::get_all(), game).is_empty()
}

fn get_swap_secrets(other: &Player, game: &Game) -> Vec<String> {
    vec![
        format!(
            "{} has the following action cards: {}",
            other.get_name(),
            other
                .action_cards
                .iter()
                .map(|id| game.cache.get_action_card(*id).name())
                .join(", ")
        ),
        format!(
            "{} has the following objective cards: {}",
            other.get_name(),
            other
                .objective_cards
                .iter()
                .map(|id| game.cache.get_objective_card(*id).name())
                .join(", ")
        ),
        format!(
            "{} has the following wonder cards: {}",
            other.get_name(),
            other.wonder_cards.iter().map(Wonder::name).join(", ")
        ),
    ]
}

pub(crate) fn validate_spy_cards(cards: &[HandCard], game: &Game) -> Result<(), String> {
    if cards.is_empty() {
        return Ok(());
    }

    if cards.len() != 2 {
        return Err("must select 2 cards".to_string());
    }

    let our = hand_cards(
        game.player(game.current_event().player.index),
        &HandCardType::get_all(),
        game,
    )
    .into_iter()
    .filter(|c| cards.contains(c))
    .collect_vec();
    if our.len() != 1 {
        return Err("you must select exactly 1 card from your hand".to_string());
    }

    // don't use `hand_cards` here, because the cards are hidden (strip_secret)
    let their = cards
        .iter()
        .find(|c| !our.contains(c))
        .expect("card not found")
        .card_type();

    if our[0].card_type() != their {
        return Err("you must select a card of the same type".to_string());
    }

    Ok(())
}
