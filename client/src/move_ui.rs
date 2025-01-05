use macroquad::math::{u32, Vec2};

use server::action::Action;
use server::game::Game;
use server::game::GameState::Movement;
use server::position::Position;
use server::unit::MovementAction;

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::unit_ui::{unit_at_pos, unit_selection_clicked};

fn possible_destinations(
    game: &Game,
    start: Position,
    player_index: usize,
    units: &[u32],
) -> Vec<Position> {
    if let Movement {
        movement_actions_left,
        moved_units,
    } = &game.state
    {
        let player = game.get_player(player_index);

        game.map
            .tiles
            .keys()
            .copied()
            .filter(|dest| {
                start != *dest
                    && player
                        .can_move_units(
                            game,
                            units,
                            start,
                            *dest,
                            *movement_actions_left,
                            moved_units,
                        )
                        .is_ok()
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    }
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
                //select all possible units
                new.units = p
                    .units
                    .iter()
                    .filter(|u| {
                        !possible_destinations(game, pos, new.player_index, &[u.id]).is_empty()
                    })
                    .map(|u| u.id)
                    .collect();
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

#[derive(Clone, Debug)]
pub struct MoveSelection {
    pub player_index: usize,
    pub units: Vec<u32>,
    pub start: Option<Position>,
    pub destinations: Vec<Position>,
}

impl MoveSelection {
    pub fn new(player_index: usize) -> MoveSelection {
        MoveSelection {
            player_index,
            units: vec![],
            start: None,
            destinations: vec![],
        }
    }
}
