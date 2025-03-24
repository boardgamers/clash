use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::card::{hand_cards, HandCard, HandCardType};
use crate::content::action_cards::get_action_card;
use crate::content::custom_phase_actions::{HandCardsRequest, PlayerRequest};
use crate::game::Game;
use crate::player::Player;
use crate::playing_actions::ActionType;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::TacticsCard;
use itertools::Itertools;
use crate::utils::remove_element;

pub(crate) fn spy(id: u8, tactics_card: TacticsCard) -> ActionCard {
    ActionCard::builder(
        id,
        "Spy",
        "Look at all Wonder, Action, and Objective cards of another player. \
        You may swap one card of the same type.",
        ActionType::regular_with_cost(ResourcePile::culture_tokens(1)),
        |_game, player| has_any_card(player),
    )
    .add_player_request(
        |e| &mut e.on_play_action_card,
        1,
        |game, player, _| {
            Some(PlayerRequest::new(
                game.players
                    .iter()
                    .filter(|p| p.index != player && has_any_card(p))
                    .map(|p| p.index)
                    .collect_vec(),
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
        |e| &mut e.on_play_action_card,
        0,
        |game, player, a| {
            let p = game.get_player(player);
            let other = game.get_player(a.selected_player.expect("player not found"));

            let all = HandCardType::get_all();
            let mut cards = hand_cards(other, &all);
            for t in all {
                if !hand_cards(other, &[t]).is_empty() {
                    cards.extend(hand_cards(p, &[t]));
                }
            }

            let secrets = get_swap_secrets(other);
            game.get_player_mut(player).secrets.extend(secrets);

            Some(HandCardsRequest::new(
                cards,
                // 1 is not allowed, but the framework can't check that
                // not can the framework validate the types are correct
                0..=2,
                "Select a Wonder, Action, or Objective card to swap",
            ))
        },
        |game, s, a| {
            let swap = &s.choice;
            if swap.is_empty() {
                game.add_info_log_item(&format!("{} decided not to swap a card", s.player_name));
                return;
            }

            swap_cards(game, swap, s.player_index, a.selected_player.expect("player not found"));
        },
    )
    .with_tactics_card(tactics_card)
    .build()
}

fn swap_cards(game: &mut Game, swap: &Vec<HandCard>, player: usize, other: usize) {
    assert_eq!(swap.len(), 2, "must select 2 cards");
    let p = game.get_player(player);
    let o = game.get_player(other);
    let our_card = get_swap_card(swap, p);
    let other_card = get_swap_card(swap, o);
    
    match our_card {
        HandCard::ActionCard(id) => {
            game.add_info_log_item(&format!(
                "{} decided to swap an action card with {}",
                p.get_name(),
                o.get_name()
            ));
            swap_action_card(game, player, other, id, other_card);
        }
        HandCard::Wonder(n) => {
            game.add_info_log_item(&format!(
                "{} decided to swap a wonder card with {}",
                p.get_name(),
                o.get_name()
            ));
            // swap_wonder_card(game, player, other, n, other_card);
        }
    }

    // let other = game.get_player(sel.player_index.selected_player.unwrap());
    // let player = sel.player_index.player_index;
    // let card = sel.choice[0].clone();
    // game.add_info_log_item(&format!(
    //     "{} decided to swap a card with {}",
    //     sel.player_name,
    //     game.player_name(other.index)
    // ));
    // swap_card(game, player, other.index, &card);
}

fn swap_action_card(
    game: &mut Game,
    player: usize,
    other: usize,
    id: u8,
    other_card: HandCard,
) {
    let p = game.get_player_mut(player);
    let card =
        remove_element(&mut p.action_cards, &id).expect("card not found");
    let HandCard::ActionCard(other_id) = other_card else {
        panic!("wrong card type");
    };
    let o = game.get_player_mut(other);
    let other_card = remove_element(&mut o.action_cards, &other_id).expect("card not found");
    o.action_cards.push(card);
    game.get_player_mut(p).action_cards.push(other_card);
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
            other.wonder_cards.iter().map(|n| n.to_string()).join(", ")
        ),
    ]
}
