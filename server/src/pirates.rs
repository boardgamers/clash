use crate::ability_initializer::AbilityInitializerSetup;
use crate::barbarians;
use crate::city::{MoodState, decrease_city_mood};
use crate::content::ability::Ability;
use crate::content::persistent_events::{
    PaymentRequest, PositionRequest, ResourceRewardRequest, UnitsRequest,
};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::incident::{BASE_EFFECT_PRIORITY, IncidentBuilder};
use crate::player::{Player, gain_unit, remove_unit};
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::tactics_card::CombatRole;
use crate::unit::UnitType;
use itertools::Itertools;

pub(crate) fn pirates_round_bonus() -> Ability {
    Ability::builder("Pirate ship destroyed", "-")
        .add_resource_request(
            |event| &mut event.combat_round_end,
            3,
            |game, p, r| {
                let c = &r.combat;
                if c.is_sea_battle(game) && c.opponent(p.index) == get_pirates_player(game).index {
                    Some(ResourceRewardRequest::new(
                        p.reward_options()
                            .sum(r.hits(CombatRole::Attacker), &[ResourceType::Gold]),
                        "-".to_string(),
                    ))
                } else {
                    None
                }
            },
        )
        .build()
}

pub(crate) fn pirates_bonus() -> Ability {
    Ability::builder("Pirates battle", "Select a reward for fighting the Pirates")
        .add_resource_request(
            |event| &mut event.combat_end,
            103,
            |game, p, i| {
                if i.opponent_player(p.index, game).civilization.is_pirates() {
                    Some(ResourceRewardRequest::new(
                        p.reward_options().tokens(1),
                        "Select a reward for fighting the Pirates".to_string(),
                    ))
                } else {
                    None
                }
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
            |game, p, i| {
                let player = p.get(game);
                if cities_with_adjacent_pirates(player, game).is_empty() {
                    return None;
                }

                if player.resources.amount() > 0 {
                    p.log(game, "Must pay 1 resource or token to bribe the pirates");
                    Some(vec![PaymentRequest::mandatory(
                        p.payment_options()
                            .sum(p.get(game), 1, &ResourceType::all()),
                        "Pay 1 Resource or token to bribe the pirates",
                    )])
                } else {
                    let state = i.get_barbarian_state();
                    state.must_reduce_mood.push(p.index);
                    None
                }
            },
            |game, s, _| {
                s.log(game, &format!("Pirates took {}", s.choice[0]));
            },
        )
        .add_incident_position_request(
            IncidentTarget::AllPlayers,
            BASE_EFFECT_PRIORITY + 1,
            |game, p, i| {
                if !i.get_barbarian_state().must_reduce_mood.contains(&p.index) {
                    return None;
                }

                let player = p.get(game);
                let choices = cities_with_adjacent_pirates(player, game)
                    .into_iter()
                    .filter(|&pos| !matches!(player.get_city(pos).mood_state, MoodState::Angry))
                    .collect_vec();
                if choices.is_empty() {
                    return None;
                }

                p.log(game, "Must reduce Mood in a city adjacent to pirates");
                let needed = 1..=1;
                Some(PositionRequest::new(
                    choices,
                    needed,
                    "Select a city to reduce Mood",
                ))
            },
            |game, s, _| {
                decrease_city_mood(game, s.choice[0], &s.origin);
            },
        )
}

fn remove_pirate_ships(builder: IncidentBuilder) -> IncidentBuilder {
    builder.add_units_request(
        |event| &mut event.incident,
        BASE_EFFECT_PRIORITY + 5,
        |game, p, i| {
            if !i.is_active_ignoring_protection(IncidentTarget::ActivePlayer, p.index) {
                return None;
            }

            let pirates = get_pirates_player(game);
            let pirate_ships = pirates
                .units
                .iter()
                .filter(|u| u.is_ship())
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
            s.log(
                game,
                &format!(
                    "Removed a Pirate Ships at {}",
                    s.choice
                        .iter()
                        .map(|u| game.player(pirates).get_unit(*u).position.to_string())
                        .join(", ")
                ),
            );
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
        move |game, p, _i| {
            let pirates = get_pirates_player(game).index;
            let player = p.get(game);
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
                p.log(game, "No valid positions for Pirate Ship");
            }

            let needed = 1..=1;
            Some(PositionRequest::new(
                sea_spaces,
                needed,
                "Select a position for the Pirate Ship",
            ))
        },
        |game, s, _| {
            gain_unit(
                game,
                &get_pirates_event_player(game, &s.origin),
                s.choice[0],
                UnitType::Ship,
            );
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
                    .any(|u| u.is_ship() && u.position == *p)
            })
        })
        .map(|c| c.position)
        .collect()
}

#[must_use]
pub(crate) fn get_pirates_event_player(game: &Game, origin: &EventOrigin) -> EventPlayer {
    let player = get_pirates_player(game);
    EventPlayer::new(player.index, origin.clone())
}

///
/// Returns the player that represents the Pirates civilization.
///
/// # Panics
/// If there is no player with the Pirates civilization in the game.
#[must_use]
pub fn get_pirates_player(game: &Game) -> &Player {
    game.players
        .iter()
        .find(|p| p.civilization.is_pirates())
        .expect("pirates should exist")
}
