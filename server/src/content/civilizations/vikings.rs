use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::city_pieces::Building;
use crate::civilization::Civilization;
use crate::consts::STACK_LIMIT;
use crate::content::ability::Ability;
use crate::content::advances::trade_routes::TradeRoute;
use crate::content::persistent_events::{PaymentRequest, UnitsRequest};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo};
use crate::leader_ability::LeaderAbility;
use crate::map::{Block, Terrain};
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player::Player;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::{Unit, UnitType, Units, carried_units};
use itertools::Itertools;
use std::ops::RangeInclusive;

pub(crate) fn vikings() -> Civilization {
    Civilization::new(
        "Vikings",
        vec![ship_construction(), longships(), raids(), runes()],
        vec![knut()],
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

const RAID: &str = "viking_raid";

fn raids() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Raiding,
        SpecialAdvanceRequirement::Advance(Advance::TradeRoutes),
        "Raiding",
        "Trade routes: Ships that are adjacent to enemy cities raid: \
        That player loses 1 resource.",
    )
    .build()
}

pub(crate) fn lose_raid_resource() -> Ability {
    Ability::builder(
        "Viking Raids",
        "Lose 1 resource to Viking Raids (triggered by Trade Routes)",
    )
    .add_payment_request_listener(
        |e| &mut e.turn_start,
        4,
        |game, player_index, ()| {
            let c = PaymentOptions::sum(
                game.player(player_index),
                PaymentReason::SpecialAdvanceAbility,
                1,
                &ResourceType::resources(),
            );
            let p = game.player_mut(player_index);
            if !p.can_afford(&c) {
                return None;
            }

            p.event_info
                .remove(RAID)
                .is_some()
                .then_some(vec![PaymentRequest::mandatory(
                    c,
                    "Pay 1 resource to Viking Raids",
                )])
        },
        |game, s, ()| {
            game.add_info_log_item(&format!(
                "{} lost {} to Viking Raids",
                s.player_name, s.choice[0]
            ));
        },
    )
    .build()
}

pub(crate) fn add_raid_bonus(game: &mut Game, player: usize, routes: &[TradeRoute]) {
    let name = game.player_name(player);
    for r in routes {
        let u = game.player(player).get_unit(r.unit_id);
        if u.is_ship() && u.position.distance(r.to) == 1 {
            let city = game.get_any_city(r.to);
            let position = city.position;
            let opponent = game.player_mut(city.player_index);
            let opponent_name = opponent.get_name();
            if opponent
                .event_info
                .insert(RAID.to_string(), "true".to_string())
                .is_none()
            {
                game.add_info_log_item(&format!("{name} raided {opponent_name} at {position}",));
            }
        }
    }
}

fn runes() -> SpecialAdvanceInfo {
    let b = SpecialAdvanceInfo::builder(
        SpecialAdvance::RuneStones,
        SpecialAdvanceRequirement::Advance(Advance::Rituals),
        "Rune Stones",
        "When you lost 2 or more units in a battle, you may convert 1 Obelisk \
        to a Rune Stone, which counts as 1 objective victory point.",
    );
    let o = b.get_key().clone();
    b.add_bool_request(
        |event| &mut event.combat_end,
        24,
        |game, player_index, s| {
            (s.player(player_index)
                .fighter_losses(s.battleground)
                .amount()
                >= 2
                && game
                    .player(player_index)
                    .is_building_available(Building::Obelisk, game))
            .then_some("Do you want to convert an Obelisk to a Rune Stone?".to_string())
        },
        move |game, s, _| {
            if s.choice {
                let p = game.player_mut(s.player_index);
                p.destroyed_structures.add_building(Building::Obelisk);
                p.gain_objective_victory_points(1.0, &o);
                game.add_info_log_item(&format!(
                    "{} converted an Obelisk to a Rune Stone for 1 objective point",
                    s.player_name
                ));
            } else {
                game.add_info_log_item(&format!(
                    "{} did not convert an Obelisk to a Rune Stone",
                    s.player_name
                ));
            }
        },
    )
    .build()
}

fn knut() -> LeaderInfo {
    // todo Ruler of the North
    // todo Danegeld
    LeaderInfo::new(
        Leader::Knut,
        "Knut the Great",
        LeaderAbility::builder("Ruler of the North", "todo").build(),
        LeaderAbility::builder("Danegeld", "todo").build(),
    )
}
