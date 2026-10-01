use crate::content::advances::AdvanceGroup;
use crate::content::objectives::city_objectives::leading_progress;
use crate::game::Game;
use crate::objective_card::{Objective, ObjectiveProgress};
use crate::player::Player;

fn advance_group_complete(objective: &str, group: AdvanceGroup) -> Objective {
    let desc = format!("You have all {group} advances.");
    Objective::builder(objective, &desc)
        .status_phase_progress(move |game, player| vec![group_progress(player, group, game)])
        .build()
}

fn group_progress(player: &Player, group: AdvanceGroup, game: &Game) -> ObjectiveProgress {
    let advances = &game.cache.get_advance_group(group).advances;
    ObjectiveProgress::new(
        format!("{group} advances"),
        advances
            .iter()
            .filter(|a| player.has_advance(a.advance))
            .count(),
        advances.len(),
    )
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
    .status_phase_progress(|game, player| {
        let government = game
            .cache
            .get_governments()
            .iter()
            .max_by_key(|g| {
                game.cache
                    .get_advance_group(g.advance_group)
                    .advances
                    .iter()
                    .filter(|a| player.has_advance(a.advance))
                    .count()
            })
            .unwrap();
        vec![group_progress(player, government.advance_group, game)]
    })
    .build()
}

pub(crate) fn goal_focused() -> Objective {
    Objective::builder(
        "Goal Focused",
        "You have more complete advance groups than any other player.",
    )
    .status_phase_progress(|game, player| {
        vec![leading_progress(
            "Complete groups · lead by 1",
            game,
            player,
            1,
            |p, g| {
                g.cache
                    .get_advance_groups()
                    .iter()
                    .filter(|g| g.advances.iter().all(|a| p.has_advance(a.advance)))
                    .count()
            },
        )]
    })
    .build()
}

pub(crate) fn diversified_research() -> Objective {
    Objective::builder(
        "Diversified Research",
        "You have at least 1 advance in 9 different advance groups.",
    )
    .status_phase_progress(|game, player| {
        vec![ObjectiveProgress::new(
            "Groups with an advance",
            game.cache
                .get_advance_groups()
                .iter()
                .filter(|g| g.advances.iter().any(|a| player.has_advance(a.advance)))
                .count(),
            9,
        )]
    })
    .build()
}

pub(crate) fn high_culture() -> Objective {
    Objective::builder(
        "High Culture",
        "You have gained all 4 of your civilization advances \
        and recruited at least 2 of your leaders.",
    )
    .status_phase_progress(|_game, player| {
        vec![
            ObjectiveProgress::new(
                "Civilization advances",
                player
                    .civilization
                    .special_advances
                    .iter()
                    .filter(|a| player.has_special_advance(a.advance))
                    .count(),
                player.civilization.special_advances.len(),
            ),
            ObjectiveProgress::new("Leaders recruited", player.recruited_leaders.len(), 2),
        ]
    })
    .build()
}
