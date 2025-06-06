use crate::content::advances::AdvanceGroup;
use crate::content::objectives::city_objectives::leading_player;
use crate::game::Game;
use crate::objective_card::Objective;
use crate::player::Player;

fn advance_group_complete(objective: &str, group: AdvanceGroup) -> Objective {
    let desc = format!("You have all {group} advances.");
    Objective::builder(objective, &desc)
        .status_phase_check(move |game, player| all_advances_in_group(player, group, game))
        .build()
}

fn all_advances_in_group(player: &Player, group: AdvanceGroup, game: &Game) -> bool {
    game.cache
        .get_advance_group(group)
        .advances
        .iter()
        .all(|a| player.has_advance(a.advance))
}

pub(crate) fn city_planner() -> Objective {
    advance_group_complete("City Planner", AdvanceGroup::Construction)
}

pub(crate) fn education_lead() -> Objective {
    advance_group_complete("Education Lead", AdvanceGroup::Education)
}

pub(crate) fn militarized() -> Objective {
    advance_group_complete("Militarized", AdvanceGroup::Warfare)
}

pub(crate) fn culture_focus() -> Objective {
    advance_group_complete("Culture Focus", AdvanceGroup::Culture)
}

pub(crate) fn science_focus() -> Objective {
    advance_group_complete("Science Focus", AdvanceGroup::Science)
}

pub(crate) fn trade_focus() -> Objective {
    advance_group_complete("Trade Focus", AdvanceGroup::Economy)
}

pub(crate) fn seafarers() -> Objective {
    advance_group_complete("Seafarers", AdvanceGroup::Seafaring)
}

pub(crate) fn government() -> Objective {
    Objective::builder(
        "Government",
        "You have all advances in one government type.",
    )
    .status_phase_check(|game, player| {
        game.cache
            .get_governments()
            .iter()
            .any(|g| all_advances_in_group(player, g.advance_group, game))
    })
    .build()
}

pub(crate) fn goal_focused() -> Objective {
    Objective::builder(
        "Goal Focused",
        "You have more complete advance groups than any other player.",
    )
    .status_phase_check(|game, player| {
        leading_player(game, player, 1, |p, g| {
            g.cache
                .get_advance_groups()
                .iter()
                .filter(|g| g.advances.iter().all(|a| p.has_advance(a.advance)))
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
    .status_phase_check(|game, player| {
        game.cache
            .get_advance_groups()
            .iter()
            .filter(|g| g.advances.iter().any(|a| player.has_advance(a.advance)))
            .count()
            >= 9
    })
    .build()
}

pub(crate) fn high_culture() -> Objective {
    Objective::builder(
        "High Culture",
        "You have gained all 4 of your civilization advances \
        and recruited at least 2 of your leaders.",
    )
    .status_phase_check(|_game, player| {
        player
            .civilization
            .special_advances
            .iter()
            .all(|a| player.has_special_advance(a.advance))
            && player.recruited_leaders.len() >= 2
    })
    .build()
}
