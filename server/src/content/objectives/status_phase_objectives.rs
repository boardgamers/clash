use crate::city_pieces::Building;
use crate::game::Game;
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
        leading_player(game, player, move |p| buildings(p, building))
    })
}

fn leading_player(
    game: &Game,
    player: &Player,
    value: impl Fn(&Player) -> usize + 'static,
) -> bool {
    value(player)
        > game
            .players
            .iter()
            .filter(|p| p.index != player.index)
            .map(|p| value(p))
            .max()
            .unwrap_or(0)
}

fn buildings(p: &Player, b: Building) -> usize {
    p.cities
        .iter()
        .filter(|c| c.pieces.building_owner(b).is_some())
        .count()
}

pub(crate) fn advanced_culture() -> Objective {
    Objective::builder(
        "Advanced Culture",
        "You have more advances than any other player - at least 6.",
    )
    .status_phase_check(|game, player| {
        player.advances.len() >= 6 && leading_player(game, player, move |p| p.advances.len())
    })
    .build()
}
