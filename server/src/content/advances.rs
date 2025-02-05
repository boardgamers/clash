use super::custom_actions::CustomActionType::*;
use crate::action::Action;
use crate::advance::AdvanceBuilder;
use crate::city_pieces::Building::{Academy, Fortress, Market, Obelisk, Observatory, Port, Temple};
use crate::collect::CollectContext;
use crate::combat::CombatModifier::*;
use crate::combat::{Combat, CombatModifier, CombatStrength};
use crate::content::custom_phase_actions::{CustomPhasePaymentRequest, CustomPhaseRewardRequest};
use crate::content::trade_routes::{gain_trade_route_reward, trade_route_reward};
use crate::game::GameState;
use crate::payment::{PaymentConversion, PaymentOptions};
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::unit::UnitType;
use crate::{
    ability_initializer::AbilityInitializerSetup,
    advance::{Advance, Bonus::*},
    game::Game,
    map::Terrain::*,
    resource_pile::ResourcePile,
};
use std::collections::{HashMap, HashSet};
use std::vec;
// use crate::content::trade_routes::start_trade_routes;

//names of advances that need special handling
pub const NAVIGATION: &str = "Navigation";
pub const ROADS: &str = "Roads";
pub const STEEL_WEAPONS: &str = "Steel Weapons";
pub const METALLURGY: &str = "Metallurgy";
pub const TACTICS: &str = "Tactics";
pub const BARTERING: &str = "Bartering";
pub const CURRENCY: &str = "Currency";
pub const IRRIGATION: &str = "Irrigation";

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
        ("Economy".to_string(), economy()),
        ("Culture".to_string(), culture()),
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
                IRRIGATION,
                "Your cities may Collect food from Barren spaces, Ignore Famine events",
            )
                .add_player_event_listener(
                    |event| &mut event.terrain_collect_options,
                    |m, (), ()| {
                        m.insert(Barren, HashSet::from([ResourcePile::food(1)]));
                    },
                    0,
                )
                .with_advance_bonus(MoodToken),
            Advance::builder(
                "Husbandry",
                "During a Collect Resources Action, you may collect from a Land space that is 2 Land spaces away, rather than 1. If you have the Roads Advance you may collect from two Land spaces that are 2 Land spaces away. This Advance can only be used once per turn.",
            )
                .with_advance_bonus(MoodToken)
                .add_player_event_listener(
                    |event| &mut event.collect_options,
                    husbandry_collect,
                    0,
                )
                .add_once_per_turn_effect("Husbandry", is_husbandry_action)
        ],
    )
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
    options: &mut HashMap<Position, HashSet<ResourcePile>>,
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

    if c.used
        .iter()
        .filter(|(pos, _)| pos.distance(c.city_position) == 2)
        .count()
        == allowed
    {
        return;
    }

    game.map
        .tiles
        .iter()
        .filter(|(pos, t)| pos.distance(c.city_position) == 2 && t.is_land())
        .for_each(|(pos, t)| {
            options.insert(*pos, c.terrain_options.get(t).cloned().unwrap_or_default());
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
            Advance::builder(
                "Sanitation",
                "When Recruiting, you may spend 1 mood token to pay for 1 Settler.")
                .with_advance_bonus(MoodToken)
                .add_player_event_listener(
                    |event| &mut event.recruit_cost,
                    |cost, (), ()| {
                        if cost.units.settlers > 0 {
                            cost.units.settlers -= 1;
                            // insert at beginning so that it's preferred over gold
                            cost.cost.conversions.insert(0, PaymentConversion {
                                from: vec![UnitType::cost(&UnitType::Settler)],
                                to: ResourcePile::mood_tokens(1),
                                limit: Some(1),
                            });
                        }
                    },
                    0,
                ),
            Advance::builder(ROADS, "When moving from or to a city, you may pay 1 food and 1 ore to extend the range of a group of land units by 1 and ignore terrain effects. May not be used to embark, disembark, or explore")
                .with_advance_bonus(CultureToken)
        ],
    )
}

