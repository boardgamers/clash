use crate::action_card::ActionCard;
use crate::content::action_cards::mercenary::mercenary;
use crate::content::tactics_cards::for_the_people;

pub(crate) fn development_action_cards() -> Vec<ActionCard> {
    vec![mercenary(13, for_the_people)]
}
