use macroquad::math::u32;

use crate::select_ui::{ConfirmSelection, SelectionConfirm};
use server::action::Action;
use server::game::Game;
use server::game::GameState::Movement;
use server::position::Position;
use server::unit::{MovementAction, Unit};

use crate::client_state::{ActiveDialog, ShownPlayer, StateUpdate};
use crate::unit_ui;
use crate::unit_ui::UnitSelection;

pub fn move_units_dialog(game: &Game, sel: &MoveSelection, player: &ShownPlayer) -> StateUpdate {
    unit_ui::unit_selection_dialog::<MoveSelection>(
        game,
        player,
        "Move Units",
        sel,
        |new| update_possible_destinations(game, new.clone()),
        |_new| StateUpdate::None,
        |ui| {
            let Movement {
                movement_actions_left,
                moved_units: _,
            } = game.state
            else {
                panic!("game is not in movement")
            };

            if ui.button(None, "End Move Units") {
                StateUpdate::execute_with_warning(
                    Action::Movement(MovementAction::Stop),
                    if movement_actions_left > 0 {
                        vec![(format!("{movement_actions_left} movement actions left"))]
                    } else {
                        vec![]
                    },
                )
            } else {
                StateUpdate::None
            }
        },
    )
}

fn update_possible_destinations(game: &Game, mut sel: MoveSelection) -> StateUpdate {
    if let Some(start) = sel.start {
        sel.destinations = possible_destinations(game, start, sel.player_index, &sel.units);
    } else {
        sel.destinations.clear();
    }

    StateUpdate::SetDialog(ActiveDialog::MoveUnits(sel))
}

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

pub fn click(pos: Position, s: &MoveSelection) -> StateUpdate {
    let mut new = s.clone();
    if s.destinations.is_empty() {
        new.start = Some(pos);
        StateUpdate::SetDialog(ActiveDialog::MoveUnits(new))
    } else if s.destinations.contains(&pos) {
        let units = s.units.clone();
        StateUpdate::execute(Action::Movement(MovementAction::Move {
            units,
            destination: pos,
        }))
    } else {
        StateUpdate::None
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
