use macroquad::math::{u32, Vec2};
use macroquad::prelude::Texture2D;
use server::action::Action;
use server::game::{CurrentMove, Game, MoveState};
use server::player::Player;
use server::position::Position;
use server::unit::{MovementAction, Unit, UnitType};

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::render_context::RenderContext;
use crate::unit_ui::{click_unit, unit_selection_clicked};

#[derive(Clone)]
pub enum MoveIntent {
    Land,
    Sea,
    Disembark,
}

impl MoveIntent {
    pub fn to_predicate(&self) -> impl Fn(&Unit) -> bool {
        match self {
            MoveIntent::Land => |u: &Unit| u.unit_type.is_land_based() && !u.is_transported(),
            MoveIntent::Sea => |u: &Unit| !u.unit_type.is_land_based(),
            MoveIntent::Disembark => |u: &Unit| u.is_transported(),
        }
    }

    pub fn toolip(&self) -> &str {
        match self {
            MoveIntent::Land => "Move land units",
            MoveIntent::Sea => "Move sea units",
            MoveIntent::Disembark => "Disembark units",
        }
    }

    pub fn icon<'a>(&self, rc: &'a RenderContext) -> &'a Texture2D {
        match self {
            MoveIntent::Land => &rc.assets().move_units,
            MoveIntent::Sea => &rc.assets().units[&UnitType::Ship],
            MoveIntent::Disembark => &rc.assets().export,
        }
    }
}

pub fn possible_destinations(
    game: &Game,
    start: Position,
    player_index: usize,
    units: &[u32],
) -> Vec<MoveDestination> {
    let player = game.get_player(player_index);

    let mut res = game
        .map
        .tiles
        .keys()
        .copied()
        .filter(|dest| {
            player
                .can_move_units(game, units, start, *dest, None)
                .is_ok()
        })
        .map(MoveDestination::Tile)
        .collect::<Vec<_>>();

    player.units.iter().for_each(|u| {
        if u.unit_type.is_ship()
            && player
                .can_move_units(game, units, start, u.position, Some(u.id))
                .is_ok()
        {
            res.push(MoveDestination::Carrier(u.id));
        }
    });
    res
}

fn move_destination(
    dest: &MoveDestination,
    pos: Position,
    unit: Option<u32>,
) -> Option<(Position, Option<u32>)> {
    match dest {
        MoveDestination::Tile(p) if p == &pos => Some((*p, None)),
        MoveDestination::Carrier(id) if unit.is_some_and(|u| u == *id) => Some((pos, Some(*id))),
        _ => None,
    }
}

pub fn click(rc: &RenderContext, pos: Position, s: &MoveSelection, mouse_pos: Vec2) -> StateUpdate {
    let game = rc.game;
    let p = game.get_player(s.player_index);
    let carrier = click_unit(rc, pos, mouse_pos, p, false);
    if let Some((destination, embark_carrier_id)) = s
        .destinations
        .iter()
        .find_map(|d| move_destination(d, pos, carrier))
    {
        let units = s.units.clone();
        StateUpdate::execute(Action::Movement(MovementAction::Move {
            units,
            destination,
            embark_carrier_id,
        }))
    } else if s.start.is_some_and(|p| p != pos) {
        // first need to deselect units
        StateUpdate::None
    } else {
        let mut new = s.clone();
        let unit = click_unit(rc, pos, mouse_pos, p, true);
        unit.map_or(StateUpdate::None, |unit_id| {
            let is_transported = p.get_unit(unit_id).unwrap().is_transported();
            new.start = Some(pos);
            if new.units.is_empty() {
                new.units = movable_units(pos, game, p, |u| u.is_transported() == is_transported);
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

pub fn movable_units(
    pos: Position,
    game: &Game,
    p: &Player,
    pred: impl Fn(&Unit) -> bool,
) -> Vec<u32> {
    p.units
        .iter()
        .filter(|u| pred(u) && !possible_destinations(game, pos, p.index, &[u.id]).is_empty())
        .map(|u| u.id)
        .collect()
}

#[derive(Clone, Debug)]
pub enum MoveDestination {
    Tile(Position),
    Carrier(u32),
}

#[derive(Clone, Debug)]
pub struct MoveSelection {
    pub player_index: usize,
    pub units: Vec<u32>,
    pub start: Option<Position>,
    pub destinations: Vec<MoveDestination>,
    // pub lo
}

impl MoveSelection {
    pub fn new(
        player_index: usize,
        start: Option<Position>,
        game: &Game,
        move_intent: &MoveIntent,
        move_state: &MoveState,
    ) -> MoveSelection {
        if let CurrentMove::Fleet { units } = &move_state.current_move {
            let fleet_pos = game
                .get_player(player_index)
                .get_unit(units[0])
                .unwrap()
                .position;
            return MoveSelection {
                player_index,
                start: Some(fleet_pos),
                units: units.clone(),
                destinations: possible_destinations(game, fleet_pos, player_index, units),
            };
        }

        match start {
            Some(pos) => {
                let movable_units = movable_units(
                    pos,
                    game,
                    game.get_player(player_index),
                    move_intent.to_predicate(),
                );
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