fn seafaring() -> Vec<Advance> {
    advance_group(
        "Fishing",
        vec![
            Advance::builder("Fishing", "Your cities may Collect food from one Sea space")
                .add_player_event_listener(|event| &mut event.collect_options, fishing_collect, 0)
                .with_advance_bonus(MoodToken)
                .with_unlocked_building(Port),
            Advance::builder(
                NAVIGATION,
                "Ships may leave the map and return at the next sea space",
            )
            .with_advance_bonus(CultureToken),
            war_ships(),
            cartography(),
        ],
    )
}

fn war_ships() -> AdvanceBuilder {
    Advance::builder(
        "War Ships",
        "Ignore the first hit it the first round of combat when attacking with Ships or disembarking from Ships")
        .add_player_event_listener(
            |event| &mut event.on_combat_round,
            |s, c, g| {
                let attacker = s.attacker && g.map.is_water(c.attacker_position);
                let defender = !s.attacker && g.map.is_water(c.defender_position);
                if c.round == 1 && (attacker || defender) {
                    s.hit_cancels += 1;
                    s.roll_log.push("War Ships ignore the first hit in the first round of combat".to_string());
                }
            },
            0,
        )
}

fn cartography() -> AdvanceBuilder {
    Advance::builder(
        "Cartography",
        "Gain 1 idea after a move action where you moved a Ship. If you used navigation, gain an additional 1 culture token.", )
        .with_advance_bonus(CultureToken)
        .add_player_event_listener(
            |event| &mut event.before_move,
            |player, units, destination| {
                //todo only for first move
                //todo undo
                let mut ship = false;
                let mut navigation = false;
                for id in units {
                    let unit = player.get_unit(*id).expect("unit should exist");
                    if unit.unit_type.is_ship() {
                        ship = true;
                        if !unit.position.is_neighbor(*destination) {
                            navigation = true;
                        }
                    }
                }
                if ship {
                    player.gain_resources(ResourcePile::ideas(1));
                    if navigation {
                        player.gain_resources(ResourcePile::culture_tokens(1));
                    }
                }
            },
            0,
        )
}

fn fishing_collect(
    options: &mut HashMap<Position, HashSet<ResourcePile>>,
    c: &CollectContext,
    game: &Game,
) {
    let city = game
        .get_any_city(c.city_position)
        .expect("city should exist");
    let port = city.port_position;
    if let Some(position) =
        port.filter(|p| game.enemy_player(c.player_index, *p).is_none())
            .or_else(|| {
                c.city_position.neighbors().into_iter().find(|p| {
                    game.map.is_water(*p) && game.enemy_player(c.player_index, *p).is_none()
                })
            })
    {
        options.insert(
            position,
            if Some(position) == port {
                HashSet::from([
                    ResourcePile::food(1),
                    ResourcePile::gold(1),
                    ResourcePile::mood_tokens(1),
                ])
            } else {
                HashSet::from([ResourcePile::food(1)])
            },
        );
    }
}

