use super::custom_actions::CustomActionType::*;
use crate::action::Action;
use crate::advance::AdvanceBuilder;
use crate::collect::CollectContext;
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::{
    ability_initializer::AbilityInitializerSetup,
    advance::{Advance, Bonus::*},
    game::Game,
    map::Terrain::*,
    resource_pile::ResourcePile,
};
use std::collections::HashMap;

//names of advances that need special handling
pub const NAVIGATION: &str = "Navigation";
pub const ROADS: &str = "Roads";
pub const SIEGECRAFT: &str = "Siegecraft";
pub const STEEL_WEAPONS: &str = "Steel Weapons";
pub const METALLURGY: &str = "Metallurgy";
pub const RITUALS: &str = "Rituals";
pub const TACTICS: &str = "Tactics";

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
        ("Spirituality".to_string(), spirituality()),
        // second half of the advances
        ("Economy".to_string(), vec![]),
        ("Culture".to_string(), vec![]),
        ("Science".to_string(), science()),
        ("Democracy".to_string(), democracy()),
        ("Autocracy".to_string(), autocracy()),
        ("Theocracy".to_string(), theocracy()),
    ]
}

fn advance_group(name: &str, advances: Vec<AdvanceBuilder>) -> Vec<Advance> {
    assert_eq!(name, advances[0].name);
    advances
        .into_iter()
        .enumerate()
        .map(|(index, builder)| {
            if index > 0 {
                assert_eq!(builder.required_advance, None);
                builder.with_required_advance(name).build()
            } else {
                // first advance in the group
                builder.build()
            }
        })
        .collect()
}

fn agriculture() -> Vec<Advance> {
    advance_group(
        "Farming",
        vec![
            Advance::builder(
                "Farming",
                "Your cities may Collect food from Grassland and wood from Forest spaces",
            ),
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
                .with_advance_bonus(MoodToken),
            Advance::builder(
                "Irrigation",
                "Your cities may Collect food from Barren spaces, Ignore Famine events",
            )
                .add_player_event_listener(
                    |event| &mut event.collect_options,
                    irrigation_collect,
                    0,
                )
                .with_advance_bonus(MoodToken),
            Advance::builder(
                "Husbandry",
                "During a Collect Resources Action, you may collect from a Land space that is 2 Land spaces away, rather than 1. If you have the Roads Advance you may collect from two Land spaces that are 2 Land spaces away. This Advance can only be used once per turn.",
            )
                //todo advance bonus?
                .add_player_event_listener(
                    |event| &mut event.collect_options,
                    husbandry_collect,
                    0,
                )
                .add_player_event_listener(
                    |event| &mut event.on_execute_action,
                    |player, action, ()| {
                        if is_husbandry_action(action) {
                            player.played_once_per_turn_effects.push("Husbandry".to_string());
                        }
                    },
                    0
                )
                .add_player_event_listener(
                    |event| &mut event.on_undo_action,
                    |player, action, ()| {
                        if is_husbandry_action(action) {
                            player.played_once_per_turn_effects.retain(|a| a != "Husbandry");
                        }
                    },
                    0
                )
        ],
    )
}

fn irrigation_collect(
    options: &mut HashMap<Position, Vec<ResourcePile>>,
    c: &CollectContext,
    game: &Game,
) {
    c.city_position
        .neighbors()
        .iter()
        .chain(std::iter::once(&c.city_position))
        .filter(|pos| game.map.get(**pos) == Some(&Barren))
        .for_each(|pos| {
            options.insert(*pos, vec![ResourcePile::food(1)]);
        });
}

fn is_husbandry_action(action: &Action) -> bool {
    match action {
        Action::Playing(PlayingAction::Collect(collect)) => collect
            .collections
            .iter()
            .any(|c| c.0.distance(collect.city_position) > 1),
        _ => false,
    }
}

fn husbandry_collect(
    options: &mut HashMap<Position, Vec<ResourcePile>>,
    c: &CollectContext,
    game: &Game,
) {
    let player = &game.players[c.player_index];
    let allowed = if player
        .played_once_per_turn_effects
        .contains(&"Husbandry".to_string())
    {
        0
    } else if player.has_advance(ROADS) {
        2
    } else {
        1
    };
    
    if c.used.iter().filter(|(pos, _)| pos.distance(c.city_position) == 2).count() == allowed {
        return;
    }

    game.map
        .tiles
        .into_iter()
        .filter(|(pos, t)| pos.distance(c.city_position) == 2)
        .for_each(|pos| {
            
            options.insert(*pos, vec![ResourcePile::food(1)]);
        });
}

