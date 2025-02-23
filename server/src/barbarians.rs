use crate::ability_initializer::AbilityInitializerSetup;
use crate::consts::STACK_LIMIT;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::{
    PositionRequest, ResourceRewardRequest, UnitTypeRequest,
};
use crate::game::Game;
use crate::incident::{IncidentBuilder, BASE_EFFECT_PRIORITY};
use crate::map::Terrain;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::unit::{UnitType, Units};
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
    pub move_units: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub moved_units: Vec<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_request: Option<BarbariansMoveRequest>,
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
            move_request: None,
            move_units: false,
        }
    }
}

pub(crate) fn barbarians_bonus() -> Builtin {
    Builtin::builder("Barbarians bonus", "-")
        .add_resource_request(
            |event| &mut event.on_combat_end,
            5,
            |game, player_index, i| {
                if i.is_winner(player_index)
                    && !game.get_player(i.opponent(player_index)).is_human()
                {
                    let sum = if i.captured_city(player_index, game) {
                        2
                    } else {
                        1
                    };
                    Some(ResourceRewardRequest {
                        reward: PaymentOptions::sum(sum, &[ResourceType::Gold]),
                        name: "-".to_string(),
                    })
                } else {
                    None
                }
            },
            |_game, _player_index, player_name, resource, _selected| {
                format!(
                    "{player_name} gained {resource} for winning a combat against the Barbarians"
                )
            },
        )
        .build()
}

pub(crate) fn barbarians_spawn(mut builder: IncidentBuilder) -> IncidentBuilder {
    builder = set_info(builder, "Barbarians spawn", |_, _, _| {});
    builder = add_barbarians_city(builder);
    builder
        .add_incident_position_request(
            IncidentTarget::ActivePlayer,
            BASE_EFFECT_PRIORITY + 1,
            |game, _player_index, _i| {
                Some(PositionRequest::new(
                    possible_barbarians_reinforcements(game),
                    Some("Select a position for the additional Barbarian unit".to_string()),
                ))
            },
            |c, _game, pos| {
                let mut state = BarbariansEventState::new();
                state.selected_position = Some(*pos);
                c.update_barbarian_info(state);
            },
        )
        .add_incident_unit_request(
            IncidentTarget::ActivePlayer,
            BASE_EFFECT_PRIORITY,
            |game, _player_index, _i| {
                let choices = get_barbarian_reinforcement_choices(game);
                Some(UnitTypeRequest::new(
                    choices,
                    get_barbarians_player(game),
                    Some("Select a unit to reinforce the barbarians".to_string()),
                ))
            },
            |c, game, unit| {
                let position = get_barbarian_state(game)
                    .selected_position
                    .expect("selected position should exist");
                let units = Units::from_iter(vec![*unit]);
                c.add_info_log_item(&format!("Barbarians reinforced with {units} at {position}",));
                c.gain_unit(get_barbarians_player(game), *unit, position);
            },
        )
}

pub(crate) fn barbarians_move(mut builder: IncidentBuilder) -> IncidentBuilder {
    builder = set_info(builder, "Barbarians move", |state, game, human| {
        if !get_movable_units(game, human, state).is_empty() {
            state.move_units = true;
        }
    });
    builder = add_barbarians_city(builder);
    for army in 0..18 {
        builder = builder
            .add_incident_position_request(
                IncidentTarget::ActivePlayer,
                BASE_EFFECT_PRIORITY + (army * 2) + 2,
                |game, player_index, _i| {
                    let armies = get_movable_units(game, player_index, &get_barbarian_state(game));
                    Some(PositionRequest::new(
                        armies,
                        Some("Select a Barbarian Army to move next".to_string()),
                    ))
                },
                |c, game, pos| {
                    let mut state = get_barbarian_state(game);
                    state.selected_position = Some(*pos);
                    c.update_barbarian_info(state);
                },
            )
            .add_incident_position_request(
                IncidentTarget::ActivePlayer,
                BASE_EFFECT_PRIORITY + (army * 2) + 1,
                |game, player_index, _i| {
                    let state = game
                        .current_custom_phase_mut()
                        .barbarians
                        .as_mut()
                        .expect("barbarians should exist");
                    if let Some(army) = state.selected_position {
                        let choices = barbarian_march_steps(
                            game,
                            game.get_player(player_index),
                            army,
                            0, // stack size was already checked in last step
                        );

                        Some(PositionRequest::new(
                            choices,
                            Some("Select a position to move the Barbarian Army".to_string()),
                        ))
                    } else {
                        None
                    }
                },
                |c, game, pos| {
                    let mut state = get_barbarian_state(game);
                    let src = state
                        .selected_position
                        .take()
                        .expect("selected position should exist");
                    state.move_request = Some(BarbariansMoveRequest {
                        from: src,
                        to: *pos,
                        player: get_barbarians_player(game),
                    });
                    c.update_barbarian_info(state);
                },
            );
    }
    builder.add_incident_listener(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY,
        |game, p, _i| {
            let s = get_barbarian_state(game);
            if s.move_units && get_movable_units(game, p.player, &s).is_empty() {
                // after all moves are done
                reinforce_after_move(game, p.player);
                // clear movement restrictions
                game.get_player_mut(get_barbarians_player(game)).end_turn();
            }
        },
    )
}

