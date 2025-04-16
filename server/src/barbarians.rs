use crate::ability_initializer::AbilityInitializerSetup;
use crate::city::City;
use crate::collect::reset_collect_within_range_for_all;
use crate::combat::move_with_possible_combat;
use crate::consts::STACK_LIMIT;
use crate::content::advances::theocracy::cities_that_can_add_units;
use crate::content::builtin::Builtin;
use crate::content::persistent_events::{PositionRequest, ResourceRewardRequest, UnitTypeRequest};
use crate::game::Game;
use crate::incident::{BASE_EFFECT_PRIORITY, IncidentBuilder, IncidentFilter, play_base_effect};
use crate::map::Terrain;
use crate::movement::MoveUnits;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::{IncidentTarget, PersistentEvent, PersistentEvents};
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
    pub fn new() -> BarbariansEventState {
        BarbariansEventState {
            moved_units: Vec::new(),
            selected_position: None,
            move_units: false,
            must_reduce_mood: Vec::new(),
        }
    }
}

pub(crate) fn barbarians_bonus() -> Builtin {
    Builtin::builder("Barbarians bonus", "-")
        .add_resource_request(
            |event| &mut event.combat_end,
            105,
            |game, player_index, i| {
                if i.is_winner(player_index)
                    && game
                        .player(i.opponent(player_index))
                        .civilization
                        .is_barbarian()
                {
                    let sum = if i.captured_city(player_index, game) {
                        2
                    } else {
                        1
                    };
                    Some(ResourceRewardRequest::new(
                        PaymentOptions::sum(sum, &[ResourceType::Gold]),
                        "-".to_string(),
                    ))
                } else {
                    None
                }
            },
            |_game, s, _| {
                vec![format!(
                    "{} gained {} for winning a combat against the Barbarians",
                    s.player_name, s.choice
                )]
            },
        )
        .build()
}