fn construction() -> Vec<Advance> {
    advance_group(
        "Mining",
        vec![
            Advance::builder("Mining", "Your cities may Collect ore from Mountain spaces"),
            Advance::builder(
                "Engineering",
                "Immediately draw 1 wonder, May Construct wonder happy cities",
            )
                .add_one_time_ability_initializer(Game::draw_wonder_card)
                .add_custom_action(ConstructWonder),
            Advance::builder(ROADS, "When moving from or to a city, you may pay 1 food and 1 ore to extend the range of a group of land units by 1 and ignore terrain effects. May not be used to embark, disembark, or explore")
        ],
    )
}

fn seafaring() -> Vec<Advance> {
    advance_group(
        "Fishing",
        vec![
            Advance::builder("Fishing", "Your cities may Collect food from one Sea space")
                .add_player_event_listener(|event| &mut event.collect_options, fishing_collect, 0)
                .with_advance_bonus(MoodToken),
            Advance::builder(
                NAVIGATION,
                "Ships may leave the map and return at the next sea space",
            ),
        ],
    )
}

fn fishing_collect(
    options: &mut HashMap<Position, Vec<ResourcePile>>,
    c: &CollectContext,
    game: &Game,
) {
    let city = game
        .get_any_city(c.city_position)
        .expect("city should exist");
    let port = city.port_position;
    if let Some(position) = port.or_else(|| {
        c.city_position
            .neighbors()
            .into_iter()
            .find(|pos| game.map.is_water(*pos))
    }) {
        options.insert(
            position,
            if port.is_some() {
                vec![
                    ResourcePile::food(1),
                    ResourcePile::gold(1),
                    ResourcePile::mood_tokens(1),
                ]
            } else {
                vec![ResourcePile::food(1)]
            },
        );
    }
}

fn education() -> Vec<Advance> {
    advance_group(
        "Philosophy",
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
        .with_advance_bonus(MoodToken)],
    )
}

fn warfare() -> Vec<Advance> {
    advance_group(
        TACTICS,
        vec![
            Advance::builder(
                TACTICS,
                "May Move Army units, May use Tactics on Action Cards",
            )
                .with_advance_bonus(CultureToken)
                .with_unlocked_building("Fortress"),
            Advance::builder(
                SIEGECRAFT,
                "When attacking a city with a Fortress, pay 2 wood to cancel the Fortress’ ability to add +1 die and/or pay 2 ore to ignore its ability to cancel a hit.",
            ),
            Advance::builder(
                STEEL_WEAPONS,
                "Immediately before a Land battle starts, you may pay 1 ore to get +2 combat value in every Combat Round against an enemy that does not have the Steel Weapons advance, but only +1 combat value against an enemy that does have it (regardless if they use it or not this battle).",
            ),
        ],
    )
}

fn spirituality() -> Vec<Advance> {
    advance_group(
        "Myths",
        vec![
            Advance::builder("Myths", "not implemented"),
            Advance::builder(RITUALS, "When you perform the Increase Happiness Action you may spend any Resources as a substitute for mood tokens. This is done at a 1:1 ratio"),
        ],
    )
}

fn science() -> Vec<Advance> {
    advance_group(
        "Math",
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
                .with_unlocked_building("Observatory"),
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
                .with_advance_bonus(CultureToken),
            // part of metallurgy is not implemented
            Advance::builder(
                METALLURGY,
                "If you have the Steel Weapons Advance, you no longer have to pay 1 ore to activate it against enemies without Steel Weapons.")
                .with_advance_bonus(CultureToken),
        ],
    )
}

fn democracy() -> Vec<Advance> {
    advance_group(
        "Voting",
        vec![
            Advance::builder(
                "Voting",
                "As a free action, you may spend 1 mood token to gain an action 'Increase happiness'",
            )
                .leading_government_advance("Democracy")
                .with_required_advance("Philosophy")
                .with_contradicting_advance(&["Nationalism", "Dogma"])
                .add_custom_action(VotingIncreaseHappiness),
            Advance::builder("Free Economy", "As a free action, you may spend 1 mood token to collect resources in one city. This must be your only collect action this turn")
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
        ],
    )
}

fn autocracy() -> Vec<Advance> {
    advance_group(
        "Nationalism",
        vec![
            Advance::builder("Nationalism", "TestGovernment1")
                .leading_government_advance("Autocracy")
                .with_contradicting_advance(&["Voting", "Dogma"])
            , Advance::builder("Absolute Power", "Once per turn, as a free action, you may spend 2 mood tokens to get an additional action")
                .add_custom_action(ForcedLabor)
            , ],
    )
}

fn theocracy() -> Vec<Advance> {
    advance_group(
        "Dogma",
        vec![
            Advance::builder("Dogma", "TestGovernment2")
                .leading_government_advance("Theocracy")
                .with_contradicting_advance(&["Voting", "Nationalism"]),
            Advance::builder("Theocracy 2", "TestGovernment2"),
        ],
    )
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
