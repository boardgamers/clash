use super::custom_actions::CustomActionType::*;
use crate::playing_actions::PlayingActionType;
use crate::{
    ability_initializer::AbilityInitializerSetup,
    advance::{Advance, Bonus::*},
    game::Game,
    map::Terrain::*,
    resource_pile::ResourcePile,
};

//names of advances that need special handling
pub const NAVIGATION: &str = "Navigation";

#[must_use]
pub fn get_all() -> Vec<Advance> {
    get_groups()
        .into_iter()
        .flat_map(|(_, advances)| advances)
        .collect()
}

#[must_use]
pub fn get_groups() -> Vec<(String, Vec<Advance>)> {
    vec![
        ("Agriculture".to_string(), agriculture()),
        ("Construction".to_string(), construction()),
        ("Seafaring".to_string(), seafaring()),
        ("Education".to_string(), education()),
        ("Warfare".to_string(), warfare()),
        ("Spirituality".to_string(), vec![]),
        // second half of the advances
        ("Economy".to_string(), vec![]),
        ("Culture".to_string(), vec![]),
        ("Science".to_string(), science()),
        ("Democracy".to_string(), democracy()),
        ("Autocracy".to_string(), autocracy()),
        ("Theocracy".to_string(), theocracy()),
    ]
}

fn agriculture() -> Vec<Advance> {
    vec![
        Advance::builder(
            "Farming",
            "Your cities may Collect food from Grassland and wood from Forest spaces",
        )
        .build(),
        Advance::builder(
            "Storage",
            "Your maximum food limit is increased from 2 to 7",
        )
        .add_one_time_ability_initializer(|game, player_index| {
            game.players[player_index].resource_limit.food = 7;
        })
        .add_ability_undo_deinitializer(|game, player_index| {
            game.players[player_index].resource_limit.food = 2;
        })
        .with_advance_bonus(MoodToken)
        .with_required_advance("Farming")
        .build(),
        Advance::builder(
            "Irrigation",
            "Your cities may Collect food from Barren spaces, Ignore Famine events",
        )
        .add_collect_option(Barren, ResourcePile::food(1))
        .with_advance_bonus(MoodToken)
        .with_required_advance("Farming")
        .build(),
    ]
}

fn construction() -> Vec<Advance> {
    vec![
        Advance::builder("Mining", "Your cities may Collect ore from Mountain spaces").build(),
        Advance::builder(
            "Engineering",
            "Immediately draw 1 wonder, May Construct wonder happy cities",
        )
        .add_one_time_ability_initializer(Game::draw_wonder_card)
        .add_custom_action(ConstructWonder)
        .with_required_advance("Mining")
        .build(),
    ]
}

fn seafaring() -> Vec<Advance> {
    vec![
        Advance::builder("Fishing", "Your cities may Collect food from one Sea space")
            .add_collect_option(Water, ResourcePile::food(1))
            .with_advance_bonus(MoodToken)
            .build(),
        Advance::builder(
            NAVIGATION,
            "Ships may leave the map and return at the next sea space",
        )
        .build(),
    ]
}

fn education() -> Vec<Advance> {
    vec![Advance::builder(
        "Philosophy",
        "Immediately gain 1 idea, Gain 1 idea after getting a Science advance",
    )
    .add_one_time_ability_initializer(|game, player_index| {
        game.players[player_index].gain_resources(ResourcePile::ideas(1));
    })
    .add_ability_undo_deinitializer(|game, player_index| {
        game.players[player_index].loose_resources(ResourcePile::ideas(1));
    })
    .add_player_event_listener(
        |event| &mut event.on_advance,
        |player, advance, ()| {
            if advance == "Math"
                || advance == "Astronomy"
                || advance == "Medicine"
                || advance == "Metallurgy"
            {
                player.gain_resources(ResourcePile::ideas(1));
            }
        },
        0,
    )
    .add_player_event_listener(
        |event| &mut event.on_undo_advance,
        |player, advance, ()| {
            if advance == "Math"
                || advance == "Astronomy"
                || advance == "Medicine"
                || advance == "Metallurgy"
            {
                player.loose_resources(ResourcePile::ideas(1));
            }
        },
        0,
    )
    .with_advance_bonus(MoodToken)
    .build()]
}

