use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::city::MoodState;
use crate::city_pieces::Building;
use crate::civilization::Civilization;
use crate::combat::move_with_possible_combat;
use crate::consts::STACK_LIMIT;
use crate::content::ability::{Ability, AbilityBuilder};
use crate::content::advances::economy::use_taxes;
use crate::content::advances::trade_routes::TradeRoute;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{PaymentRequest, PositionRequest, UnitsRequest};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::{LeaderAbility, activate_leader_city, can_activate_leader_city};
use crate::map::{Block, Terrain, block_for_position, capital_city_position};
use crate::movement::{MoveUnits, move_action_log};
use crate::player::{Data, Player};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::{Unit, UnitType, Units, carried_units};
use crate::victory_points::{
    SpecialVictoryPoints, VictoryPointAttribution, add_special_victory_points,
};
use itertools::Itertools;
use std::ops::RangeInclusive;

pub(crate) fn vikings() -> Civilization {
    Civilization::new(
        "Vikings",
        vec![ship_construction(), longships(), raids(), runes()],
        vec![knut(), erik(), ragnar()],
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
        |game, p, units| {
            let player_index = p.index;
            let dest = p.get(game).get_unit(units[0]).position;

            Some(UnitsRequest::new(
                player_index,
                units.clone(),
                convert_to_settler_range(
                    &units
                        .iter()
                        .map(|&id| p.get(game).get_unit(id))
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
                s.log(
                    game,
                    &format!("Unloaded {} from ships", unload_units.to_string(None)),
                );
            }
            s.log(
                game,
                &format!(
                    "Converted {} to {}",
                    Units::new(0, 0, all_ships.len() as u8, 0, 0, None).to_string(None),
                    units.to_string(None)
                ),
            );
        },
    )
    .add_transient_event_listener(
        |event| &mut event.advance_cost,
        4,
        |i, &a, _, p| {
            if a == Advance::Navigation {
                i.set_zero_resources(p);
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
        |game, p, ()| {
            let c = p
                .payment_options()
                .sum(p.get(game), 1, &ResourceType::resources());
            let p = p.get_mut(game);
            if !p.can_afford(&c) {
                return None;
            }

            p.event_info
                .remove(RAID)
                .is_some()
                .then_some(vec![PaymentRequest::mandatory(c, "Pay 1 resource")])
        },
        |game, s, ()| {
            s.log(game, &format!("Lose {}", s.choice[0]));
        },
    )
    .build()
}

pub(crate) fn add_raid_bonus(game: &mut Game, player: usize, routes: &[TradeRoute]) {
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
                game.log(
                    player,
                    &EventOrigin::SpecialAdvance(SpecialAdvance::Raiding),
                    &format!("Raided {opponent_name} at {position}"),
                );
            }
        }
    }
}

fn runes() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::RuneStones,
        SpecialAdvanceRequirement::Advance(Advance::Rituals),
        "Rune Stones",
        "When you lost 2 or more units in a battle, you may convert 1 Obelisk \
        from your supply to a Rune Stone, which counts as 1 objective victory point.",
    )
    .add_bool_request(
        |event| &mut event.combat_end,
        24,
        |game, p, s| {
            (s.player(p.index).fighter_losses(s.battleground).amount() >= 2
                && p.get(game).is_building_available(Building::Obelisk, game))
            .then_some("Do you want to convert an Obelisk to a Rune Stone?".to_string())
        },
        move |game, s, _| {
            if s.choice {
                let p = game.player_mut(s.player_index);
                p.destroyed_structures.add_building(Building::Obelisk);
                p.gain_objective_victory_points(1.0, &s.origin);
                s.log(
                    game,
                    "Converted an Obelisk to a Rune Stone for 1 objective point",
                );
            } else {
                s.log(game, "Did not convert an Obelisk to a Rune Stone");
            }
        },
    )
    .build()
}

fn knut() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Knut,
        "Knut the Great",
        ruler_of_the_north(),
        danegeld(),
    )
}

fn danegeld() -> LeaderAbility {
    LeaderAbility::builder(
        "Danegeld",
        "If you have Taxes: Once per turn, as an action, you may activate the leader city: \
        Collect taxes (cannot be used in the same turn as the regular Tax action)",
    )
    .add_custom_action(
        CustomActionType::Danegeld,
        |c| {
            c.once_per_turn_mutually_exclusive(CustomActionType::Taxes)
                .action()
                .no_resources()
        },
        |b| {
            use_taxes(b.add_simple_persistent_event_listener(
                |event| &mut event.custom_action,
                -1,
                |game, player, _| {
                    activate_leader_city(game, player);
                },
            ))
        },
        |game, p| can_activate_leader_city(game, p) && p.can_use_advance(Advance::Taxes),
    )
    .build()
}

