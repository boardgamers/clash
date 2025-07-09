use crate::content::action_cards::spy::validate_spy_cards;
use crate::content::action_cards::synergies::validate_new_plans;
use crate::content::civilizations::rome::validate_princeps_cards;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::log::{ActionLogEntry, add_action_log_item};
use crate::player::Player;
use crate::utils::Shuffle;
use crate::wonder::Wonder;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Copy, Eq, PartialEq)]
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

impl Display for HandCardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandCardType::Action => write!(f, "an action card"),
            HandCardType::Objective => write!(f, "an objective card"),
            HandCardType::Wonder => write!(f, "a wonder card"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Ord, Debug, PartialOrd)]
pub enum HandCard {
    ActionCard(u8),
    ObjectiveCard(u8),
    Wonder(Wonder),
}

impl HandCard {
    #[must_use]
    pub fn card_type(&self) -> HandCardType {
        match self {
            HandCard::ActionCard(_) => HandCardType::Action,
            HandCard::ObjectiveCard(_) => HandCardType::Objective,
            HandCard::Wonder(_) => HandCardType::Wonder,
        }
    }

    #[must_use]
    pub fn name(&self, game: &Game) -> String {
        match self {
            HandCard::ActionCard(id) => game.cache.get_action_card(*id).name(),
            HandCard::ObjectiveCard(id) => game.cache.get_objective_card(*id).name(),
            HandCard::Wonder(wonder) => wonder.name(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Ord, Debug, PartialOrd)]
pub enum HandCardLocation {
    DrawPile,
    DrawPilePeeked(usize),
    Hand(usize),
    RevealedHand(usize),
    DiscardPile,
    // converted from incident to hand
    Incident,
    PlayToDiscard,
    // the tactics card counts as being discarded even though it is discarded at the end of combat
    PlayToDiscardFaceDown,
    PlayToKeep,
    CompleteObjective(String),
    Public,
    GreatSeer(usize),
}

impl HandCardLocation {
    #[must_use]
    pub fn player(&self) -> Option<usize> {
        match self {
            HandCardLocation::Hand(p) => Some(*p),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_public(&self) -> bool {
        !matches!(
            self,
            HandCardLocation::DrawPile
                | HandCardLocation::DrawPilePeeked(_)
                | HandCardLocation::Hand(_)
                | HandCardLocation::PlayToDiscardFaceDown
                | HandCardLocation::GreatSeer(_)
        )
    }
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

pub(crate) fn log_card_transfer(
    game: &mut Game,
    card: &HandCard,
    from: HandCardLocation,
    to: HandCardLocation,
    origin: &EventOrigin,
) {
    let (player_index, message): (usize, &str) = if let HandCardLocation::Hand(p) = to {
        let name = card_name(card, &from, game);
        (
            p,
            match from {
                HandCardLocation::DrawPile => &format!("Draw {name}"),
                HandCardLocation::Hand(from) | HandCardLocation::RevealedHand(from) => {
                    &format!("Gain {name} from {}", game.player_name(from))
                }
                HandCardLocation::DiscardPile => &format!("Gain {name} from discard pile"),
                HandCardLocation::Public => &format!("Gain {name} from the public area"),
                HandCardLocation::GreatSeer(_) => &format!("Gain {name} from Great Seer"),
                HandCardLocation::Incident => &format!("Gain {name} from the current event"),
                _ => {
                    panic!(
                        "Cannot transfer card from played to hand: {card:?} from {from:?} to {to:?}"
                    )
                }
            },
        )
    } else if let HandCardLocation::Hand(p) = from {
        let name = card_name(card, &to, game);
        (
            p,
            match to {
                HandCardLocation::DiscardPile => &format!("Discard {name}"),
                HandCardLocation::DrawPile => &format!("Shuffle {name} back into the draw pile"),
                HandCardLocation::PlayToDiscard | HandCardLocation::PlayToKeep => {
                    &format!("Play {name}")
                }
                HandCardLocation::CompleteObjective(ref o) => &format!("Complete {o} using {name}"),
                HandCardLocation::PlayToDiscardFaceDown => &format!("Play {name} face down"),
                HandCardLocation::Public => &format!("Place {name} in public area"),
                _ => panic!(
                    "Cannot transfer card from hand to draw pile: {card:?} from {from:?} to {to:?}"
                ),
            },
        )
    } else if let HandCardLocation::GreatSeer(p) = to {
        assert_eq!(from, HandCardLocation::DrawPile);
        (p, "Placed a card from the draw pile in the Great Seer")
    } else if let HandCardLocation::DrawPilePeeked(p) = from {
        assert_eq!(to, HandCardLocation::DrawPile);
        (
            p,
            "Reshuffled a card from the draw pile back into the draw pile",
        )
    } else {
        panic!("Invalid card transfer from {from:?} to {to:?}");
    };

    game.log(player_index, origin, message);
    add_action_log_item(
        game,
        player_index,
        ActionLogEntry::hand_card(card.clone(), from, to),
        origin.clone(),
        vec![],
    );
}

fn card_name(card: &HandCard, l: &HandCardLocation, game: &mut Game) -> String {
    if l.is_public() {
        card.name(game)
    } else {
        card.card_type().to_string()
    }
}
