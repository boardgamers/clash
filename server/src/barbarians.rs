use crate::action::Action;
use crate::consts::STACK_LIMIT;
use crate::content::custom_phase_actions::{
    CustomPhaseEventAction, CustomPhasePositionRequest, CustomPhaseUnitRequest,
};
use crate::game::Game;
use crate::incident::{IncidentBuilder, BASE_EFFECT_PRIORITY};
use crate::map::Terrain;
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::unit::{UnitType, Units};

pub(crate) fn barbarians_spawn(builder: IncidentBuilder) -> IncidentBuilder {
    builder
        .add_incident_position_listener(
            IncidentTarget::ActivePlayer,
            BASE_EFFECT_PRIORITY + 2,
            |game, player_index, _i| {
                Some(CustomPhasePositionRequest::new(
                    possible_barbarians_spawns(game, game.get_player(player_index)),
                    Some("Select a position for the new city and infantry unit".to_string()),
                ))
            },
            |c, game, pos| {
                c.add_info_log_item(&format!(
                    "Barbarians spawned a new city and a new Infantry unit at {pos}",
                ));
                let player = get_barbarians_player(game);
                c.gain_city(player, *pos);
                c.gain_unit(player, UnitType::Infantry, *pos);
            },
        )
        .add_incident_position_listener(
            IncidentTarget::ActivePlayer,
            BASE_EFFECT_PRIORITY + 1,
            |game, _player_index, _i| {
                Some(CustomPhasePositionRequest::new(
                    possible_barbarians_reinforcements(game),
                    Some("Select a position for the additional Barbarian unit".to_string()),
                ))
            },
            |_c, _game, _pos| {
                // used by next listener
            },
        )
        .add_incident_unit_listener(
            IncidentTarget::ActivePlayer,
            BASE_EFFECT_PRIORITY,
            |game, _player_index, _i| {
                let choices = get_barbarian_reinforcement_choices(game);
                Some(CustomPhaseUnitRequest::new(
                    choices,
                    Some("Select a unit to reinforce the barbarians".to_string()),
                ))
            },
            |c, game, unit| {
                let position = get_barbarian_reinforcement_position(game);
                let units = Units::from_iter(vec![*unit]);
                c.add_info_log_item(&format!("Barbarians reinforced with {units} at {position}",));
                let player = get_barbarians_player(game);
                c.gain_unit(player, *unit, position);
            },
        )
}

fn possible_barbarians_spawns(game: &Game, player: &Player) -> Vec<Position> {
    let primary: Vec<Position> = game
        .map
        .tiles
        .keys()
        .filter(|&pos| {
            is_base_barbarian_tile(game, *pos, player)
                && city_exactly_land_range2_and_at_least_range2_other_cities(game, player, *pos)
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
            is_base_barbarian_tile(game, *pos, player) && adjacent_to_cities(player, *pos)
        })
        .copied()
        .collect();

    secondary
}

fn possible_barbarians_reinforcements(game: &Game) -> Vec<Position> {
    let p = game.get_player(get_barbarians_player(game));
    p.cities
        .iter()
        .filter(|c| p.get_units(c.position).len() < STACK_LIMIT)
        .map(|c| c.position)
        .collect()
}

fn get_barbarian_reinforcement_choices(game: &Game) -> Vec<UnitType> {
    let player = game.get_player(get_barbarians_player(game));
    let possible = if player
        .get_units(get_barbarian_reinforcement_position(game))
        .iter()
        .any(|u| u.unit_type == UnitType::Infantry)
    {
        vec![UnitType::Infantry, UnitType::Cavalry, UnitType::Elephant]
    } else {
        vec![UnitType::Infantry]
    };
    possible
        .iter()
        .filter(|u| player.available_units().has_unit(u))
        .copied()
        .collect()
}

fn get_barbarian_reinforcement_position(game: &Game) -> Position {
    game.action_log
        .iter()
        .rev()
        .find_map(|a| {
            if let Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(p)) = &a.action {
                Some(*p)
            } else {
                None
            }
        })
        .expect("last action should be a custom phase event action")
}

fn is_base_barbarian_tile(game: &Game, pos: Position, player: &Player) -> bool {
    game.map
        .get(pos)
        .is_some_and(|t| t.is_land() && !matches!(t, Terrain::Barren))
        && is_empty(game, pos)
        && cities_in_range(game, |p| p.index != player.index, pos, 2) == 0
}

fn is_empty(game: &Game, pos: Position) -> bool {
    !game
        .players
        .iter()
        .any(|p| p.units.iter().any(|u| u.position == pos))
}

fn city_exactly_land_range2_and_at_least_range2_other_cities(
    game: &Game,
    player: &Player,
    start: Position,
) -> bool {
    cities_in_range(game, |p| p == player, start, 1) == 0
        && start.neighbors().into_iter().any(|middle| {
            game.map.is_land(middle)
                && player
                    .cities
                    .iter()
                    .any(|c| c.position.distance(start) == 2 && c.position.distance(middle) == 1)
        })
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
) -> usize {
    game.players
        .iter()
        .filter(|p| player(p))
        .map(|p| {
            p.cities
                .iter()
                .filter(|c| c.position.distance(pos) <= range)
                .count()
        })
        .sum()
}

#[must_use]
fn get_barbarians_player(game: &Game) -> usize {
    game.players
        .iter()
        .find(|p| p.civilization.is_barbarian())
        .expect("barbarians should exist")
        .index
}
