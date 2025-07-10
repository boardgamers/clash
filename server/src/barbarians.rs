use crate::ability_initializer::{AbilityInitializerSetup, SelectedMultiChoice};
use crate::city::{City, gain_city};
use crate::combat::move_with_possible_combat;
use crate::consts::STACK_LIMIT;
use crate::content::ability::Ability;
use crate::content::advances::theocracy::cities_that_can_add_units;
use crate::content::persistent_events::{
    PersistentEventType, PositionRequest, ResourceRewardRequest, UnitTypeRequest,
};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::incident::{BASE_EFFECT_PRIORITY, IncidentBuilder, IncidentFilter, play_base_effect};
use crate::map::Terrain;
use crate::movement::MoveUnits;
use crate::player::{Player, end_turn, gain_unit};
use crate::player_events::{IncidentInfo, IncidentTarget, PersistentEvent, PersistentEvents};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::unit::{UnitType, Units};
use crate::utils;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct BarbariansMoveRequest {
    pub from: Position,
    pub to: Position,
    pub player: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct BarbariansEventState {
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
    pub move_units: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub moved_units: Vec<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub must_reduce_mood: Vec<usize>,
}

impl Default for BarbariansEventState {
    fn default() -> Self {
        Self::new()
    }
}

impl BarbariansEventState {
    #[must_use]
    pub fn new() -> BarbariansEventState {
        BarbariansEventState {
            moved_units: Vec::new(),
            selected_position: None,
            move_units: false,
            must_reduce_mood: Vec::new(),
        }
    }
}

pub(crate) fn barbarians_bonus() -> Ability {
    Ability::builder("Barbarian battle", "-")
        .add_resource_request(
            |event| &mut event.combat_end,
            105,
            |game, p, i| {
                let player = p.index;
                if i.is_winner(player)
                    && i.opponent_player(player, game).civilization.is_barbarian()
                {
                    let mut sum = 0;
                    if i.captured_city(player).is_some() {
                        sum += 1;
                    }
                    if i.opponent(player).losses.amount() > 0 {
                        sum += 1;
                    }

                    Some(ResourceRewardRequest::new(
                        p.reward_options().sum(sum, &[ResourceType::Gold]),
                        "-".to_string(),
                    ))
                } else {
                    None
                }
            },
        )
        .build()
}

pub(crate) fn barbarians_spawn(mut builder: IncidentBuilder) -> IncidentBuilder {
    let event_name = "Barbarians spawn";
    builder = set_info(builder, event_name, |_, _, _| {});
    builder = add_barbarians_city(builder, event_name);
    builder = builder.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 1,
        |game, p, _i| {
            let r = possible_barbarians_reinforcements(game);
            if r.is_empty() {
                p.log(game, "Barbarians cannot reinforce");
            }
            let needed = 1..=1;
            Some(PositionRequest::new(
                r,
                needed,
                "Barbarians spawn: select a position for the additional Barbarian unit",
            ))
        },
        |_game, s, i| {
            let mut state = BarbariansEventState::new();
            state.selected_position = Some(s.choice[0]);
            i.barbarians = Some(state);
        },
    );
    barbarian_reinforcement(
        builder,
        |e| &mut e.incident,
        BASE_EFFECT_PRIORITY,
        |game, p, i| {
            IncidentFilter::new(
                IncidentTarget::ActivePlayer,
                BASE_EFFECT_PRIORITY,
                None,
                None,
            )
            .is_active(game, i, p)
        },
        |i| {
            i.barbarians
                .as_ref()
                .expect("barbarians should exist")
                .selected_position
        },
    )
}

pub(crate) fn barbarian_reinforcement<E, S, V>(
    b: S,
    event: E,
    prio: i32,
    filter: impl Fn(&Game, usize, &V) -> bool + 'static + Clone + Sync + Send,
    get_barbarian_city: impl Fn(&V) -> Option<Position> + 'static + Clone + Sync + Send,
) -> S
where
    E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
    S: AbilityInitializerSetup,
    V: Clone + PartialEq,
{
    let get_barbarian_city2 = get_barbarian_city.clone();
    b.add_unit_type_request(
        event,
        prio,
        move |game, p, v| {
            if !filter(game, p.index, v) {
                return None;
            }

            let city = get_barbarian_city(v)?;
            let choices = get_barbarian_reinforcement_choices(game, city);
            Some(UnitTypeRequest::new(
                choices,
                get_barbarians_player(game).index,
                &format!("Select a unit to reinforce the barbarians at {city}"),
            ))
        },
        move |game, s, v| {
            let position = get_barbarian_city2(v).expect("barbarians should exist");
            gain_unit(
                game,
                &get_barbarians_event_player(game, &s.origin),
                position,
                s.choice,
            );
        },
    )
}

