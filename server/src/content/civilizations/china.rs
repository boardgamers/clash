use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::content::persistent_events::PositionRequest;
use crate::map::Terrain;
use crate::player::Player;
use crate::playing_actions::Recruit;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{
    SpecialAdvance, SpecialAdvanceBuilder, SpecialAdvanceInfo, SpecialAdvanceRequirement,
};

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
    let mut b = SpecialAdvanceInfo::builder(
        SpecialAdvance::Expansion,
        SpecialAdvanceRequirement::Advance(Advance::Husbandry),
        "Expansion",
        "When you recruited at least 1 settler: Every settler in your cities gains one move",
    );
    for i in 0..4 {
        b = add_settler_move(b, 10 + i * 2);
    }

    b.build()
}

fn add_settler_move(b: SpecialAdvanceBuilder, prio: i32) -> SpecialAdvanceBuilder {
    b.add_position_request(
        |event| &mut event.recruit,
        prio + 1,
        |game, player_index, r| {
            Some(PositionRequest::new(
                movable_settlers(game.player(player_index), r),
                1..=1,
                "Select a settler to move (may select the current tile)",
            ))
        },
        |game, s, r| {
            let pos = s.choice[0];
            game.add_info_log_item(&format!(
                "{} selected settler at {} to move",
                s.player_name, pos
            ));
            r.selected_position = Some(pos);
        },
    )
    .add_position_request(
        |event| &mut event.recruit,
        prio,
        |game, player_index, r| {
            r.selected_position.map(|pos| {
                PositionRequest::new(
                    game.player(player_index).available_moves(pos),
                    1..=1,
                    "Select a tile to move the settler to",
                )
            })
        },
        |game, s, r| {
            let pos = s.choice[0];
            if pos == r.selected_position.expect("Selected position") {
                game.add_info_log_item(&format!(
                    "{} selected the settler at {pos} to stay in place",
                    s.player_name
                ))
            } else {
                game.add_info_log_item(&format!(
                    "{} selected {pos} to move the settler to",
                    s.player_name
                ));
                // todo move
                // r.moved_units // todo
            }
        },
    )
}

fn movable_settlers(player: &Player, r: &Recruit) -> Vec<Position> {
    // todo 1) in city 2) no move restrictions
    player
        .units
        .iter()
        .filter(|u| u.unit_type.is_settler() && u.movement_restrictions.contains())
        .map(|u| u.position)
        .collect()
}
