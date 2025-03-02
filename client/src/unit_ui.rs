use macroquad::math::{u32, vec2, Vec2};
use macroquad::shapes::draw_circle;

use server::game::Game;
use server::position::Position;
use server::unit::{carried_units, MovementRestriction, Unit, UnitType};

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::hex_ui;
use crate::select_ui::{may_cancel, ConfirmSelection, HighlightType};

use crate::dialog_ui::ok_button;
use crate::layout_ui::{draw_scaled_icon, is_in_circle};
use crate::move_ui::MoveDestination;
use crate::render_context::RenderContext;
use crate::tooltip::show_tooltip_for_circle;
use itertools::Itertools;
use server::consts::ARMY_MOVEMENT_REQUIRED_ADVANCE;
use server::player::Player;

pub struct UnitPlace {
    pub center: Vec2,
    pub radius: f32,
}

impl UnitPlace {
    pub fn new(center: Vec2, radius: f32) -> UnitPlace {
        UnitPlace { center, radius }
    }
}

struct UnitHighlight {
    player: usize,
    unit: u32,
    highlight_type: HighlightType,
}

pub fn draw_unit_type(
    rc: &RenderContext,
    unit_highlight_type: HighlightType,
    center: Vec2,
    unit_type: UnitType,
    player_index: usize,
    tooltip: &str,
    size: f32,
) -> bool {
    draw_circle(center.x, center.y, size, unit_highlight_type.color());
    draw_circle(center.x, center.y, size - 2., rc.player_color(player_index));
    let icon_size = size * 1.1;
    draw_scaled_icon(
        rc,
        &rc.assets().units[&unit_type],
        tooltip,
        vec2(center.x - icon_size / 2., center.y - icon_size / 2.),
        icon_size,
    )
}

fn carried_unit_place(carrier: &UnitPlace, index: usize) -> UnitPlace {
    let r = carrier.radius / 2.0;
    UnitPlace::new(hex_ui::rotate_around(carrier.center, r, 180 * index), r)
}

fn unit_place(rc: &RenderContext, index: usize, position: Position) -> UnitPlace {
    let has_city = rc.game.get_any_city(position).is_some();
    let c = hex_ui::center(position);
    let n = units_on_tile(rc.game, position)
        .filter(|(_, u)| !u.is_transported())
        .count();
    if has_city || n > 4 {
        UnitPlace::new(hex_ui::rotate_around(c, 40.0, (40 * index) + 45), 11.0)
    } else if n == 1 {
        UnitPlace::new(c, 18.0)
    } else if n == 2 {
        UnitPlace::new(hex_ui::rotate_around(c, 27.0, 180 * index), 18.0)
    } else {
        UnitPlace::new(hex_ui::rotate_around(c, 27.0, 90 * index), 18.0)
    }
}

pub fn click_unit(
    rc: &RenderContext,
    pos: Position,
    mouse_pos: Vec2,
    player: &Player,
    can_select_carried_units: bool,
) -> Option<u32> {
    player
        .units
        .iter()
        .filter(|u| u.position == pos && !u.is_transported())
        .enumerate()
        .find_map(|(i, u)| {
            let place = unit_place(rc, i, pos);
            if can_select_carried_units {
                let game = rc.game;
                let player_index = player.index;
                let carried_units = carried_units(u.id, &game.players[player_index]);
                for (j, carried) in carried_units.iter().enumerate() {
                    let carried_place = carried_unit_place(&place, j);
                    if is_in_circle(mouse_pos, carried_place.center, carried_place.radius) {
                        return Some(*carried);
                    }
                }
            }
            if is_in_circle(mouse_pos, place.center, place.radius) {
                Some(u.id)
            } else {
                None
            }
        })
}

