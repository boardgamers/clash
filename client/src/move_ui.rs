use server::action::Action;
use crate::ui_state::{ActiveDialog, StateUpdate, StateUpdates};
use crate::unit_ui;
use crate::unit_ui::UnitsSelection;
use server::game::Game;
use server::position::Position;
use server::unit::{can_move_units, MovementAction};

pub fn move_units_dialog(game: &Game, sel: &UnitsSelection) -> StateUpdate {
    unit_ui::unit_selection_dialog(game, sel, |new| update_possible_destinations(game, new))
}

fn update_possible_destinations(game: &Game, mut sel: UnitsSelection) -> StateUpdate {
    if let Some(start) = sel.start {
        let player = game.get_player(sel.player_index);
        sel.destinations = game
            .map
            .tiles
            .keys()
            .copied()
            .filter(|dest| {
                start != *dest && can_move_units(game, player, &sel.units, start, *dest).is_ok()
            })
            .collect::<Vec<_>>();
    } else {
        sel.destinations.clear();
    }

    StateUpdate::SetDialog(ActiveDialog::MoveUnits(sel))
}

pub fn click(pos: Position, s: &UnitsSelection) -> StateUpdates {
    let updates = StateUpdates::new();
    let mut new = s.clone();
    if s.destinations.is_empty() {
        new.start = Some(pos);
        StateUpdate::SetDialog(ActiveDialog::MoveUnits(new))
    } else if s.destinations.contains(&pos) {
        let units = s.units.clone();
        StateUpdate::execute(
            Action::Movement(MovementAction::Move {
                units,
                destination: pos,
            }),
            vec![],
        )
    } else {
        StateUpdate::None
    }
    updates
}
