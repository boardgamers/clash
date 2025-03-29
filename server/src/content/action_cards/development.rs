use crate::action_card::ActionCard;
use crate::content::action_cards::mercenaries::mercenaries;
use crate::content::tactics_cards::{for_the_people, heavy_resistance};

pub(crate) fn development_action_cards() -> Vec<ActionCard> {
    vec![
        mercenaries(13, for_the_people),
        mercenaries(14, heavy_resistance),
    ]
}
