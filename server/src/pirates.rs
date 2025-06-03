use crate::ability_initializer::AbilityInitializerSetup;
use crate::barbarians;
use crate::city::MoodState;
use crate::content::ability::Ability;
use crate::content::persistent_events::{
    PaymentRequest, PositionRequest, ResourceRewardRequest, UnitsRequest,
};
use crate::game::Game;
use crate::incident::{BASE_EFFECT_PRIORITY, IncidentBuilder};
use crate::payment::{PaymentOptions, PaymentReason, ResourceReward};
use crate::player::{Player, gain_unit, remove_unit};
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::tactics_card::CombatRole;
use crate::unit::UnitType;
use itertools::Itertools;

pub(crate) fn pirates_round_bonus() -> Ability {
    Ability::builder("Pirates bonus", "-")
        .add_resource_request(
            |event| &mut event.combat_round_end,
            3,
            |game, player_index, r| {
                let c = &r.combat;
                if c.is_sea_battle(game)
                    && c.opponent(player_index) == get_pirates_player(game).index
                {
                    Some(ResourceRewardRequest::new(
                        ResourceReward::sum(r.hits(CombatRole::Attacker), &[ResourceType::Gold]),
                        "-".to_string(),
                    ))
                } else {
                    None
                }
            },
            |_game, s, _| {
                vec![format!(
                    "{} gained {} for destroying Pirate Ships",
                    s.player_name, s.choice
                )]
            },
        )
        .build()
}

pub(crate) fn pirates_bonus() -> Ability {
    Ability::builder(
        "Barbarians bonus",
        "Select a reward for fighting the Pirates",
    )
    .add_resource_request(
        |event| &mut event.combat_end,
        103,
        |game, player_index, i| {
            if i.opponent_player(player_index, game)
                .civilization
                .is_pirates()
            {
                Some(ResourceRewardRequest::new(
                    ResourceReward::tokens(1),
                    "Select a reward for fighting the Pirates".to_string(),
                ))
            } else {
                None
            }
        },
        |_game, s, _| {
            vec![format!(
                "{} gained {} for fighting the Pirates",
                s.player_name, s.choice
            )]
        },
    )
    .build()
}

pub(crate) fn pirates_spawn_and_raid(mut builder: IncidentBuilder) -> IncidentBuilder {
    builder = barbarians::set_info(builder, "Pirates spawn", |_, _, _| {});
    builder = remove_pirate_ships(builder);
    builder = place_pirate_ship(builder, BASE_EFFECT_PRIORITY + 4, true);
    builder = place_pirate_ship(builder, BASE_EFFECT_PRIORITY + 3, false);

    builder
        .add_incident_payment_request(
            IncidentTarget::AllPlayers,
            BASE_EFFECT_PRIORITY + 2,
            |game, player_index, i| {
                let player = game.player(player_index);
                if cities_with_adjacent_pirates(player, game).is_empty() {
                    return None;
                }

                if player.resources.amount() > 0 {
                    game.add_info_log_item(&format!(
                        "{player} must pay 1 resource or token to bribe the pirates",
                    ));
                    Some(vec![PaymentRequest::mandatory(
                        PaymentOptions::sum(
                            game.player(player_index),
                            PaymentReason::Incident,
                            1,
                            &ResourceType::all(),
                        ),
                        "Pay 1 Resource or token to bribe the pirates",
                    )])
                } else {
                    let state = i.get_barbarian_state();
                    state.must_reduce_mood.push(player_index);
                    None
                }
            },
            |c, s, _| {
                c.add_info_log_item(&format!("Pirates took {}", s.choice[0]));
            },
        )
        .add_incident_position_request(
            IncidentTarget::AllPlayers,
            BASE_EFFECT_PRIORITY + 1,
            |game, player_index, i| {
                if !i
                    .get_barbarian_state()
                    .must_reduce_mood
                    .contains(&player_index)
                {
                    return None;
                }

                let player = game.player(player_index);
                let choices = cities_with_adjacent_pirates(player, game)
                    .into_iter()
                    .filter(|&pos| !matches!(player.get_city(pos).mood_state, MoodState::Angry))
                    .collect_vec();
                if choices.is_empty() {
                    return None;
                }

                game.add_info_log_item(&format!(
                    "{player} must reduce Mood in a city adjacent to pirates",
                ));

                let needed = 1..=1;
                Some(PositionRequest::new(
                    choices,
                    needed,
                    "Select a city to reduce Mood",
                ))
            },
            |game, s, _| {
                let pos = s.choice[0];
                game.add_info_log_item(&format!(
                    "{} reduced Mood in the city at {}",
                    s.player_name, pos
                ));
                game.player_mut(s.player_index)
                    .get_city_mut(pos)
                    .decrease_mood_state();
            },
        )
}

