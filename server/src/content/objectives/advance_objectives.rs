use crate::content::advances;
use crate::content::objectives::city_objectives::leading_player;
use crate::objective_card::Objective;
use crate::player::Player;

fn advance_group_complete(objective: &str, group: &'static str) -> Objective {
    Objective::builder(objective, &format!("You have all {group} advances."))
        .status_phase_check(move |_game, player| all_advances_in_group(player, group))
        .build()
}

fn all_advances_in_group(player: &Player, group: &str) -> bool {
    advances::get_group(group)
        .advances
        .iter()
        .all(|a| player.has_advance(&a.name))
}

pub(crate) fn city_planner() -> Objective {
    advance_group_complete("City Planner", "Construction")
}

pub(crate) fn education_lead() -> Objective {
    advance_group_complete("Education Lead", "Education")
}

pub(crate) fn militarized() -> Objective {
    advance_group_complete("Militarized", "Warfare")
}

pub(crate) fn culture_focus() -> Objective {
    advance_group_complete("Culture Focus", "Culture")
}

pub(crate) fn science_focus() -> Objective {
    advance_group_complete("Science Focus", "Science")
}

pub(crate) fn trade_focus() -> Objective {
    advance_group_complete("Trade Focus", "Economy")
}

pub(crate) fn seafarers() -> Objective {
    advance_group_complete("Seafarers", "Seafaring")
}

pub(crate) fn government() -> Objective {
    Objective::builder(
        "Government",
        "You have all advances in one government type.",
    )
    .status_phase_check(|_game, player| {
        advances::get_governments()
            .iter()
            .any(|g| all_advances_in_group(player, &g.name))
    })
    .build()
}

pub(crate) fn goal_focused() -> Objective {
    Objective::builder(
        "Goal Focused",
        "You have more complete advance groups than any other player.",
    )
    .status_phase_check(|game, player| {
        leading_player(game, player, 1, |p, _| {
            advances::get_groups()
                .iter()
                .filter(|g| g.advances.iter().all(|a| p.has_advance(&a.name)))
                .count()
        })
    })
    .build()
}

pub(crate) fn diversified_research() -> Objective {
    Objective::builder(
        "Diversified Research",
        "You have at least 1 advance in 9 different advance groups.",
    )
    .status_phase_check(|_game, player| {
        advances::get_groups()
            .iter()
            .filter(|g| g.advances.iter().any(|a| player.has_advance(&a.name)))
            .count()
            >= 9
    })
    .build()
}
