//! Read-only research-source metadata. The caller must first check the source
//! action's actual availability (turn, hand/leader ownership, costs and limits).
//! Keep target mappings in engine identifiers, never translated descriptions.
use crate::advance::Advance;
use crate::content::advances::AdvanceGroup;
use crate::content::custom_actions::CustomActionType;
use crate::game::Game;
use crate::player::{CostTrigger, Player};

fn in_groups(game: &Game, player: &Player, groups: &[AdvanceGroup]) -> Vec<Advance> {
    groups
        .iter()
        .flat_map(|group| &game.cache.get_advance_group(*group).advances)
        .filter(|a| player.can_advance_free(a.advance, game))
        .map(|a| a.advance)
        .collect()
}

pub(super) fn card_choices(game: &Game, seat: usize, id: u8) -> Vec<Advance> {
    use AdvanceGroup::{
        Autocracy, Construction, Culture, Democracy, Economy, Education, Science, Seafaring,
        Spirituality, Theocracy, Warfare,
    };
    // The same groups used by incidents::great_persons::great_person_card.
    // Playability alone is insufficient: a Great Person's secondary effect may
    // remain playable after all its advances have already been researched.
    let groups: &[AdvanceGroup] = match id {
        118 => &[Seafaring],
        119 | 156 => &[Culture],
        120 => &[Spirituality],
        121 => &[Education],
        122 => &[Science],
        123 => &[Democracy, Autocracy, Theocracy],
        124 => &[Warfare],
        125 => &[Economy],
        126 => &[Construction],
        _ => {
            return crate::content::action_cards::research_choices(
                game,
                game.player(seat),
                &game.cache.get_action_card(id).civil_card.name,
            );
        }
    };
    in_groups(game, game.player(seat), groups)
}

pub(super) fn card_is_free_research(id: u8) -> bool {
    matches!(id, 1..=4 | 39 | 40 | 118..=126 | 156)
}

pub(super) fn ability_choices(game: &Game, seat: usize, action: CustomActionType) -> Vec<Advance> {
    use AdvanceGroup::{Agriculture, Construction, Education, Warfare};
    use CustomActionType::{
        AgricultureEconomist, Aqueduct, ArtOfWar, Constructor, GoldenAge, Master,
    };
    let player = game.player(seat);
    match action {
        // Same targets as leader_ability::advance_gain_custom_action. The
        // action availability check already verifies the happy leader city.
        ArtOfWar => in_groups(game, player, &[Warfare]),
        AgricultureEconomist => in_groups(game, player, &[Agriculture]),
        Master => in_groups(game, player, &[Education]),
        Constructor => in_groups(game, player, &[Construction]),
        Aqueduct => vec![Advance::Sanitation],
        GoldenAge => {
            // Match india::golden_age_advances before its upfront payment:
            // reserve the culture token, then test actual research payments.
            let mut available = player.resources.clone();
            if available.culture_tokens == 0 {
                return vec![];
            }
            available.culture_tokens -= 1;
            game.cache
                .get_advance_groups()
                .iter()
                .filter(|group| !group.advances.iter().any(|a| player.has_advance(a.advance)))
                .flat_map(|group| &group.advances)
                .filter(|a| {
                    player.can_advance_free(a.advance, game)
                        && player
                            .advance_cost(a.advance, game, CostTrigger::NoModifiers)
                            .cost
                            .first_valid_payment(&available)
                            .is_some()
                })
                .map(|a| a.advance)
                .collect()
        }
        // Triggered rewards and borrowed abilities are not research actions.
        _ => vec![],
    }
}

pub(super) fn ability_is_free_research(action: CustomActionType) -> bool {
    matches!(
        action,
        CustomActionType::ArtOfWar
            | CustomActionType::AgricultureEconomist
            | CustomActionType::Master
            | CustomActionType::Constructor
    )
}