fn remove_pirate_ships(builder: IncidentBuilder) -> IncidentBuilder {
    builder.add_units_request(
        |event| &mut event.incident,
        BASE_EFFECT_PRIORITY + 5,
        |game, player_index, i| {
            if !i.is_active(IncidentTarget::ActivePlayer, player_index) {
                return None;
            }

            let pirates = get_pirates_player(game);
            let pirate_ships = pirates
                .units
                .iter()
                .filter(|u| u.unit_type == UnitType::Ship)
                .map(|u| u.id)
                .collect();
            let needs_removal = 2_u8.saturating_sub(pirates.available_units().get(&UnitType::Ship));

            Some(UnitsRequest::new(
                pirates.index,
                pirate_ships,
                needs_removal..=needs_removal,
                "Select Pirate Ships to remove",
            ))
        },
        |game, s, _| {
            let pirates = get_pirates_player(game).index;
            game.add_info_log_item(&format!(
                "{} removed a Pirate Ships at {}",
                s.player_name,
                s.choice
                    .iter()
                    .map(|u| game.player(pirates).get_unit(*u).position.to_string())
                    .join(", ")
            ));
            for unit in &s.choice {
                remove_unit(pirates, *unit, game);
            }
        },
    )
}

fn place_pirate_ship(builder: IncidentBuilder, priority: i32, blockade: bool) -> IncidentBuilder {
    builder.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        priority,
        move |game, player_index, _i| {
            let pirates = get_pirates_player(game).index;
            let player = game.player(player_index);
            let mut sea_spaces = game
                .map
                .tiles
                .keys()
                .filter(|&pos| {
                    game.map.is_sea(*pos)
                        && game.players.iter().all(|p| {
                            p.index == pirates || p.units.iter().all(|u| u.position != *pos)
                        })
                })
                .copied()
                .collect_vec();

            if blockade {
                let adjacent = adjacent_sea(player);
                let blocking = sea_spaces
                    .iter()
                    .copied()
                    .filter(|&pos| adjacent.contains(&pos))
                    .collect_vec();
                if !blocking.is_empty() {
                    sea_spaces = blocking;
                }
            }

            if sea_spaces.is_empty() && blockade {
                // don't log this twice (blockade is only for first call)
                game.add_info_log_item("No valid positions for Pirate Ship");
            }

            let needed = 1..=1;
            Some(PositionRequest::new(
                sea_spaces,
                needed,
                "Select a position for the Pirate Ship",
            ))
        },
        |game, s, _| {
            let pirate = get_pirates_player(game).index;
            let pos = s.choice[0];
            game.add_info_log_item(&format!("Pirates spawned a Pirate Ship at {pos}"));
            gain_unit(pirate, pos, UnitType::Ship, game);
        },
    )
}

fn adjacent_sea(player: &Player) -> Vec<Position> {
    player
        .cities
        .iter()
        .map(|c| c.position)
        .flat_map(|c| c.neighbors())
        .collect_vec()
}

fn cities_with_adjacent_pirates(player: &Player, game: &Game) -> Vec<Position> {
    let pirates = get_pirates_player(game);
    player
        .cities
        .iter()
        .filter(|c| {
            c.position.neighbors().iter().any(|p| {
                pirates
                    .get_units(*p)
                    .iter()
                    .any(|u| u.unit_type == UnitType::Ship && u.position == *p)
            })
        })
        .map(|c| c.position)
        .collect()
}

#[must_use]
pub(crate) fn get_pirates_player(game: &Game) -> &Player {
    game.players
        .iter()
        .find(|p| p.civilization.is_pirates())
        .expect("pirates should exist")
}
