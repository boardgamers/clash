use crate::objective_card::Objective;

pub(crate) fn status_phase_objectives() -> Vec<Objective> {
    vec![large_civ()]
}

pub(crate) fn large_civ() -> Objective {
    Objective::builder("Large Civilization", "You have at least 6 cities")
        .status_phase_check(|_game, player| player.cities.len() >= 6)
        .build()
}
