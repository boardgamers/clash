use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::game::{Game, GameState};
use crate::map::Terrain;
use crate::movement::{MoveState, possible_move_destinations};
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use itertools::Itertools;

pub(crate) fn china() -> Civilization {
    Civilization::new("China", vec![rice(), expansion()], vec![])
}

fn rice() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::RiceCultivation,
        SpecialAdvanceRequirement::Advance(Advance::Irrigation),
        "Rice Cultivation",
        "Collect add additional +1 food from up to 2 Grassland spaces outside your city, \
        if occupied by one of your settlers.",
    )
    .add_transient_event_listener(
        |event| &mut event.collect_total,
        2,
        |i, game, collections| {
            let city = game.get_any_city(i.city);
            let food = collections
                .iter()
                .filter(|c| {
                    let pos = c.position;
                    pos.distance(city.position) > 0
                        && game.map.get(pos) == Some(&Terrain::Fertile)
                        && game
                            .player(i.info.player)
                            .units
                            .iter()
                            .any(|u| u.position == pos && u.unit_type.is_settler())
                })
                .count()
                .min(2);
            i.total += ResourcePile::food(food as u8);
            i.info.log.push(format!(
                "Rice Cultivation added {}",
                ResourcePile::food(food as u8)
            ));
        },
    )
    .build()
}

fn expansion() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Expansion,
        SpecialAdvanceRequirement::Advance(Advance::Husbandry),
        "Expansion",
        "When you recruited at least 1 settler: Every settler in your cities gains one move",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.recruit,
        10,
        |game, player_index, _player_name, r| {
            if r.units.settlers == 0 {
                return;
            }

            let p = game.player(player_index);
            let settlers = movable_settlers(game, p);
            if settlers.is_empty() {
                return;
            }
            let moved_units = p
                .units
                .iter()
                .filter(|u| !u.unit_type.is_settler())
                .map(|u| u.id)
                .collect_vec();

            game.state = GameState::Movement(MoveState {
                moved_units,
                movement_actions_left: settlers.len() as u32,
                ..MoveState::default()
            });

            game.add_info_log_item(&format!(
                "Expansion allows to move the settlers at {}",
                settlers.iter().map(ToString::to_string).join(", ")
            ));
        },
    )
    .build()
}

fn movable_settlers(game: &Game, player: &Player) -> Vec<Position> {
    player
        .units
        .iter()
        .filter(|u| {
            player.try_get_city(u.position).is_some()
                && !possible_move_destinations(game, player.index, &[u.id], u.position)
                    .list
                    .is_empty()
        })
        .map(|u| u.position)
        .collect_vec()
}
