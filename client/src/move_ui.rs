use macroquad::math::{u32, Vec2};

use server::action::Action;
use server::game::Game;
use server::game::GameState::Movement;
use server::player::Player;
use server::position::Position;
use server::unit::MovementAction;

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::unit_ui::{unit_at_pos, unit_selection_clicked};

pub fn possible_destinations(
    game: &Game,
    start: Position,
    player_index: usize,
    units: &[u32],
) -> Vec<Position> {
    let player = game.get_player(player_index);

    let (moved_units, movement_actions_left) = if let Movement(m) = &game.state {
        (&m.moved_units, m.movement_actions_left)
    } else {
        (&vec![], 1)
    };

    start
        .neighbors()
        .into_iter()
        .filter(|dest| {
            game.map.tiles.contains_key(dest)
                && player
                    .can_move_units(
                        game,
                        units,
                        start,
                        *dest,
                        movement_actions_left,
                        moved_units,
                    )
                    .is_ok()
        })
        .collect::<Vec<_>>()
}

pub fn click(pos: Position, s: &MoveSelection, mouse_pos: Vec2, game: &Game) -> StateUpdate {
    if s.destinations.contains(&pos) {
        let units = s.units.clone();
        StateUpdate::execute(Action::Movement(MovementAction::Move {
            units,
            destination: pos,
        }))
    } else if s.start.is_some_and(|p| p != pos) {
        // first need to deselect units
        StateUpdate::None
    } else {
        let mut new = s.clone();
        let p = game.get_player(s.player_index);
        let unit = unit_at_pos(pos, mouse_pos, p);
        unit.map_or(StateUpdate::None, |unit_id| {
            new.start = Some(pos);
            if new.units.is_empty() {
                new.units = movable_units(pos, game, p);
            } else {
                unit_selection_clicked(unit_id, &mut new.units);
            }
            if new.units.is_empty() {
                new.destinations.clear();
                new.start = None;
            } else {
                new.destinations = possible_destinations(game, pos, new.player_index, &new.units);
            }
            StateUpdate::OpenDialog(ActiveDialog::MoveUnits(new))
        })
    }
}

pub fn movable_units(pos: Position, game: &Game, p: &Player) -> Vec<u32> {
    p.units
        .iter()
        .filter(|u| !possible_destinations(game, pos, p.index, &[u.id]).is_empty())
        .map(|u| u.id)
        .collect()
}

#[derive(Clone, Debug)]
pub struct MoveSelection {
    pub player_index: usize,
    pub units: Vec<u32>,
    pub start: Option<Position>,
    pub destinations: Vec<Position>,
}

impl MoveSelection {
    pub fn new(player_index: usize, start: Option<Position>, game: &Game) -> MoveSelection {
        match start {
            Some(pos) => {
                let movable_units = movable_units(pos, game, game.get_player(player_index));
                if movable_units.is_empty() {
                    return Self::empty(player_index);
                }
                MoveSelection {
                    player_index,
                    start: Some(pos),
                    destinations: possible_destinations(game, pos, player_index, &movable_units),
                    units: movable_units,
                }
            }
            None => Self::empty(player_index),
        }
    }

    fn empty(player_index: usize) -> MoveSelection {
        MoveSelection {
            player_index,
            start: None,
            units: vec![],
            destinations: vec![],
        }
    }
}
