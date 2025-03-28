use crate::action_card::ActionCard;
use crate::content::tactics_cards::{for_the_people, TacticsCardFactory};
use crate::playing_actions::ActionType;

pub(crate) fn development_action_cards() -> Vec<ActionCard> {
    vec![mercenary(13, for_the_people)]
}

fn mercenary(id: u8, tactics_card: TacticsCardFactory) -> ActionCard {
    //todo
    ActionCard::builder(
        id,
        "Mercenary",
        "todo",
        ActionType::regular(),
        |_game, _player| true,
    )
    .tactics_card(tactics_card)
    .build()
}