fn education() -> Vec<Advance> {
    advance_group(
        "Writing",
        vec![
            Advance::builder("Writing", "todo")
                .with_advance_bonus(CultureToken)
                .with_unlocked_building(Academy),
            Advance::builder(
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
            .with_advance_bonus(MoodToken),
        ],
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
                .with_unlocked_building(Fortress)
                .add_player_event_listener(
                    |event| &mut event.on_combat_round,
                    fortress,
                    1),
            Advance::builder(
                "Siegecraft",
                "When attacking a city with a Fortress, pay 2 wood to cancel the Fortress’ ability to add +1 die and/or pay 2 ore to ignore its ability to cancel a hit.",
            )
                .add_payment_request_listener(
                    |e| &mut e.on_combat_start,
                    0,
                    |game, player_index| {
                        let GameState::Combat(c) = &game.state else { panic!("Invalid state") };

                        let extra_die = PaymentOptions::sum(2, &[ResourceType::Wood, ResourceType::Gold]);
                        let ignore_hit = PaymentOptions::sum(2, &[ResourceType::Ore, ResourceType::Gold]);

                        let player = &game.players[player_index];
                        if game
                            .get_any_city(c.defender_position)
                            .is_some_and(|c| c.pieces.fortress.is_some())
                            && (player.can_afford(&extra_die) || player.can_afford(&ignore_hit))
                        {
                            Some(vec![
                                CustomPhasePaymentRequest {
                                    cost: extra_die,
                                    name: "Cancel fortress ability to add an extra die in the first round of combat".to_string(),
                                    optional: true,
                                },
                                CustomPhasePaymentRequest {
                                    cost: ignore_hit,
                                    name: "Cancel fortress ability to ignore the first hit in the first round of combat".to_string(),
                                    optional: true,
                                },
                            ])
                        } else {
                            None
                        }
                    },
                    |game, _player_index, player_name, payment| {
                        game.add_info_log_item(
                            format!("{player_name} paid for siegecraft: "));
                        let mut paid = false;
                        let mut modifiers: Vec<CombatModifier> = Vec::new();
                        if !payment[0].is_empty() {
                            modifiers.push(CancelFortressExtraDie);
                            game.add_to_last_log_item(&format!("{} to cancel the fortress ability to add an extra die", payment[0]));
                            paid = true;
                        }
                        if !payment[1].is_empty() {
                            modifiers.push(CancelFortressIgnoreHit);
                            if paid {
                                game.add_to_last_log_item(" and ");
                            }
                            game.add_to_last_log_item(&format!("{} to cancel the fortress ability to ignore a hit", payment[1]));
                            paid = true;
                        }
                        if !paid {
                            game.add_to_last_log_item("nothing");
                        }
                        let GameState::Combat(c) = &mut game.state else { panic!("Invalid state") };
                        c.modifiers.extend(modifiers);
                    },
                ),
            Advance::builder(
                STEEL_WEAPONS,
                "Immediately before a Land battle starts, you may pay 1 ore to get +2 combat value in every Combat Round against an enemy that does not have the Steel Weapons advance, but only +1 combat value against an enemy that does have it (regardless if they use it or not this battle).",
            )
                .add_payment_request_listener(
                    |e| &mut e.on_combat_start,
                    1,
                    |game, player_index| {
                        let GameState::Combat(c) = &game.state else { panic!("Invalid state") };
                        let player = &game.players[player_index];

                        let cost = steel_weapons_cost(game, c, player_index);
                        if cost.is_free() {
                            let GameState::Combat(c) = &mut game.state else { panic!("Invalid state") };
                            add_steel_weapons(player_index, c);
                            return None;
                        }

                        if player.can_afford(&cost) {
                            Some(vec![CustomPhasePaymentRequest {
                                cost,
                                name: "Use steel weapons".to_string(),
                                optional: true,
                            }])
                        } else {
                            None
                        }
                    },
                    |game, player_index, player_name, payment| {
                        let GameState::Combat(c) = &mut game.state else { panic!("Invalid state") };
                        add_steel_weapons(player_index, c);
                        game.add_info_log_item(
                            format!("{player_name} paid for steel weapons: {}", payment[0]));
                    },
                )
                .add_player_event_listener(
                    |event| &mut event.on_combat_round,
                    use_steel_weapons,
                    0,
                ),
            Advance::builder(
                "Draft",
                "When Recruiting, you may spend 1 mood token to pay for 1 Infantry Army Unit.")
                .with_advance_bonus(CultureToken)
                .add_player_event_listener(
                    |event| &mut event.recruit_cost,
                    |cost, (), ()| {
                        if cost.units.infantry > 0 {
                            cost.units.infantry -= 1;
                            // insert at beginning so that it's preferred over gold
                            cost.cost.conversions.insert(0, PaymentConversion {
                                from: vec![UnitType::cost(&UnitType::Infantry)],
                                to: ResourcePile::mood_tokens(1),
                                limit: Some(1),
                            });
                        }
                    },
                    0,
                )
        ],
    )
}

fn add_steel_weapons(player_index: usize, c: &mut Combat) {
    if player_index == c.attacker {
        c.modifiers.push(SteelWeaponsAttacker);
    } else {
        c.modifiers.push(SteelWeaponsDefender);
    }
}

#[must_use]
fn steel_weapons_cost(game: &Game, combat: &Combat, player_index: usize) -> PaymentOptions {
    let player = &game.players[player_index];
    let attacker = &game.players[combat.attacker];
    let defender = &game.players[combat.defender];
    let both_steel_weapons =
        attacker.has_advance(STEEL_WEAPONS) && defender.has_advance(STEEL_WEAPONS);
    let cost = u32::from(!player.has_advance(METALLURGY) || both_steel_weapons);
    PaymentOptions::sum(cost, &[ResourceType::Ore, ResourceType::Gold])
}

