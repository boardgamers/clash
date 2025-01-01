use macroquad::math::{u32, Vec2};

use crate::select_ui::{ConfirmSelection, SelectionConfirm};
use server::action::Action;
use server::game::Game;
use server::game::GameState::Movement;
use server::position::Position;
use server::unit::{MovementAction, Unit};

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::unit_ui::{clicked_unit, UnitSelection};

fn possible_destinations(
    game: &Game,
    start: Position,
    player_index: usize,
    units: &Vec<u32>,
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
        let unit = clicked_unit(pos, mouse_pos, p);
        unit.map_or(StateUpdate::None, |unit_id| {
            new.start = Some(pos);
            if new.units.contains(&unit_id) {
                // deselect unit
                new.units.retain(|&id| id != unit_id);
            } else {
                new.units.push(unit_id);
            }
            if new.units.is_empty() {
                new.destinations.clear();
                new.start = None;
            } else {
                new.destinations = possible_destinations(game, pos, new.player_index, &new.units);
            }
            StateUpdate::SetDialog(ActiveDialog::MoveUnits(new))
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
    pub fn new(player_index: usize, start: Option<Position>) -> MoveSelection {
        MoveSelection {
            player_index,
            units: vec![],
            start,
            destinations: vec![],
        }
    }
}

impl UnitSelection for MoveSelection {
    fn selected_units(&self) -> &[u32] {
        &self.units
    }
    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.units
    }

    fn can_select(&self, game: &Game, unit: &Unit) -> bool {
        !possible_destinations(game, unit.position, self.player_index, &vec![unit.id]).is_empty()
    }

    fn current_tile(&self) -> Option<Position> {
        self.start
    }
}

impl ConfirmSelection for MoveSelection {
    fn confirm(&self, _game: &Game) -> SelectionConfirm {
        SelectionConfirm::NoConfirm
    }
}