fn reinforce_after_move(game: &mut Game, player_index: usize) {
    let player = game.get_player(player_index);
    let barbarian = get_barbarians_player(game);
    let available = player.available_units().get(&UnitType::Infantry) as usize;

    let cities: Vec<Position> = player
        .cities
        .iter()
        .flat_map(|c| cities_in_range(game, |p| p.index == barbarian, c.position, 2))
        .unique()
        .filter(|&p| game.get_player(barbarian).get_units(p).len() < STACK_LIMIT)
        .take(available)
        .collect();
    for pos in cities {
        game.get_player_mut(barbarian)
            .add_unit(pos, UnitType::Infantry);
        game.add_info_log_item(&format!("Barbarians spawned a new Infantry unit at {pos}",));
    }
}

fn get_movable_units(game: &Game, human: usize, state: &BarbariansEventState) -> Vec<Position> {
    let human = game.get_player(human);
    let barbarian = game.get_player(get_barbarians_player(game));

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

    let barbarian = game.get_player(get_barbarians_player(game));
    steps_towards_land_range2_cites(game, human, from)
        .into_iter()
        .filter(|&p| {
            let units = barbarian.get_units(p);
            stack_size + units.len() <= STACK_LIMIT
        })
        .collect()
}

fn set_info(
    builder: IncidentBuilder,
    event_name: &str,
    init: impl Fn(&mut BarbariansEventState, &Game, usize) + 'static + Clone,
) -> IncidentBuilder {
    let name = event_name.to_string();
    builder.add_incident_listener(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 200,
        move |game, p, _i| {
            if game.current_custom_phase().barbarians.is_none() {
                let mut state = BarbariansEventState::new();
                init(&mut state, game, p.player);
                game.current_custom_phase_mut().barbarians = Some(state);
                game.add_to_last_log_item(&name);
            }
        },
    )
}

fn add_barbarians_city(builder: IncidentBuilder) -> IncidentBuilder {
    builder.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY + 100,
        move |game, player_index, _i| {
            (!get_barbarian_state(game).move_units).then_some(PositionRequest::new(
                possible_barbarians_spawns(game, game.get_player(player_index)),
                Some("Select a position for the new city and infantry unit".to_string()),
            ))
        },
        move |c, game, pos| {
            c.add_info_log_item(&format!(
                "Barbarians spawned a new city and a new Infantry unit at {pos}",
            ));
            let b = get_barbarians_player(game);
            c.gain_city(b, *pos);
            c.gain_unit(b, UnitType::Infantry, *pos);
        },
    )
}

fn get_barbarian_state(game: &Game) -> BarbariansEventState {
    game.current_custom_phase()
        .barbarians
        .as_ref()
        .expect("barbarians should exist")
        .clone()
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
    let barbarian = game.get_player(get_barbarians_player(game));
    let avail = barbarian.available_units();
    if !barbarian_fighters().iter().any(|u| avail.has_unit(u)) {
        return vec![];
    }
    barbarian
        .cities
        .iter()
        .filter(|c| barbarian.get_units(c.position).len() < STACK_LIMIT)
        .map(|c| c.position)
        .collect()
}

fn get_barbarian_reinforcement_choices(game: &Game) -> Vec<UnitType> {
    let barbarian = game.get_player(get_barbarians_player(game));
    let pos = get_barbarian_state(game)
        .selected_position
        .expect("selected position should exist");
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
        && no_units_present(game, pos)
        && cities_in_range(game, |p| p.index != player.index, pos, 2).is_empty()
}

fn no_units_present(game: &Game, pos: Position) -> bool {
    !game
        .players
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
fn get_barbarians_player(game: &Game) -> usize {
    game.players
        .iter()
        .find(|p| p.civilization.is_barbarian())
        .expect("barbarians should exist")
        .index
}
