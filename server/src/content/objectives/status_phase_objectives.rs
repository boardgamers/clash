use crate::city_pieces::Building;
use crate::objective_card::{Objective, ObjectiveBuilder};
use crate::player::Player;

pub(crate) fn status_phase_objectives() -> Vec<Objective> {
    vec![large_civ(), science_lead(), coastal_lead()]
}

pub(crate) fn large_civ() -> Objective {
    Objective::builder("Large Civilization", "You have at least 6 cities")
        .status_phase_check(|_game, player| player.cities.len() >= 6)
        .build()
}

pub(crate) fn science_lead() -> Objective {
    building_lead(
        Objective::builder(
            "Scientific Lead",
            "You have more academies than any other player",
        ),
        Building::Academy,
    )
    .build()
}

pub(crate) fn coastal_lead() -> Objective {
    building_lead(
        Objective::builder(
            "Coastal Culture",
            "You have more ports than any other player",
        ),
        Building::Port,
    )
    .build()
}

fn building_lead(b: ObjectiveBuilder, building: Building) -> ObjectiveBuilder {
    b.status_phase_check(move |game, player| {
        buildings(player, building)
            > game
                .players
                .iter()
                .filter(|p| p.index != player.index)
                .map(|p| buildings(p, building))
                .max()
                .unwrap_or(0)
    })
}

fn buildings(p: &Player, b: Building) -> usize {
    p.cities
        .iter()
        .filter(|c| c.pieces.building_owner(b).is_some())
        .count()
}
