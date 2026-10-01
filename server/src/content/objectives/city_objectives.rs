use crate::city::MoodState;
use crate::city_pieces::Building;
use crate::game::Game;
use crate::objective_card::{Objective, ObjectiveProgress};
use crate::player::Player;
use itertools::Itertools;

pub(crate) fn science_lead() -> Objective {
    building_lead("Scientific Lead", Building::Academy)
}

pub(crate) fn coastal_lead() -> Objective {
    building_lead("Coastal Culture", Building::Port)
}

pub(crate) fn religious_fervor() -> Objective {
    building_lead("Religious Fervor", Building::Temple)
}

pub(crate) fn fortifications() -> Objective {
    building_lead("Fortifications", Building::Fortress)
}

pub(crate) fn star_gazers() -> Objective {
    building_lead("Star Gazers", Building::Observatory)
}

pub(crate) fn traders() -> Objective {
    building_lead("Traders", Building::Market)
}

pub(crate) fn legacy() -> Objective {
    building_lead("Legacy", Building::Obelisk)
}

fn building_lead(objective: &'static str, building: Building) -> Objective {
    Objective::builder(
        objective,
        &format!("More of your cities contain a {building} than any other player's cities. The buildings may be any color.",),
    )
    .status_phase_progress(move |game, player| {
        vec![leading_progress(&format!("Cities with {building} · lead by 1"), game, player, 1, move |p, _| buildings(p, building))]
    })
    .build()
}

fn buildings(p: &Player, b: Building) -> usize {
    p.cities
        .iter()
        .filter(|c| c.pieces.building_owner(b).is_some())
        .count()
}

pub(crate) fn large_civ() -> Objective {
    Objective::builder("Large Civilization", "You own at least 6 cities.")
        .status_phase_progress(|_game, player| {
            vec![ObjectiveProgress::new("Cities", player.cities.len(), 6)]
        })
        .build()
}

pub(crate) fn leading_progress(
    label: &str,
    game: &Game,
    player: &Player,
    margin: usize,
    value: impl Fn(&Player, &Game) -> usize,
) -> ObjectiveProgress {
    ObjectiveProgress::new(
        label,
        value(player, game),
        game.players
            .iter()
            .filter(|p| p.index != player.index && p.is_human())
            .map(|p| value(p, game))
            .max()
            .unwrap_or(0)
            + margin,
    )
}

pub(crate) fn advanced_culture() -> Objective {
    Objective::builder(
        "Advanced Culture",
        "You have at least 6 standard advances, and more than every other player. Civilization advances do not count.",
    )
    .status_phase_progress(|game, player| {
        let mut progress = leading_progress("Standard advances · at least 6, lead by 1", game, player, 1, |p, _| p.advances.len());
        progress.target = progress.target.max(6);
        vec![progress]
    })
    .build()
}

pub(crate) fn happy_population() -> Objective {
    Objective::builder("Happy Population", "You own at least 4 Happy cities.")
        .status_phase_progress(|_game, player| {
            vec![ObjectiveProgress::new(
                "Happy cities",
                player
                    .cities
                    .iter()
                    .filter(|c| c.mood_state == MoodState::Happy)
                    .count(),
                4,
            )]
        })
        .build()
}

pub(crate) fn architecture() -> Objective {
    Objective::builder(
        "Architecture",
        "Your cities contain at least 4 different building types in your color.",
    )
    .status_phase_progress(|_game, player| {
        vec![ObjectiveProgress::new(
            "Building types in your color",
            player
                .cities
                .iter()
                .flat_map(|c| c.pieces.buildings(Some(player.index)))
                .unique()
                .count(),
            4,
        )]
    })
    .build()
}

pub(crate) fn consulate() -> Objective {
    Objective::builder(
        "Consulate",
        "At least 2 cities owned by other players contain buildings in your color.",
    )
    .status_phase_progress(|game, player| {
        vec![ObjectiveProgress::new(
            "Other cities with your buildings",
            game.players
                .iter()
                .filter(|p| p.index != player.index)
                .flat_map(|p| &p.cities)
                .filter(|c| !c.pieces.buildings(Some(player.index)).is_empty())
                .count(),
            2,
        )]
    })
    .build()
}

pub(crate) fn metropolis() -> Objective {
    Objective::builder(
        "Metropolis",
        "You own at least 1 city of size 5 or greater.",
    )
    .status_phase_progress(|_game, player| {
        vec![ObjectiveProgress::new(
            "Largest city size",
            player
                .cities
                .iter()
                .map(|c| c.size() as usize)
                .max()
                .unwrap_or(0),
            5,
        )]
    })
    .build()
}

pub(crate) fn expansionist() -> Objective {
    Objective::builder(
        "Expansionist",
        "You own at least 4 cities that are not adjacent to any other city, including your own and barbarian cities.",
    )
    .status_phase_progress(|game, player| {
        vec![ObjectiveProgress::new("Non-adjacent cities", player
            .cities
            .iter()
            .filter(|c| {
                c.position
                    .neighbors()
                    .iter()
                    .all(|n| game.try_get_any_city(*n).is_none())
            })
            .count(), 4)]
    })
    .build()
}

pub(crate) fn culture_power() -> Objective {
    Objective::builder(
        "Culture Power",
        "You have influenced more buildings than any other player.",
    )
    .status_phase_progress(|game, player| {
        vec![leading_progress(
            "Influenced buildings · lead by 1",
            game,
            player,
            1,
            influenced_buildings,
        )]
    })
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
