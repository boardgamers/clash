pub(crate) mod cultural_takeover;
pub(crate) mod development;
mod inspiration;
mod mercenaries;
pub(crate) mod negotiation;
pub(crate) mod spy;
pub(crate) mod synergies;

use crate::action_card::ActionCard;
use crate::content::action_cards::inspiration::inspiration_action_cards;
use crate::content::action_cards::negotiation::negotiation_action_cards;
use crate::content::action_cards::synergies::synergies_action_cards;
use development::development_action_cards;
use itertools::Itertools;

#[must_use]
pub(crate) fn get_all_uncached() -> Vec<ActionCard> {
    let all = vec![
        inspiration_action_cards(),
        development_action_cards(),
        negotiation_action_cards(),
        synergies_action_cards(),
    ]
    .into_iter()
    .flatten()
    .collect_vec();
    assert_eq!(
        all.iter().unique_by(|i| i.id).count(),
        all.len(),
        "action card ids are not unique"
    );
    all
}

// Reuse each card's actual eligibility rules for read-only research reminders.
pub(crate) fn research_choices(game: &crate::game::Game, player: &crate::player::Player, name: &str) -> Vec<crate::advance::Advance> {
    let mut choices = match name {
        "Quick Advance" => inspiration::possible_advances(player, game),
        "Inspiration" | "Technology Trade" => inspiration::possible_inspiration_advances(game, player),
        "Synergies" => synergies::categories_with_2_affordable_advances(player, game),
        "New Ideas" => synergies::advances_that_can_be_gained(player, game),
        _ => vec![],
    };
    choices.sort();
    choices.dedup();
    choices
}
