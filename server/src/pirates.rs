use crate::ability_initializer::AbilityInitializerSetup;
use crate::barbarians;
use crate::city::MoodState;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::{
    new_position_request, PaymentRequest, ResourceRewardRequest, UnitsRequest,
};
use crate::game::Game;
use crate::incident::{IncidentBuilder, BASE_EFFECT_PRIORITY};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::tactics_card::CombatRole;
use crate::unit::UnitType;
use itertools::Itertools;

pub(crate) fn pirates_round_bonus() -> Builtin {
    Builtin::builder("Pirates bonus", "-")
        .add_resource_request(
            |event| &mut event.on_combat_round_end,
            3,
            |game, player_index, r| {
                let c = &r.combat;
                if c.is_sea_battle(game)
                    && c.opponent(player_index) == get_pirates_player(game).index
                {
                    let hits = r.casualties(CombatRole::Defender).fighters as u32;
                    Some(ResourceRewardRequest::new(
                        PaymentOptions::sum(hits, &[ResourceType::Gold]),
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

pub(crate) fn pirates_bonus() -> Builtin {
    Builtin::builder(
        "Barbarians bonus",
        "Select a reward for fighting the Pirates",
    )
    .add_resource_request(
        |event| &mut event.on_combat_end,
        3,
        |game, player_index, i| {
            if game
                .get_player(i.opponent(player_index))
                .civilization
                .is_pirates()
            {
                Some(ResourceRewardRequest::new(
                    PaymentOptions::tokens(1),
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
            |game, player_index, _i| {
                let player = game.get_player(player_index);
                if cities_with_adjacent_pirates(player, game).is_empty() {
                    return None;
                }

                if player.resources.amount() > 0 {
                    game.add_info_log_item(&format!(
                        "{} must pay 1 resource or token to bribe the pirates",
                        player.get_name()
                    ));
                    Some(vec![PaymentRequest::new(
                        PaymentOptions::sum(1, &ResourceType::all()),
                        "Pay 1 Resource or token to bribe the pirates",
                        false,
                    )])
                } else {
                    let state = barbarians::get_barbarian_state_mut(game);
                    state.must_reduce_mood.push(player_index);
                    None
                }
            },
            |c, s| {
                c.add_info_log_item(&format!("Pirates took {}", s.choice[0]));
            },
        )
        .add_incident_position_request(
            IncidentTarget::AllPlayers,
            BASE_EFFECT_PRIORITY + 1,
            |game, player_index, _i| {
                if !barbarians::get_barbarian_state(game)
                    .must_reduce_mood
                    .contains(&player_index)
                {
                    return None;
                }

                let player = game.get_player(player_index);
                let choices = cities_with_adjacent_pirates(player, game)
                    .into_iter()
                    .filter(|&pos| !matches!(player.get_city(pos).mood_state, MoodState::Angry))
                    .collect_vec();
                if choices.is_empty() {
                    return None;
                }

                game.add_info_log_item(&format!(
                    "{} must reduce Mood in a city adjacent to pirates",
                    player.get_name()
                ));

                Some(new_position_request(
                    choices,
                    1..=1,
                    "Select a city to reduce Mood",
                ))
            },
            |game, s| {
                let pos = s.choice[0];
                game.add_info_log_item(&format!(
                    "{} reduced Mood in the city at {}",
                    s.player_name, pos
                ));
                game.get_player_mut(s.player_index)
                    .get_city_mut(pos)
                    .decrease_mood_state();
            },
        )
}

fn remove_pirate_ships(builder: IncidentBuilder) -> IncidentBuilder {
    builder.add_units_request(
        |event| &mut event.on_incident,
        BASE_EFFECT_PRIORITY + 5,
        |game, player_index, i| {
            if !i.is_active(IncidentTarget::ActivePlayer, player_index, game) {
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
                    .map(|u| game.get_player(pirates).get_unit(*u).position.to_string())
                    .join(", ")
            ));
            for unit in &s.choice {
                game.get_player_mut(pirates).remove_unit(*unit);
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
            let player = game.get_player(player_index);
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

            Some(new_position_request(
                sea_spaces,
                1..=1,
                "Select a position for the Pirate Ship",
            ))
        },
        |game, s| {
            let pirate = get_pirates_player(game).index;
            let pos = s.choice[0];
            game.add_info_log_item(&format!("Pirates spawned a Pirate Ship at {pos}"));
            game.get_player_mut(pirate).add_unit(pos, UnitType::Ship);
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
