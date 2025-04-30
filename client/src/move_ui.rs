use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::cancel_button_with_tooltip;
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use crate::unit_ui::{click_unit, unit_selection_clicked};
use macroquad::math::{Vec2, u32};
use macroquad::prelude::Texture2D;
use server::action::Action;
use server::events::EventOrigin;
use server::game::{Game, GameState};
use server::movement::{CurrentMove, MoveUnits, MovementAction, possible_move_units_destinations};
use server::payment::PaymentOptions;
use server::player::Player;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::{Unit, UnitType};
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub enum MoveIntent {
    Land,
    Sea,
    Disembark,
}

#[derive(Clone)]
pub struct MovePayment {
    pub action: MovementAction,
    pub payment: Payment<String>,
}

impl MoveIntent {
    pub fn to_predicate(self) -> impl Fn(&Unit) -> bool {
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

    pub fn icon<'a>(self, rc: &'a RenderContext) -> &'a Texture2D {
        match self {
            MoveIntent::Land => &rc.assets().move_units,
            MoveIntent::Sea => rc.assets().unit(UnitType::Ship, rc.shown_player),
            MoveIntent::Disembark => &rc.assets().export,
        }
    }
}

pub fn possible_destinations(
    game: &Game,
    start: Position,
    player_index: usize,
    units: &[u32],
) -> MoveDestinations {
    let player = game.player(player_index);
    let mut modifiers = HashSet::new();

    let mut res = possible_move_units_destinations(player, game, units, start, None)
        .unwrap_or_default()
        .into_iter()
        .map(|route| {
            modifiers.extend(route.cost.modifiers.clone());
            MoveDestination::Tile(route.destination, route.cost)
        })
        .collect::<Vec<_>>();

    player.units.iter().for_each(|u| {
        if u.unit_type.is_ship()
            && possible_move_units_destinations(player, game, units, start, Some(u.id))
                .is_ok_and(|v| v.iter().any(|route| route.destination == u.position))
        {
            res.push(MoveDestination::Carrier(u.id));
        }
    });
    MoveDestinations {
        list: res,
        modifiers,
    }
}

pub fn click(rc: &RenderContext, pos: Position, s: &MoveSelection, mouse_pos: Vec2) -> StateUpdate {
    let game = rc.game;
    let p = game.player(s.player_index);
    let carrier = click_unit(rc, pos, mouse_pos, p, false);
    match s
        .clone()
        .destinations
        .list
        .into_iter()
        .find_map(|d| match d {
            MoveDestination::Tile(p, cost) if p == pos => Some((p, None, cost)),
            MoveDestination::Carrier(id) if carrier.is_some_and(|u| u == id) => {
                Some((pos, Some(id), PaymentOptions::free()))
            }
            _ => None,
        }) {
        Some((destination, embark_carrier_id, cost)) => {
            let units = s.units.clone();
            let action = MovementAction::Move(MoveUnits::new(
                units,
                destination,
                embark_carrier_id,
                ResourcePile::empty(),
            ));

            if !cost.is_free() {
                return StateUpdate::OpenDialog(ActiveDialog::MovePayment(MovePayment {
                    action,
                    payment: rc.new_payment(&cost, "Move units".to_string(), "Move units", true),
                }));
            }
            StateUpdate::execute(Action::Movement(action))
        }
        _ => {
            if s.start.is_some_and(|p| p != pos) {
                // first need to deselect units
                StateUpdate::None
            } else {
                click_unit(rc, pos, mouse_pos, p, true).map_or_else(
                    || tile_clicked(pos, s, game, p),
                    |unit_id| unit_clicked(pos, s, game, p, unit_id),
                )
            }
        }
    }
}

fn unit_clicked(
    pos: Position,
    s: &MoveSelection,
    game: &Game,
    p: &Player,
    unit_id: u32,
) -> StateUpdate {
    let mut new = s.clone();
    new.start = Some(pos);
    let is_transported = p.get_unit(unit_id).is_transported();
    if new.units.is_empty() {
        new.units = movable_units(pos, game, p, |u| u.is_transported() == is_transported);
    } else {
        unit_selection_clicked(unit_id, &mut new.units);
    }

    unit_selection_changed(pos, game, new)
}

fn tile_clicked(pos: Position, s: &MoveSelection, game: &Game, p: &Player) -> StateUpdate {
    let mut new = s.clone();
    new.start = Some(pos);
    if new.units.is_empty() {
        new.units = movable_units(pos, game, p, |u| !u.is_transported());
        unit_selection_changed(pos, game, new)
    } else {
        StateUpdate::None
    }
}

fn unit_selection_changed(pos: Position, game: &Game, mut new: MoveSelection) -> StateUpdate {
    if new.units.is_empty() {
        new.destinations.list.clear();
        new.start = None;
    } else {
        new.destinations = possible_destinations(game, pos, new.player_index, &new.units);
    }
    StateUpdate::OpenDialog(ActiveDialog::MoveUnits(new))
}

pub fn movable_units(
    pos: Position,
    game: &Game,
    p: &Player,
    pred: impl Fn(&Unit) -> bool,
) -> Vec<u32> {
    p.units
        .iter()
        .filter(|u| {
            u.position == pos
                && pred(u)
                && !possible_destinations(game, pos, p.index, &[u.id])
                    .list
                    .is_empty()
        })
        .map(|u| u.id)
        .collect()
}

#[derive(Clone, Debug)]
pub struct MoveDestinations {
    pub list: Vec<MoveDestination>,
    pub modifiers: HashSet<EventOrigin>,
}

#[derive(Clone, Debug)]
pub enum MoveDestination {
    Tile(Position, PaymentOptions),
    Carrier(u32),
}

#[derive(Clone, Debug)]
pub struct MoveSelection {
    pub player_index: usize,
    pub units: Vec<u32>,
    pub start: Option<Position>,
    pub destinations: MoveDestinations,
}

impl MoveSelection {
    pub fn new(
        player_index: usize,
        start: Option<Position>,
        game: &Game,
        move_intent: MoveIntent,
        current_move: &CurrentMove,
    ) -> MoveSelection {
        if let CurrentMove::Fleet { units } = current_move {
            let fleet_pos = game.player(player_index).get_unit(units[0]).position;
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
                    game.player(player_index),
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
            destinations: MoveDestinations {
                list: vec![],
                modifiers: HashSet::new(),
            },
        }
    }
}

pub(crate) fn move_units_dialog(rc: &RenderContext) -> StateUpdate {
    if matches!(rc.game.state, GameState::Playing)
        && cancel_button_with_tooltip(rc, "Back to playing actions")
    {
        return StateUpdate::CloseDialog;
    }
    StateUpdate::None
}

pub(crate) fn move_payment_dialog(rc: &RenderContext, mp: &MovePayment) -> StateUpdate {
    payment_dialog(
        rc,
        &mp.payment.clone(),
        true,
        |p| {
            let mut new = mp.clone();
            new.payment = p;
            ActiveDialog::MovePayment(new)
        },
        |payment| {
            if let MovementAction::Move(mut m) = mp.action.clone() {
                m.payment = payment;
                StateUpdate::execute(Action::Movement(MovementAction::Move(m)))
            } else {
                panic!("Unexpected action");
            }
        },
    )
}