fn warfare() -> Vec<Advance> {
    vec![Advance::builder(
        "Tactics",
        "May Move Army units, May use Tactics on Action Cards",
    )
    .with_advance_bonus(CultureToken)
    .with_unlocked_building("Fortress")
    .build()]
}

fn science() -> Vec<Advance> {
    vec![
        Advance::builder(
            "Math",
            "Engineering and Roads can be bought at no food cost",
        )
        .add_player_event_listener(
            |event| &mut event.advance_cost,
            |cost, advance, ()| {
                if advance == "Engineering" || advance == "Roads" {
                    *cost = 0;
                }
            },
            0,
        )
        .with_advance_bonus(CultureToken)
        .with_unlocked_building("Observatory")
        .build(),
        Advance::builder(
            "Astronomy",
            "Navigation and Cartography can be bought at no food cost",
        )
        .add_player_event_listener(
            |event| &mut event.advance_cost,
            |cost, advance, ()| {
                if advance == "Navigation" || advance == "Cartography" {
                    *cost = 0;
                }
            },
            0,
        )
        .with_required_advance("Math")
        .with_advance_bonus(CultureToken)
        .build(),
    ]
}

fn democracy() -> Vec<Advance> {
    vec![
        Advance::builder(
            "Voting",
            "As a free action, you may spend 1 mood token to gain an action 'Increase happiness'",
        )
        .leading_government_advance("Democracy")
        .with_required_advance("Philosophy")
        .with_contradicting_advance(&["Nationalism", "Dogma"])
        .add_custom_action(VotingIncreaseHappiness)
        .build(),
        Advance::builder("Free Economy", "As a free action, you may spend 1 mood token to collect resources in one city. This must be your only collect action this turn")
            .with_required_advance("Voting")
            .add_custom_action(FreeEconomyCollect)
            .add_player_event_listener(
                |event| &mut event.is_playing_action_available,
                |available, action_type, player| {
                    if matches!(action_type, PlayingActionType::Collect) && player.played_once_per_turn_actions.contains(&FreeEconomyCollect) {
                        *available = false;
                    }
                },
                0,
            )
            .build(),
    ]
}

fn autocracy() -> Vec<Advance> {
    vec![
       Advance::builder("Nationalism", "TestGovernment1")
            .leading_government_advance("Autocracy")
           .with_contradicting_advance(&["Voting", "Dogma"])
            .build(),
        Advance::builder("Absolute Power", "Once per turn, as a free action, you may spend 2 mood tokens to get an additional action")
            .with_required_advance("Nationalism")
            .add_custom_action(ForcedLabor)
            .build(),
    ]
}

fn theocracy() -> Vec<Advance> {
    vec![
        Advance::builder("Dogma", "TestGovernment2")
            .leading_government_advance("Theocracy")
            .with_contradicting_advance(&["Voting", "Nationalism"])
            .build(),
        Advance::builder("Theocracy 2", "TestGovernment2")
            .with_required_advance("Dogma")
            .build(),
    ]
}

///
/// # Panics
///
/// Panics if advance with name doesn't exist
#[must_use]
pub fn get_advance_by_name(name: &str) -> Advance {
    get_all()
        .into_iter()
        .find(|advance| advance.name == name)
        .unwrap_or_else(|| {
            panic!("Advance with name {name} not found");
        })
}

#[must_use]
pub fn get_leading_government_advance(government: &str) -> Option<Advance> {
    get_all().into_iter().find(|advance| {
        advance
            .government
            .as_ref()
            .is_some_and(|value| value.as_str() == government)
    })
}

#[must_use]
pub fn get_governments() -> Vec<(String, Advance)> {
    get_all()
        .into_iter()
        .filter_map(|advance| advance.government.clone().map(|g| (g.clone(), advance)))
        .collect()
}

///
///
/// # Panics
///
/// Panics if government doesn't have a leading government advance or if some of the government advances do no have their government tier specified
#[must_use]
pub fn get_government(government: &str) -> Vec<Advance> {
    let leading_government =
        get_leading_government_advance(government).expect("government should exist");
    let mut government_advances = get_all()
        .into_iter()
        .filter(|advance| {
            advance
                .required
                .as_ref()
                .is_some_and(|required_advance| required_advance == &leading_government.name)
        })
        .collect::<Vec<Advance>>();
    government_advances.push(leading_government);
    government_advances.reverse();
    government_advances
}