pub(crate) fn on_stop_barbarian_movement(game: &mut Game, movable: Vec<Position>) {
    let old_movable = movable.clone();
    match game.trigger_persistent_event(
        &game.human_players(0),
        |events| &mut events.stop_barbarian_movement,
        movable,
        PersistentEventType::StopBarbarianMovement,
    ) {
        None => (),
        Some(movable) => {
            if movable == old_movable {
                // nothing changed, so we can skip the rest
                return;
            }

            let mut event_state = game.events.pop().expect("event should exist");
            if let PersistentEventType::Incident(i) = &mut event_state.event_type {
                let state = i.get_barbarian_state();
                for pos in old_movable {
                    if !movable.contains(&pos) {
                        // if the position was not selected, it means the unit cannot move
                        let units = get_barbarians_player(game)
                            .get_units(pos)
                            .iter()
                            .map(|u| u.id)
                            .collect_vec();
                        state.moved_units.extend(units);
                    }
                }
                game.events.push(event_state);
            } else {
                panic!(
                    "StopBarbarianMovement should only be triggered from an Incident, not {:?}",
                    game.current_event().event_type
                )
            }
        }
    }
}

pub(crate) fn barbarians_move(mut builder: IncidentBuilder) -> IncidentBuilder {
    let event_name = "Barbarians move";
    builder = set_info(builder, event_name, |state, game, human| {
        if get_movable_units(game, human.index, state).is_empty() {
            human.log(
                game,
                "Barbarians cannot move - will try to spawn a new city instead",
            );
        } else {
            state.move_units = true;
        }
    });
    builder = add_barbarians_city(builder, event_name).add_simple_incident_listener(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 99,
        |game, _, i| {
            let movable = get_movable_units(game, i.active_player, i.get_barbarian_state());
            if movable.is_empty() {
                return;
            }

            on_stop_barbarian_movement(game, movable);
        },
    );

    for army in 0..18 {
        builder = builder
            .add_incident_position_request(
                IncidentTarget::ActivePlayer,
                BASE_EFFECT_PRIORITY + (army * 2) + 2,
                |game, p, i| {
                    if !i.get_barbarian_state().move_units {
                        return None;
                    }

                    let armies = get_movable_units(game, p.index, i.get_barbarian_state());
                    let needed = 1..=1;
                    Some(PositionRequest::new(
                        armies,
                        needed,
                        "Select a Barbarian Army to move next",
                    ))
                },
                |_game, s, i| {
                    i.get_barbarian_state().selected_position = Some(s.choice[0]);
                },
            )
            .add_incident_position_request(
                IncidentTarget::ActivePlayer,
                BASE_EFFECT_PRIORITY + (army * 2) + 1,
                |game, p, i| {
                    let state = i.barbarians.as_mut().expect("barbarians should exist");
                    if let Some(army) = state.selected_position {
                        let choices = barbarian_march_steps(
                            game,
                            game.player(p.index),
                            army,
                            0, // stack size was already checked in last step
                        );

                        let needed = 1..=1;
                        Some(PositionRequest::new(
                            choices,
                            needed,
                            "Select a position to move the Barbarian Army",
                        ))
                    } else {
                        None
                    }
                },
                execute_barbarian_move,
            );
    }
    builder.add_simple_incident_listener(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY,
        |game, player, i| {
            let s = i.get_barbarian_state();
            if s.move_units && get_movable_units(game, player.index, s).is_empty() {
                // after all moves are done
                reinforce_after_move(game, player);
                // clear movement restrictions
                end_turn(game, get_barbarians_player(game).index);
            }
        },
    )
}

