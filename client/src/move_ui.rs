use server::action::Action;
use server::game::{Game, GameState};
use server::position::Position;
use server::unit::{can_move_units, MovementAction};

use crate::ui_state::{ActiveDialog, StateUpdate};
use crate::unit_ui;
use crate::unit_ui::UnitsSelection;

pub fn move_units_dialog(game: &Game, sel: &UnitsSelection) -> StateUpdate {
    unit_ui::unit_selection_dialog(
        game,
        sel,
        |unit| {
            !possible_destinations(game, unit.position, sel.player_index, &vec![unit.id]).is_empty()
        },
        |new| update_possible_destinations(game, new),
    )
}

fn update_possible_destinations(game: &Game, mut sel: UnitsSelection) -> StateUpdate {
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
    if let GameState::Movement {
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
                    && can_move_units(
                        game,
                        player,
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

pub fn click(pos: Position, s: &UnitsSelection) -> StateUpdate {
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
