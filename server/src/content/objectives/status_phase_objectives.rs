use crate::objective_card::Objective;
use crate::player::Player;

pub(crate) fn status_phase_objectives() -> Vec<Objective> {
    vec![large_civ(), science_lead()]
}

pub(crate) fn large_civ() -> Objective {
    Objective::builder("Large Civilization", "You have at least 6 cities")
        .status_phase_check(|_game, player| player.cities.len() >= 6)
        .build()
}

pub(crate) fn science_lead() -> Objective {
    Objective::builder(
        "Scientific Lead",
        "You have more advances than any other player",
    )
    .status_phase_check(|game, player| {
        academies(player)
            > game
                .players
                .iter()
                .filter(|p| p.index != player.index)
                .map(academies)
                .max()
                .unwrap_or(0)
    })
    .build()
}

fn academies(p: &Player) -> usize {
    p.cities
        .iter()
        .filter(|c| c.pieces.academy.is_some())
        .count()
}
