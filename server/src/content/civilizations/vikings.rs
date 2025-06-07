use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::consts::STACK_LIMIT;
use crate::content::persistent_events::UnitsRequest;
use crate::game::Game;
use crate::map::{Block, Terrain};
use crate::player::Player;
use crate::position::Position;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::{Unit, UnitType, Units, carried_units};
use itertools::Itertools;
use std::ops::RangeInclusive;

pub(crate) fn vikings() -> Civilization {
    Civilization::new(
        "Vikings",
        vec![ship_construction(), longships()],
        vec![],
        Some(Block::new([
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Forest,
            Terrain::Water,
        ])),
    )
}

fn ship_construction() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::ShipConstruction,
        SpecialAdvanceRequirement::Advance(Advance::Fishing),
        "Ship Construction",
        "May move settlers and infantry in and out of water (if there is no enemy),\
        converting them to ships and back (or being carried by other ships). \
        The cost of Navigation is reduced to 0 resources.",
    )
    .add_units_request(
        |e| &mut e.ship_construction_conversion,
        0,
        |game, player_index, units| {
            let p = game.player(player_index);
            let dest = p.get_unit(units[0]).position;

            Some(UnitsRequest::new(
                player_index,
                units.clone(),
                convert_to_settler_range(
                    &units
                        .iter()
                        .map(|&id| game.player(player_index).get_unit(id))
                        .collect_vec(),
                    player_index,
                    dest,
                    game,
                )
                .expect("Ship construction should always have a valid range"),
                "Select units to convert to settlers (instead of infantry)",
            ))
        },
        |game, s, all_ships| {
            let player = game.player_mut(s.player_index);
            let mut units = Units::empty();
            let mut unload = vec![];
            for id in all_ships.iter() {
                let target = if s.choice.contains(id) {
                    UnitType::Settler
                } else {
                    UnitType::Infantry
                };
                units += &target;
                let unit = player.get_unit_mut(*id);
                unit.unit_type = target;
                unload.extend(carried_units(*id, player));
            }
            let mut unload_units = Units::empty();
            for id in unload {
                let unit = player.get_unit_mut(id);
                unit.carrier_id = None;
                unload_units += &unit.unit_type;
            }
            if !unload_units.is_empty() {
                game.add_info_log_item(&format!(
                    "{} unloaded {} from ships",
                    s.player_name,
                    unload_units.to_string(None)
                ));
            }
            game.add_info_log_item(&format!(
                "{} converted {} to {}",
                s.player_name,
                Units::new(0, 0, all_ships.len() as u8, 0, 0, None).to_string(None),
                units.to_string(None)
            ));
        },
    )
    .add_transient_event_listener(
        |event| &mut event.advance_cost,
        4,
        |i, &a, _| {
            if a == Advance::Navigation {
                i.set_zero_resources();
                i.info
                    .log
                    .push("Ship construction reduced the cost to 0".to_string());
            }
        },
    )
    .build()
}

pub(crate) fn is_ship_construction_move(game: &Game, units: &Vec<&Unit>, dest: Position) -> bool {
    let p = game.player(units[0].player_index);
    if !p.has_special_advance(SpecialAdvance::ShipConstruction)
        || game.enemy_player(p.index, dest).is_some()
    {
        return false;
    }

    if units.iter().all(|u| u.is_settler() || u.is_infantry()) {
        return p.available_units().ships >= units.len() as u8;
    }

    if units.iter().all(|u| u.is_ship()) {
        return convert_to_settler_range(units, p.index, dest, game).is_some();
    }

    false
}

pub(crate) fn convert_to_settler_range(
    units: &[&Unit],
    player: usize,
    dest: Position,
    game: &Game,
) -> Option<RangeInclusive<u8>> {
    let p = game.player(player);

    let present = p.get_units(dest);
    let carried: u8 = units
        .iter()
        .map(|u| {
            carried_units(u.id, p)
                .iter()
                .filter(|&c| p.get_unit(*c).is_army_unit())
                .count()
        })
        .sum::<usize>() as u8;
    let standing = present.iter().filter(|u| u.is_army_unit()).count() as u8;
    let max = units.len() as u8;
    (0..=max)
        .filter(|&i| can_add(p, carried + standing, i, max - i))
        .minmax()
        .into_option()
        .map(|(min, max)| min..=max)
}

fn can_add(p: &Player, army_units: u8, settlers: u8, infantry: u8) -> bool {
    p.available_units().settlers >= settlers
        && p.available_units().infantry >= infantry
        && army_units + infantry <= STACK_LIMIT as u8
}

fn longships() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Longships,
        SpecialAdvanceRequirement::Advance(Advance::WarShips),
        "Longships",
        "Ignore battle movement restrictions for ships. \
        Ships can carry up to 3 units.",
    )
    .build()
}