fn fortress(s: &mut CombatStrength, c: &Combat, game: &Game) {
    if s.attacker || !c.defender_fortress(game) || c.round != 1 {
        return;
    }

    if !c.modifiers.contains(&CancelFortressExtraDie) {
        s.roll_log.push("fortress added one extra die".to_string());
        s.extra_dies += 1;
    }

    if !c.modifiers.contains(&CancelFortressIgnoreHit) {
        s.roll_log.push("fortress cancelled one hit".to_string());
        s.hit_cancels += 1;
    }
}

fn use_steel_weapons(s: &mut CombatStrength, c: &Combat, game: &Game) {
    let steel_weapon_value = if game.get_player(c.attacker).has_advance(STEEL_WEAPONS)
        && game.get_player(c.defender).has_advance(STEEL_WEAPONS)
    {
        1
    } else {
        2
    };

    let add_combat_value = |s: &mut CombatStrength, value: u8| {
        s.extra_combat_value += value;
        s.roll_log
            .push(format!("steel weapons added {value} combat value"));
    };

    if s.attacker {
        if c.modifiers.contains(&SteelWeaponsAttacker) {
            add_combat_value(s, steel_weapon_value);
        }
    } else if c.modifiers.contains(&SteelWeaponsDefender) {
        add_combat_value(s, steel_weapon_value);
    }
}

fn spirituality() -> Vec<Advance> {
    advance_group(
        "Myths",
        vec![
            Advance::builder("Myths", "not implemented")
                .with_advance_bonus(MoodToken)
                .with_unlocked_building(Temple),
            Advance::builder("Rituals", "When you perform the Increase Happiness Action you may spend any Resources as a substitute for mood tokens. This is done at a 1:1 ratio")
                .with_advance_bonus(CultureToken)
                .add_player_event_listener(
                    |event| &mut event.happiness_cost,
                    |cost, (), ()| {
                        for r in &[
                            ResourceType::Food,
                            ResourceType::Wood,
                            ResourceType::Ore,
                            ResourceType::Ideas,
                            ResourceType::Gold,
                        ] {
                            cost.conversions.push(PaymentConversion::unlimited(vec![ResourcePile::mood_tokens(1)], ResourcePile::of(*r, 1)));
                        }
                    },
                    0,
                ),
        ],
    )
}

fn economy() -> Vec<Advance> {
    advance_group(
        BARTERING,
        vec![
            Advance::builder(BARTERING, "todo")
                .with_advance_bonus(MoodToken)
                .with_unlocked_building(Market),
            trade_routes(),
            Advance::builder(
                CURRENCY,
                // also for Taxation
                "You may collect gold instead of food for Trade Routes",
            )
            .with_advance_bonus(CultureToken),
        ],
    )
}

fn trade_routes() -> AdvanceBuilder {
    Advance::builder(
        "Trade Routes",
        "At the beginning of your turn, you gain 1 food for every trade route you can make, to a maximum of 4. A trade route is made between one of your Settlers or Ships and a non-Angry enemy player city within 2 spaces (without counting through unrevealed Regions). Each Settler or Ship can only be paired with one enemy player city. Likewise, each enemy player city must be paired with a different Settler or Ship. In other words, to gain X food you must have at least X Units (Settlers or Ships), each paired with X different enemy cities.")
        .add_reward_request_listener(
            |event| &mut event.on_turn_start,
            0,
            |game, _player_index| {
                trade_route_reward(game).map(|(reward, _routes)| {
                    CustomPhaseRewardRequest {
                        reward,
                        name: "Collect trade routes reward".to_string(),
                    }
                })
            },
            |game, player_index, _player_name, p, selected| {
                let (reward, routes) =
                    trade_route_reward(game).expect("No trade route reward");
                assert!(reward.is_valid_payment(p), "Invalid payment"); // it's a gain
                gain_trade_route_reward(game, player_index, &routes, p, selected);
            },
        )
}

fn culture() -> Vec<Advance> {
    advance_group(
        "Arts",
        vec![Advance::builder("Arts", "todo")
            .with_advance_bonus(CultureToken)
            .with_unlocked_building(Obelisk)],
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
                .with_unlocked_building(Observatory),
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