pub(crate) fn barbarians_spawn(mut builder: IncidentBuilder) -> IncidentBuilder {
    builder = set_info(builder, "Barbarians spawn", |_, _, _| {});
    builder = add_barbarians_city(builder);
    builder = builder.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 1,
        |game, _player_index, _i| {
            let r = possible_barbarians_reinforcements(game);
            if r.is_empty() {
                game.add_info_log_item("Barbarians cannot reinforce");
            }
            let needed = 1..=1;
            Some(PositionRequest::new(
                r,
                needed,
                "Select a position for the additional Barbarian unit",
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
            IncidentFilter::new(IncidentTarget::ActivePlayer, BASE_EFFECT_PRIORITY, None)
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
        move |game, player_index, v| {
            if !filter(game, player_index, v) {
                return None;
            }

            let choices = get_barbarian_reinforcement_choices(game, get_barbarian_city(v));
            Some(UnitTypeRequest::new(
                choices,
                get_barbarians_player(game).index,
                "Select a unit to reinforce the barbarians",
            ))
        },
        move |game, s, v| {
            let position = get_barbarian_city2(v).expect("barbarians should exist");
            let units = Units::from_iter(vec![s.choice]);
            game.add_info_log_item(&format!("Barbarians reinforced with {units} at {position}",));
            game.player_mut(get_barbarians_player(game).index)
                .add_unit(position, s.choice);
        },
    )
}

pub(crate) fn barbarians_move(mut builder: IncidentBuilder) -> IncidentBuilder {
    builder = set_info(builder, "Barbarians move", |state, game, human| {
        if get_movable_units(game, human, state).is_empty() {
            game.add_info_log_item("Barbarians cannot move - will try to spawn a new city instead");
        } else {
            state.move_units = true;
        }
    });
    builder = add_barbarians_city(builder);
    for army in 0..18 {
        builder = builder
            .add_incident_position_request(
                IncidentTarget::ActivePlayer,
                BASE_EFFECT_PRIORITY + (army * 2) + 2,
                |game, player_index, i| {
                    let armies = get_movable_units(game, player_index, i.get_barbarian_state());
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
                |game, player_index, i| {
                    let state = i.barbarians.as_mut().expect("barbarians should exist");
                    if let Some(army) = state.selected_position {
                        let choices = barbarian_march_steps(
                            game,
                            game.player(player_index),
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
                |game, s, i| {
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
                    game.add_info_log_item(&format!(
                        "Barbarians move from {from} to {to}: {unit_types}"
                    ));
                    move_with_possible_combat(
                        game,
                        get_barbarians_player(game).index,
                        from,
                        &MoveUnits::new(units, to, None, ResourcePile::empty()),
                    );
                },
            );
    }
    builder.add_simple_incident_listener(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY,
        |game, player, _, i| {
            let s = i.get_barbarian_state();
            if s.move_units && get_movable_units(game, player, s).is_empty() {
                // after all moves are done
                reinforce_after_move(game, player);
                // clear movement restrictions
                game.player_mut(get_barbarians_player(game).index)
                    .end_turn();
            }
        },
    )
}

fn reinforce_after_move(game: &mut Game, player_index: usize) {
    let player = game.player(player_index);
    let barbarian = get_barbarians_player(game).index;
    let available = player.available_units().get(&UnitType::Infantry) as usize;

    let cities: Vec<Position> = player
        .cities
        .iter()
        .flat_map(|c| cities_in_range(game, |p| p.index == barbarian, c.position, 2))
        .unique()
        .filter(|&p| get_barbarians_player(game).get_units(p).len() < STACK_LIMIT)
        .take(available)
        .collect();
    for pos in cities {
        game.player_mut(barbarian).add_unit(pos, UnitType::Infantry);
        game.add_info_log_item(&format!("Barbarians spawned a new Infantry unit at {pos}",));
    }
}

fn get_movable_units(game: &Game, human: usize, state: &BarbariansEventState) -> Vec<Position> {
    let human = game.player(human);
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
            stack > 0 && !barbarian_march_steps(game, human, *pos, stack).is_empty()
        })
        .copied()
        .collect()
}

fn barbarian_march_steps(
    game: &Game,
    human: &Player,
    from: Position,
    stack_size: usize,
) -> Vec<Position> {
    let primary = cities_in_range(game, |p| p.index == human.index, from, 1);
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
    init: impl Fn(&mut BarbariansEventState, &mut Game, usize) + 'static + Clone + Sync + Send,
) -> IncidentBuilder {
    let name = event_name.to_string();
    builder.add_simple_incident_listener(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 200,
        move |game, player, _, i| {
            if play_base_effect(i) && i.barbarians.is_none() {
                game.add_info_log_item(&format!("Base effect: {name}"));
                let mut state = BarbariansEventState::new();
                init(&mut state, game, player);
                i.barbarians = Some(state);
            }
        },
    )
}

fn add_barbarians_city(builder: IncidentBuilder) -> IncidentBuilder {
    builder.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 100,
        move |game, player_index, i| {
            if i.get_barbarian_state().move_units {
                return None;
            }

            let choices = possible_barbarians_spawns(game, game.player(player_index));
            if choices.is_empty() {
                game.add_info_log_item("Barbarians cannot spawn a new city");
            }
            let needed = 1..=1;
            Some(PositionRequest::new(
                choices,
                needed,
                "Select a position for the new city and infantry unit",
            ))
        },
        move |game, s, _| {
            let pos = s.choice[0];
            game.add_info_log_item(&format!(
                "Barbarians spawned a new city and a new Infantry unit at {pos}"
            ));
            let b = get_barbarians_player(game).index;
            let p = game.player_mut(b);
            p.cities.push(City::new(b, pos));
            p.add_unit(pos, UnitType::Infantry);
            reset_collect_within_range_for_all(game, pos);
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
                && cities_in_range(game, |p| p == player, *pos, 1).is_empty()
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

fn get_barbarian_reinforcement_choices(game: &Game, pos: Option<Position>) -> Vec<UnitType> {
    let barbarian = get_barbarians_player(game);
    let Some(pos) = pos else {
        return vec![];
    };

    let possible = if barbarian
        .get_units(pos)
        .iter()
        .any(|u| u.unit_type == UnitType::Infantry)
    {
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
    game.map
        .get(pos)
        .is_some_and(|t| t.is_land() && !matches!(t, Terrain::Barren))
        && !units_present(game, pos)
        && cities_in_range(game, |p| p.index != player.index, pos, 2).is_empty()
}

fn units_present(game: &Game, pos: Position) -> bool {
    game.players
        .iter()
        .any(|p| p.units.iter().any(|u| u.position == pos))
}

fn steps_towards_land_range2_cites(game: &Game, player: &Player, start: Position) -> Vec<Position> {
    start
        .neighbors()
        .into_iter()
        .filter(|&middle| {
            game.map.is_land(middle)
                && player
                    .cities
                    .iter()
                    .any(|c| c.position.distance(start) == 2 && c.position.distance(middle) == 1)
        })
        .collect()
}

fn adjacent_to_cities(player: &Player, pos: Position) -> bool {
    player
        .cities
        .iter()
        .any(|c| c.position.neighbors().contains(&pos))
}

fn cities_in_range(
    game: &Game,
    player: impl Fn(&Player) -> bool,
    pos: Position,
    range: u32,
) -> Vec<Position> {
    game.players
        .iter()
        .filter(|p| player(p))
        .flat_map(|p| {
            p.cities
                .iter()
                .filter(|c| c.position.distance(pos) <= range)
                .map(|c| c.position)
        })
        .collect()
}

#[must_use]
pub(crate) fn get_barbarians_player(game: &Game) -> &Player {
    game.players
        .iter()
        .find(|p| p.civilization.is_barbarian())
        .expect("barbarians should exist")
}