pub fn non_leader_names() -> [(UnitType, &'static str); 5] {
    [
        (UnitType::Settler, "Settler"),
        (UnitType::Infantry, "Infantry"),
        (UnitType::Ship, "Ship"),
        (UnitType::Elephant, "Elephant"),
        (UnitType::Cavalry, "Cavalry"),
    ]
}

pub fn draw_units(rc: &RenderContext, tooltip: bool) {
    let player = rc.shown_player.index;
    let highlighted_units = match rc.state.active_dialog {
        ActiveDialog::MoveUnits(ref s) => {
            let mut h = highlight_units(player, &s.units, HighlightType::Primary);
            for d in &s.destinations.list {
                if let MoveDestination::Carrier(id) = d {
                    h.push(UnitHighlight {
                        player,
                        unit: *id,
                        highlight_type: HighlightType::Secondary,
                    });
                }
            }
            h
        }
        ActiveDialog::ReplaceUnits(ref s) => highlight_units(
            rc.shown_player.index,
            &s.replaced_units,
            HighlightType::Primary,
        ),
        ActiveDialog::UnitsRequest(ref s) => {
            highlight_units(s.player, &s.units, HighlightType::Primary)
                .into_iter()
                .chain(highlight_units(
                    s.player,
                    &s.selectable,
                    HighlightType::Secondary,
                ))
                .collect_vec()
        }
        _ => vec![],
    };

    for (_pos, on_tile) in &rc
        .game
        .players
        .iter()
        .flat_map(|p| {
            p.units
                .iter()
                .filter(|u| !u.is_transported())
                .map(move |u| (p.index, u))
        })
        .sorted_by_key(|(_, u)| u.position)
        .chunk_by(|(_, a)| a.position)
    {
        on_tile
            .collect::<Vec<_>>()
            .iter()
            .enumerate()
            .for_each(|(i, (p, u))| {
                let place = unit_place(rc, i, u.position);

                draw_unit(rc, tooltip, &highlighted_units, *p, u, &place);

                let player = rc.game.get_player(*p);
                let game = rc.game;
                let player_index = *p;
                let carrier = u.id;
                let carried = carried_units(carrier, &game.players[player_index]);
                carried.iter().enumerate().for_each(|(j, u)| {
                    draw_unit(
                        rc,
                        tooltip,
                        &highlighted_units,
                        *p,
                        player.get_unit(*u).unwrap(),
                        &carried_unit_place(&place, j),
                    );
                });
            });
    }
}

fn highlight_units(
    player: usize,
    units: &[u32],
    highlight_type: HighlightType,
) -> Vec<UnitHighlight> {
    units
        .iter()
        .map(move |unit| UnitHighlight {
            player,
            unit: *unit,
            highlight_type,
        })
        .collect()
}

fn draw_unit(
    rc: &RenderContext,
    tooltip: bool,
    selected_units: &[UnitHighlight],
    player_index: usize,
    unit: &Unit,
    place: &UnitPlace,
) {
    let center = place.center;
    let radius = place.radius;
    let game = &rc.game;
    if tooltip {
        let army_move = game
            .get_player(player_index)
            .has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE);
        show_tooltip_for_circle(rc, &unit_label(unit, army_move), center, radius);
    } else {
        let highlight = selected_units
            .iter()
            .find(|u| u.unit == unit.id && u.player == player_index)
            .map_or(HighlightType::None, |u| u.highlight_type);

        draw_unit_type(
            rc,
            highlight,
            center,
            unit.unit_type,
            unit.player_index,
            "",
            radius,
        );
    }
}

pub trait UnitSelection: ConfirmSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32>;
    fn can_select(&self, game: &Game, unit: &Unit) -> bool;
    fn player_index(&self) -> usize;
}

pub fn unit_selection_click<T: UnitSelection>(
    rc: &RenderContext,
    pos: Position,
    mouse_pos: Vec2,
    sel: &T,
    on_change: impl Fn(T) -> StateUpdate,
) -> StateUpdate {
    let p = rc.game.get_player(sel.player_index());
    if let Some(unit_id) = click_unit(rc, pos, mouse_pos, p, true) {
        if sel.can_select(rc.game, p.get_unit(unit_id).unwrap()) {
            let mut new = sel.clone();
            unit_selection_clicked(unit_id, new.selected_units_mut());
            return on_change(new);
        }
    }
    StateUpdate::None
}

pub fn unit_selection_dialog<T: UnitSelection>(
    rc: &RenderContext,
    sel: &T,
    on_ok: impl FnOnce(T) -> StateUpdate,
) -> StateUpdate {
    if ok_button(rc, sel.confirm(rc.game)) {
        on_ok(sel.clone())
    } else {
        may_cancel(sel, rc)
    }
}

pub fn units_on_tile(game: &Game, pos: Position) -> impl Iterator<Item = (usize, Unit)> + '_ {
    game.players.iter().flat_map(move |p| {
        p.units.iter().filter_map(move |unit| {
            if unit.position == pos {
                Some((p.index, unit.clone()))
            } else {
                None
            }
        })
    })
}

pub fn name(u: &UnitType) -> &str {
    if let UnitType::Leader = u {
        return "Leader";
    }
    non_leader_names()
        .into_iter()
        .find(|(unit_type, _)| unit_type == u)
        .unwrap()
        .1
}

pub fn unit_label(unit: &Unit, army_move: bool) -> String {
    let name = name(&unit.unit_type);
    let mut notes = vec![];

    if unit.unit_type.is_army_unit() && !army_move {
        notes.push("research Tactics to move the unit");
    } else {
        for r in unit.movement_restrictions.iter().unique() {
            match r {
                MovementRestriction::Battle => {
                    notes.push("can't move again (battle)");
                }
                MovementRestriction::Mountain => {
                    notes.push("can't move out of a Mountain this turn");
                }
                MovementRestriction::Forest => {
                    notes.push("can't attack from a Forest this turn");
                }
            }
        }
    }
    let suffix = if notes.is_empty() {
        ""
    } else {
        &format!(" ({})", notes.join(", "))
    };

    format!("{name}{suffix}")
}

pub fn unit_selection_clicked(unit_id: u32, units: &mut Vec<u32>) {
    if units.contains(&unit_id) {
        // deselect unit
        units.retain(|&id| id != unit_id);
    } else {
        units.push(unit_id);
    }
}
