use crate::game::Game;
use crate::player::Player;
use crate::utils::Shuffle;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum HandCard {
    ActionCard(u8),
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

    game.lock_undo(); // new information is revealed

    if leave_card {
        get_pile(game).first().cloned()
    } else {
        Some(get_pile(game).remove(0))
    }
}
