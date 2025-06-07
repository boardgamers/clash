use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::content::persistent_events::UnitsRequest;
use crate::map::{Block, Terrain};
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::{UnitType, Units, carried_units};

pub(crate) fn vikings() -> Civilization {
    Civilization::new(
        "Vikings",
        vec![ship_construction()],
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
    // todo The cost of Navigation is reduced to 0 resources
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
        |_game, player_index, units| {
            // todo check stack limit including carried units
            Some(UnitsRequest::new(
                player_index,
                units.clone(),
                0..=units.len() as u8,
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
    .build()
}
