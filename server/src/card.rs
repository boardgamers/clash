use crate::content::action_cards::spy::validate_spy_cards;
use crate::content::action_cards::synergies::validate_new_plans;
use crate::content::civilizations::rome::validate_princeps_cards;
use crate::content::persistent_events::PersistentEventType;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::objective_card::match_objective_cards;
use crate::player::Player;
use crate::utils::Shuffle;
use crate::wonder::Wonder;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub enum HandCardType {
    Action,
    Objective,
    Wonder,
}

impl HandCardType {
    #[must_use]
    pub fn get_all() -> Vec<HandCardType> {
        vec![
            HandCardType::Action,
            HandCardType::Objective,
            HandCardType::Wonder,
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Ord, Debug, PartialOrd)]
pub enum HandCard {
    ActionCard(u8),
    ObjectiveCard(u8),
    Wonder(Wonder),
}

pub(crate) fn draw_card_from_pile<T>(
    game: &mut Game,
    name: &str,
    get_pile: impl Fn(&mut Game) -> &mut Vec<T>,
    reshuffle_pile: impl Fn(&Game) -> Vec<T>,
    get_owned: impl Fn(&Player) -> Vec<T>,
) -> Option<T>
where
    T: Clone + PartialEq,
{
    if get_pile(game).is_empty() {
        let mut new_pile = reshuffle_pile(game);
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
        game.information_revealed();
    }

    Some(get_pile(game).remove(0))
}

pub(crate) fn discard_card(
    discard: impl Fn(&mut Game) -> &mut Vec<u8>,
    card: u8,
    player: usize,
    game: &mut Game,
) {
    if game
        .player(player)
        .wonders_owned
        .contains(Wonder::GreatMausoleum)
    {
        discard(game).insert(0, card);
    } else {
        discard(game).push(card);
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
            HandCardType::Objective => player
                .objective_cards
                .iter()
                .map(|&id| HandCard::ObjectiveCard(id))
                .collect_vec(),
            HandCardType::Wonder => player
                .wonder_cards
                .iter()
                .map(|n| HandCard::Wonder(*n))
                .collect(),
        })
        .collect()
}

///
/// Validates the selection of cards in the hand.
///
/// # Returns
///
/// Card names to show in the UI - if possible.
///
/// # Errors
///
/// If the selection is invalid, an error message is returned.
pub fn validate_card_selection(
    cards: &[HandCard],
    game: &Game,
) -> Result<Vec<(u8, String)>, String> {
    let Some(h) = &game.current_event().player.handler.as_ref() else {
        return Err("no selection handler".to_string());
    };
    validate_card_selection_for_origin(cards, game, &h.origin)
}

pub(crate) fn validate_card_selection_for_origin(
    cards: &[HandCard],
    game: &Game,
    o: &EventOrigin,
) -> Result<Vec<(u8, String)>, String> {
    match o {
        EventOrigin::CivilCard(id) if *id == 7 || *id == 8 => {
            validate_spy_cards(cards, game).map(|()| Vec::new())
        }
        EventOrigin::CivilCard(id) if *id == 31 || *id == 32 => {
            validate_new_plans(cards, game).map(|()| Vec::new())
        }
        EventOrigin::Ability(b) if b == "Select Objective Cards to Complete" => {
            let PersistentEventType::SelectObjectives(c) = &game.current_event().event_type else {
                return Err("no selection handler".to_string());
            };

            match_objective_cards(cards, &c.objective_opportunities, game)
        }
        EventOrigin::Ability(b) if b == "Princeps" => {
            validate_princeps_cards(cards).map(|()| Vec::new())
        }
        _ => Ok(Vec::new()),
    }
}

pub(crate) fn all_action_hand_cards(p: &Player) -> Vec<HandCard> {
    p.action_cards
        .iter()
        .map(|a| HandCard::ActionCard(*a))
        .collect_vec()
}

pub(crate) fn all_objective_hand_cards(player: &Player) -> Vec<HandCard> {
    player
        .objective_cards
        .iter()
        .map(|&a| HandCard::ObjectiveCard(a))
        .collect_vec()
}