fn ruler_of_the_north() -> LeaderAbility {
    LeaderAbility::builder(
        "Ruler of the North",
        "Knut is worth half a point per distance to your capital",
    )
    .add_transient_event_listener(
        |event| &mut event.dynamic_victory_points,
        0,
        move |s, game, (), player| {
            let p = player.get(game);
            let points = if p.active_leader().is_some_and(|l| l == Leader::Knut) {
                leader_position(p).distance(capital_city_position(game, p)) as f32 / 2.0
            } else {
                0_f32
            };
            s.push(SpecialVictoryPoints::new(
                points,
                player.origin.clone(),
                VictoryPointAttribution::Events,
            ));
        },
    )
    .build()
}

fn erik() -> LeaderInfo {
    LeaderInfo::new(Leader::Erik, "Erik the Red", explorer(), new_colonies())
}

fn new_colonies() -> LeaderAbility {
    LeaderAbility::builder(
        "New Colonies",
        "As a free action, if Erik is on a ship: \
        Unload all units to an adjacent land space, which may start a battle",
    )
    .add_custom_action(
        CustomActionType::NewColonies,
        |c| c.any_times().free_action().no_resources(),
        |b| {
            b.add_position_request(
                |event| &mut event.custom_action,
                0,
                |game, p, _| {
                    Some(PositionRequest::new(
                        unload_positions(game, p.get(game)),
                        1..=1,
                        "Select a land position to unload Erik's ship(s)",
                    ))
                },
                |game, s, _a| {
                    let to = s.choice[0];
                    let p = game.player(s.player_index);
                    let units = p
                        .get_units(leader_position(p))
                        .iter()
                        .filter(|u| !u.is_ship())
                        .map(|u| u.id)
                        .collect_vec();

                    let m = MoveUnits::new(units, to, None, ResourcePile::empty());
                    s.log(game, &move_action_log(game, p, &m));

                    move_with_possible_combat(game, s.player_index, &m);
                },
            )
        },
        |game, p| !unload_positions(game, p).is_empty(),
    )
    .build()
}

fn unload_positions(game: &Game, p: &Player) -> Vec<Position> {
    let position = leader_position(p);
    if !game.map.is_sea(position) {
        return vec![];
    }
    position
        .neighbors()
        .into_iter()
        .filter(|n| game.map.is_land(*n))
        .collect_vec()
}

fn explorer() -> LeaderAbility {
    LeaderAbility::builder(
        "Legendary Explorer",
        "As an action, if Erik is on a ship, pay 1 culture token: \
        Place an explorer token a region that has no explorer token yet. \
        Each token is worth 1 objective point at the end of the game.",
    )
    .add_custom_action(
        CustomActionType::LegendaryExplorer,
        |c| {
            c.any_times()
                .action()
                .resources(ResourcePile::culture_tokens(1))
        },
        use_legendary_explorer,
        |game, p| {
            let position = leader_position(p);
            game.map.is_sea(position) && !is_current_block_tagged(game, p, position)
        },
    )
    .build()
}

fn use_legendary_explorer(b: AbilityBuilder) -> AbilityBuilder {
    b.add_simple_persistent_event_listener(
        |event| &mut event.custom_action,
        0,
        move |game, p, _| {
            let player = p.get_mut(game);
            let position = leader_position(player);

            add_special_victory_points(player, 1.0, &p.origin, VictoryPointAttribution::Objectives);

            player
                .custom_data
                .entry(String::from("explorer"))
                .or_insert(Data::Positions(Vec::new()))
                .positions_mut()
                .push(position);
            p.log(game, &format!("Place an explorer token at {position}"));
        },
    )
}

fn is_current_block_tagged(game: &Game, player: &Player, position: Position) -> bool {
    let block_position = block_for_position(game, position).1;
    player.custom_data.get("explorer").is_some_and(|data| {
        data.positions()
            .iter()
            .any(|p| block_for_position(game, *p).1 == block_position)
    })
}

#[must_use]
pub fn has_explore_token(game: &Game, position: Position) -> bool {
    game.players.iter().any(|player| {
        player
            .custom_data
            .get("explorer")
            .is_some_and(|v| v.positions().contains(&position))
    })
}

fn ragnar() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Ragnar,
        "Ragnar Lodbrok",
        LeaderAbility::builder("Prey", "When you captured a non-Angry city: Gain 2 gold")
            .add_simple_persistent_event_listener(
                |event| &mut event.combat_end,
                25,
                |game, player, s| {
                    if s.attacker.survived_leader()
                        && s.captured_city(player.index)
                            .is_some_and(|m| m != MoodState::Angry)
                    {
                        player.gain_resources(game, ResourcePile::gold(2));
                    }
                },
            )
            .build(),
        LeaderAbility::builder(
            "Horror of the North",
            "In leader battles when disembarking: Get +2 combat value",
        )
        .add_combat_strength_listener(106, |game, c, s, r| {
            if c.has_leader(r, game) && c.is_disembarking_attacker(r, game) {
                s.extra_combat_value += 2;
                s.roll_log.push("Ragnar adds 2 combat value".to_string());
            }
        })
        .build(),
    )
}