fn execute_barbarian_move(
    game: &mut Game,
    s: &SelectedMultiChoice<Vec<Position>>,
    i: &mut IncidentInfo,
) {
    let state = i.barbarians.as_mut().expect("barbarians should exist");
    let from = state
        .selected_position
        .take()
        .expect("selected position should exist");
    let to = s.choice[0];
    let ids = get_barbarians_player(game).get_units(from);
    let units: Vec<u32> = ids.iter().map(|u| u.id).collect();
    state.moved_units.extend(units.iter());
    let unit_types = ids.iter().map(|u| u.unit_type).collect::<Units>();
    s.log(
        game,
        &format!(
            "Barbarians move from {from} to {to}: {}",
            unit_types.to_string(None)
        ),
    );
    move_with_possible_combat(
        game,
        get_barbarians_player(game).index,
        &MoveUnits::new(units, to, None, ResourcePile::empty()),
    );
}

fn reinforce_after_move(game: &mut Game, player: &EventPlayer) {
    let p = game.player(player.index);
    let barbarian = &get_barbarians_event_player(game, &player.origin);
    let available = p.available_units().get(&UnitType::Infantry) as usize;

    let cities: Vec<Position> = p
        .cities
        .iter()
        .flat_map(|c| cities_in_land_range(game, |p| p.index == barbarian.index, c.position, 2))
        .unique()
        .filter(|&p| get_barbarians_player(game).get_units(p).len() < STACK_LIMIT)
        .take(available)
        .collect();
    for pos in cities {
        gain_unit(game, barbarian, pos, UnitType::Infantry);
    }
}

pub(crate) fn get_movable_units(
    game: &Game,
    target_player: usize,
    state: &BarbariansEventState,
) -> Vec<Position> {
    let target = game.player(target_player);
    let barbarian = get_barbarians_player(game);

    game.map
        .tiles
        .keys()
        .filter(|&pos| {
            // Check to see if there are any Barbarian Armies within 2 spaces of your cities.
            let stack = barbarian
                .get_units(*pos)
                .iter()
                .filter(|u| !state.moved_units.contains(&u.id))
                .count();
            stack > 0 && !barbarian_march_steps(game, target, *pos, stack).is_empty()
        })
        .sorted()
        .copied()
        .collect()
}

fn barbarian_march_steps(
    game: &Game,
    human: &Player,
    from: Position,
    stack_size: usize,
) -> Vec<Position> {
    let primary = cities_in_land_range(game, |p| p.index == human.index, from, 1);
    if !primary.is_empty() {
        return primary;
    }

    let barbarian = get_barbarians_player(game);
    steps_towards_land_range2_cites(game, human, from)
        .into_iter()
        .filter(|&p| {
            let units = barbarian.get_units(p);
            stack_size + units.len() <= STACK_LIMIT
        })
        .collect()
}

pub(crate) fn set_info(
    builder: IncidentBuilder,
    event_name: &str,
    init: impl Fn(&mut BarbariansEventState, &mut Game, &EventPlayer) + 'static + Clone + Sync + Send,
) -> IncidentBuilder {
    let name = event_name.to_string();
    builder.add_simple_incident_listener(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 200,
        move |game, player, i| {
            if play_base_effect(i) && i.barbarians.is_none() {
                player.log(game, &format!("Base effect: {name}"));
                let mut state = BarbariansEventState::new();
                init(&mut state, game, player);
                i.barbarians = Some(state);
            }
        },
    )
}

fn add_barbarians_city(builder: IncidentBuilder, event_name: &'static str) -> IncidentBuilder {
    builder.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 100,
        move |game, p, i| {
            if i.get_barbarian_state().move_units {
                return None;
            }

            let choices = possible_barbarians_spawns(game, game.player(p.index));
            if choices.is_empty() {
                p.log(game, "Barbarians cannot spawn a new city");
            }
            let needed = 1..=1;
            Some(PositionRequest::new(
                choices,
                needed,
                &format!("{event_name}: Select a position for the new city and infantry unit"),
            ))
        },
        move |game, s, _| {
            let pos = s.choice[0];
            let player = &get_barbarians_event_player(game, &s.origin);
            gain_city(game, player, City::new(player.index, pos));
            gain_unit(game, player, pos, UnitType::Infantry);
        },
    )
}

