use crate::city::MoodState;
use crate::city_pieces::Building;
use crate::game::Game;
use crate::objective_card::{Objective, ObjectiveBuilder};
use crate::player::Player;
use itertools::Itertools;

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

pub(crate) fn religious_fervor() -> Objective {
    building_lead(
        Objective::builder(
            "Religious Fervor",
            "You have more temples than any other player",
        ),
        Building::Temple,
    )
    .build()
}

pub(crate) fn fortifications() -> Objective {
    building_lead(
        Objective::builder(
            "Fortifications",
            "You have more fortresses than any other player",
        ),
        Building::Fortress,
    )
    .build()
}

fn building_lead(b: ObjectiveBuilder, building: Building) -> ObjectiveBuilder {
    b.status_phase_check(move |game, player| {
        leading_player(game, player, 1, move |p, _| buildings(p, building))
    })
}

fn buildings(p: &Player, b: Building) -> usize {
    p.cities
        .iter()
        .filter(|c| c.pieces.building_owner(b).is_some())
        .count()
}

pub(crate) fn large_civ() -> Objective {
    Objective::builder("Large Civilization", "You have at least 6 cities")
        .status_phase_check(|_game, player| player.cities.len() >= 6)
        .build()
}

pub(crate) fn leading_player(
    game: &Game,
    player: &Player,
    margin: usize,
    value: impl Fn(&Player, &Game) -> usize + 'static,
) -> bool {
    value(player, game)
        >= game
            .players
            .iter()
            .filter(|p| p.index != player.index && p.is_human())
            .map(|p| value(p, game))
            .max()
            .unwrap_or(0)
            + margin
}

pub(crate) fn advanced_culture() -> Objective {
    Objective::builder(
        "Advanced Culture",
        "You have more advances than any other player - at least 6.",
    )
    .status_phase_check(|game, player| {
        player.advances.len() >= 6 && leading_player(game, player, 1, move |p, _| p.advances.len())
    })
    .build()
}

pub(crate) fn happy_population() -> Objective {
    Objective::builder("Happy Population", "You have at least 4 happy cities.")
        .status_phase_check(|_game, player| {
            player
                .cities
                .iter()
                .filter(|c| c.mood_state == MoodState::Happy)
                .count()
                >= 4
        })
        .build()
}

pub(crate) fn diversity() -> Objective {
    Objective::builder(
        "Diversity",
        "You have at least 4 different types of buildings \
        (that are not influenced by another player).",
    )
    .status_phase_check(|_game, player| {
        player
            .cities
            .iter()
            .flat_map(|c| c.pieces.buildings(Some(player.index)))
            .unique()
            .count()
            >= 4
    })
    .build()
}

pub(crate) fn consulate() -> Objective {
    Objective::builder("Consulate", "2 cities are culturally influenced by you.")
        .status_phase_check(|game, player| {
            game.players
                .iter()
                .filter(|p| p.index != player.index)
                .flat_map(|p| &p.cities)
                .filter(|c| !c.pieces.buildings(Some(player.index)).is_empty())
                .count()
                >= 2
        })
        .build()
}

pub(crate) fn metropolis() -> Objective {
    Objective::builder("Metropolis", "You have at least 1 city with size 5.")
        .status_phase_check(|_game, player| {
            player.cities.iter().filter(|c| c.size() >= 5).count() >= 1
        })
        .build()
}

pub(crate) fn expansionist() -> Objective {
    Objective::builder(
        "Expansionist",
        "You have at least 4 cities that are not adjacent to other cities.",
    )
    .status_phase_check(|game, player| {
        player
            .cities
            .iter()
            .filter(|c| {
                c.position
                    .neighbors()
                    .iter()
                    .all(|n| game.try_get_any_city(*n).is_none())
            })
            .count()
            >= 4
    })
    .build()
}

pub(crate) fn culture_power() -> Objective {
    Objective::builder(
        "Culture Power",
        "You have influenced more buildings than any other player.",
    )
    .status_phase_check(|game, player| leading_player(game, player, 1, influenced_buildings))
    .build()
}

fn influenced_buildings(player: &Player, game: &Game) -> usize {
    game.players
        .iter()
        .filter(|p| p.index != player.index)
        .flat_map(|p| &p.cities)
        .map(|c| c.pieces.buildings(Some(player.index)).len())
        .sum()
}
