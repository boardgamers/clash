use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::card::{HandCard, HandCardType, hand_cards};
use crate::content::action_cards::get_action_card;
use crate::content::persistent_events::{HandCardsRequest, PersistentEventType, PlayerRequest};
use crate::content::tactics_cards::TacticsCardFactory;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::player::Player;
use crate::playing_actions::ActionType;
use crate::resource_pile::ResourcePile;
use crate::utils::remove_element;
use itertools::Itertools;

pub(crate) fn spy(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    ActionCard::builder(
        id,
        "Spy",
        "Look at all Wonder, Action, and Objective cards of another player. \
        You may swap one card of the same type.",
        ActionType::regular_with_cost(ResourcePile::culture_tokens(1)),
        |game, player, _| !players_with_cards(game, player.index).is_empty(),
    )
    .add_player_request(
        |e| &mut e.play_action_card,
        1,
        |game, player, _| {
            Some(PlayerRequest::new(
                players_with_cards(game, player),
                "Select a player to look at all Wonder, Action, and Objective cards of",
            ))
        },
        |game, s, a| {
            let p = s.choice;
            game.add_info_log_item(&format!(
                "{} decided to looked at all Wonder, Action, and Objective cards of {}",
                s.player_name,
                game.player_name(p)
            ));
            a.selected_player = Some(p);
        },
    )
    .add_hand_card_request(
        |e| &mut e.play_action_card,
        0,
        |game, player, a| {
            game.lock_undo(); // you've seen the cards

            let p = game.player(player);
            let other = game.player(a.selected_player.expect("player not found"));

            let all = HandCardType::get_all();
            let mut cards = hand_cards(other, &all);
            for t in all {
                if !hand_cards(other, &[t]).is_empty() {
                    cards.extend(hand_cards(p, &[t]));
                }
            }

            let secrets = get_swap_secrets(other);
            game.player_mut(player).secrets.extend(secrets);

            Some(HandCardsRequest::new(
                cards,
                // 1 is not allowed, but the framework can't check that
                // not can the framework validate the types are correct
                0..=2,
                "Select a Wonder, Action, or Objective card to swap",
            ))
        },
        |game, s, a| {
            game.lock_undo(); // can't undo swap - the other player saw your card

            let _ = swap_cards(
                game,
                &s.choice,
                s.player_index,
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
        .filter(|p| p.index != player && has_any_card(p))
        .map(|p| p.index)
        .collect_vec()
}

fn swap_cards(
    game: &mut Game,
    swap: &[HandCard],
    player: usize,
    other: usize,
) -> Result<(), String> {
    if swap.is_empty() {
        game.add_info_log_item(&format!(
            "{} decided not to swap a card",
            game.player_name(other)
        ));
        return Ok(());
    }

    if swap.len() != 2 {
        return Err("must select 2 cards".to_string());
    }

    let p = game.player(player);
    let o = game.player(other);
    let our_card = get_swap_card(swap, p);
    let other_card = get_swap_card(swap, o);

    let t = match our_card {
        HandCard::ActionCard(id) => {
            let HandCard::ActionCard(other_id) = other_card else {
                return Err("wrong card type".to_string());
            };
            swap_card(game, player, other, &id, &other_id, |p| &mut p.action_cards);
            "action"
        }
        HandCard::Wonder(name) => {
            let HandCard::Wonder(other_name) = other_card else {
                return Err("wrong card type".to_string());
            };
            swap_card(game, player, other, &name, &other_name, |p| {
                &mut p.wonder_cards
            });
            "wonder"
        }
        HandCard::ObjectiveCard(id) => {
            let HandCard::ObjectiveCard(other_id) = other_card else {
                return Err("wrong card type".to_string());
            };
            swap_card(game, player, other, &id, &other_id, |p| {
                &mut p.objective_cards
            });
            "objective"
        }
    };
    game.add_info_log_item(&format!(
        "{} decided to swap an {t} card with {}",
        game.player_name(player),
        game.player_name(other)
    ));

    Ok(())
}

fn swap_card<T: PartialEq + Ord>(
    game: &mut Game,
    player: usize,
    other: usize,
    id: &T,
    other_id: &T,
    get_list: impl Fn(&mut Player) -> &mut Vec<T>,
) {
    let card = remove_element(get_list(game.player_mut(player)), id).expect("card not found");
    let o = game.player_mut(other);
    let other_card = remove_element(get_list(o), other_id).expect("card not found");

    get_list(o).push(card);
    get_list(o).sort();
    let p = game.player_mut(player);
    get_list(p).push(other_card);
    get_list(p).sort();
}

fn get_swap_card(swap: &[HandCard], p: &Player) -> HandCard {
    hand_cards(p, &HandCardType::get_all())
        .into_iter()
        .find(|c| swap.contains(c))
        .expect("card not found")
        .clone()
}

fn has_any_card(p: &Player) -> bool {
    !hand_cards(p, &HandCardType::get_all()).is_empty()
}

fn get_swap_secrets(other: &Player) -> Vec<String> {
    vec![
        format!(
            "{} has the following action cards: {}",
            other.get_name(),
            other
                .action_cards
                .iter()
                .map(|id| {
                    let a = get_action_card(*id);
                    format!(
                        "{}/{}",
                        a.civil_card.name,
                        a.tactics_card.map_or("-".to_string(), |c| c.name.clone())
                    )
                })
                .join(", ")
        ),
        format!(
            "{} has the following wonder cards: {}",
            other.get_name(),
            other
                .wonder_cards
                .iter()
                .map(std::string::ToString::to_string)
                .join(", ")
        ),
    ]
}

/// # Panics
///
/// Panics if the game is in an invalid state
#[must_use]
pub fn validate_if_spy(cards: &[HandCard], game: &Game) -> bool {
    let s = game.current_event();
    let h = &s.player.handler.as_ref().expect("handler not found");
    match h.origin {
        EventOrigin::CivilCard(id) if id == 7 || id == 8 => {
            let mut g = game.clone();
            let PersistentEventType::ActionCard(c) = &s.event_type else {
                panic!("wrong event type");
            };

            swap_cards(
                &mut g,
                cards,
                s.player.index,
                c.selected_player.expect("no player found"),
            )
            .is_ok()
        }
        _ => true,
    }
}