fn possible_barbarians_spawns(game: &Game, player: &Player) -> Vec<Position> {
    let primary: Vec<Position> = game
        .map
        .tiles
        .keys()
        .filter(|&pos| {
            is_base_barbarian_spawn_pos(game, *pos, player)
                && cities_in_land_range(game, |p| p.index == player.index, *pos, 1).is_empty()
                && !steps_towards_land_range2_cites(game, player, *pos).is_empty()
        })
        .copied()
        .collect();

    if !primary.is_empty() {
        return primary;
    }

    let secondary: Vec<Position> = game
        .map
        .tiles
        .keys()
        .filter(|&pos| {
            is_base_barbarian_spawn_pos(game, *pos, player) && adjacent_to_cities(player, *pos)
        })
        .copied()
        .collect();

    secondary
}

fn possible_barbarians_reinforcements(game: &Game) -> Vec<Position> {
    let barbarian = get_barbarians_player(game);
    let avail = barbarian.available_units();
    if !barbarian_fighters().iter().any(|u| avail.has_unit(u)) {
        return vec![];
    }
    cities_that_can_add_units(barbarian)
}

fn get_barbarian_reinforcement_choices(game: &Game, pos: Position) -> Vec<UnitType> {
    let barbarian = get_barbarians_player(game);

    let possible = if barbarian.get_units(pos).iter().any(|u| u.is_infantry()) {
        barbarian_fighters()
    } else {
        vec![UnitType::Infantry]
    };
    possible
        .iter()
        .filter(|u| barbarian.available_units().has_unit(u))
        .copied()
        .collect()
}

fn barbarian_fighters() -> Vec<UnitType> {
    vec![UnitType::Infantry, UnitType::Cavalry, UnitType::Elephant]
}

fn is_base_barbarian_spawn_pos(game: &Game, pos: Position, player: &Player) -> bool {
    game.map.get(pos).is_some_and(|t| {
        t.is_land() && !matches!(t, Terrain::Barren) && !matches!(t, Terrain::Exhausted(_))
    }) && !anything_present(game, pos)
        && cities_in_land_range(game, |p| p.index != player.index, pos, 2).is_empty()
}

fn anything_present(game: &Game, pos: Position) -> bool {
    game.players
        .iter()
        .any(|p| p.units.iter().any(|u| u.position == pos))
        || game
            .players
            .iter()
            .any(|p| p.cities.iter().any(|c| c.position == pos))
}

fn steps_towards_land_range2_cites(game: &Game, player: &Player, start: Position) -> Vec<Position> {
    let mut steps = player
        .cities
        .iter()
        .flat_map(|c| land_steps_towards_range2(game, start, c.position))
        .collect_vec();
    steps.sort();
    steps.dedup();
    steps
}

fn land_steps_towards_range2(game: &Game, start: Position, end: Position) -> Vec<Position> {
    start
        .neighbors()
        .into_iter()
        .filter(|&middle| {
            game.map.is_land(middle) && end.distance(start) == 2 && end.distance(middle) == 1
        })
        .collect()
}

fn adjacent_to_cities(player: &Player, pos: Position) -> bool {
    player
        .cities
        .iter()
        .any(|c| c.position.neighbors().contains(&pos))
}

fn cities_in_land_range(
    game: &Game,
    player: impl Fn(&Player) -> bool,
    start: Position,
    range: u32,
) -> Vec<Position> {
    assert!((1..=2).contains(&range), "cities_in_land_range only supports range 1 or 2, got {range}");

    game.players
        .iter()
        .filter(|p| player(p))
        .flat_map(|p| {
            p.cities
                .iter()
                .filter(|c| {
                    let end = c.position;
                    let d = end.distance(start);
                    d <= range
                        && (d == 1 || !land_steps_towards_range2(game, start, end).is_empty())
                })
                .map(|c| c.position)
        })
        .collect()
}

#[must_use]
pub(crate) fn get_barbarians_event_player(game: &Game, origin: &EventOrigin) -> EventPlayer {
    let player = get_barbarians_player(game);
    EventPlayer::new(player.index, player.get_name(), origin.clone())
}

///
/// Returns the player controlling the barbarians.
///
/// # Panics if the barbarians player does not exist.
#[must_use]
pub fn get_barbarians_player(game: &Game) -> &Player {
    game.players
        .iter()
        .find(|p| p.civilization.is_barbarian())
        .expect("barbarians should exist")
}
