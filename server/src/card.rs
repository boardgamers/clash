use crate::content::action_cards::spy::validate_spy_cards;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::objective_card::match_objective_cards;
use crate::player::Player;
use crate::utils::Shuffle;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use crate::content::persistent_events::PersistentEventType;

#[derive(Clone, Copy)]
pub enum HandCardType {
    Action,
    Wonder,
}

impl HandCardType {
    #[must_use]
    pub fn get_all() -> Vec<HandCardType> {
        vec![HandCardType::Action, HandCardType::Wonder]
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Ord, Debug, PartialOrd)]
pub enum HandCard {
    ActionCard(u8),
    ObjectiveCard(u8),
    Wonder(String),
}

pub(crate) fn draw_card_from_pile<T>(
    game: &mut Game,
    name: &str,
    leave_card: bool,
    get_pile: impl Fn(&mut Game) -> &mut Vec<T>,
    reshuffle_pile: impl Fn() -> Vec<T>,
    get_owned: impl Fn(&Player) -> Vec<T>,
) -> Option<T>
where
    T: Clone + PartialEq,
{
    if get_pile(game).is_empty() {
        let mut new_pile = reshuffle_pile();
        for p in &game.players {
            let owned = get_owned(p);
            new_pile.retain(|c| !owned.contains(c));
        }

        if !new_pile.is_empty() {
            game.add_info_log_item(&format!("Reshuffling {name} pile"));
            *get_pile(game) = new_pile.shuffled(&mut game.rng);
        }
    }

    if get_pile(game).is_empty() {
        game.add_info_log_item(&format!("No {name} left to draw"));
        return None;
    }

    if game.age > 0 {
        game.lock_undo(); // new information is revealed
    }

    if leave_card {
        get_pile(game).first().cloned()
    } else {
        Some(get_pile(game).remove(0))
    }
}

#[must_use]
pub fn hand_cards(player: &Player, types: &[HandCardType]) -> Vec<HandCard> {
    types
        .iter()
        .flat_map(|t| match t {
            HandCardType::Action => player
                .action_cards
                .iter()
                .map(|&id| HandCard::ActionCard(id))
                .collect_vec(),
            HandCardType::Wonder => player
                .wonder_cards
                .iter()
                .map(|n| HandCard::Wonder(n.clone()))
                .collect(),
        })
        .collect()
}

///
/// Validates the selection of cards in the hand.
///
/// # Errors
///
/// If the selection is invalid, an error message is returned.
pub fn validate_card_selection(cards: &[HandCard], game: &Game) -> Result<(), String> {
    let s = game.current_event();
    let player = &s.player;
    let Some(h) = player.handler.as_ref() else {
        return Err("no selection handler".to_string());
    };
    match &h.origin {
        EventOrigin::CivilCard(id) if *id == 7 || *id == 8 => validate_spy_cards(cards, game),
        EventOrigin::Builtin(b) if b == "Select Objective Cards to Complete" => {
            let PersistentEventType::SelectObjectives(c) = &s.event_type else {
                return Err("no selection handler".to_string());
            };
            
            match_objective_cards(cards, &c.objective_opportunities)
                .map(|_| ())
        }
        _ => Ok(()),
    }
}
