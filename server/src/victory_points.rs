use crate::consts::{
    ADVANCE_VICTORY_POINTS, BUILDING_VICTORY_POINTS, CAPTURED_LEADER_VICTORY_POINTS,
    OBJECTIVE_VICTORY_POINTS,
};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::player::Player;
use crate::player_events::trigger_event_with_game_info;
use crate::wonder::{wonders_built_points, wonders_owned_points};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::mem;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy)]
pub enum VictoryPointAttribution {
    Events,
    Objectives,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecialVictoryPoints {
    pub points: f32,
    pub origin: EventOrigin,
    pub attribution: VictoryPointAttribution,
    #[serde(default)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dynamic: bool,
}

impl SpecialVictoryPoints {
    #[must_use]
    pub fn new(points: f32, origin: EventOrigin, attribution: VictoryPointAttribution) -> Self {
        Self {
            points,
            origin,
            attribution,
            dynamic: false,
        }
    }
}

pub(crate) fn add_special_victory_points(
    player: &mut Player,
    points: f32,
    origin: &EventOrigin,
    attribution: VictoryPointAttribution,
) {
    update_special_victory_points(player, origin, attribution, false, |v| v + points);
}

pub(crate) fn add_dynamic_victory_points(game: &mut Game, player_index: usize) {
    let mut dynamic_victory_points: Vec<SpecialVictoryPoints> = vec![];
    trigger_event_with_game_info(
        game,
        player_index,
        |event| &mut event.transient.dynamic_victory_points,
        &mut dynamic_victory_points,
        &(),
        &mut (),
    );
    let player = game.player_mut(player_index);
    let mut used = vec![];
    for v in dynamic_victory_points {
        let origin = &v.origin;
        used.push(origin.clone());
        update_special_victory_points(player, origin, v.attribution, true, |_| v.points);
    }
    // prune all dynamic that are not used anymore
    player
        .special_victory_points
        .retain(|p| !p.dynamic || used.contains(&p.origin));
}

fn update_special_victory_points(
    player: &mut Player,
    origin: &EventOrigin,
    attribution: VictoryPointAttribution,
    dynamic: bool,
    update: impl Fn(f32) -> f32,
) {
    if let Some(v) = player
        .special_victory_points
        .iter()
        .position(|p| &p.origin == origin)
    {
        player.special_victory_points[v].points =
            update(mem::take(&mut player.special_victory_points[v].points));
    } else {
        let mut points = SpecialVictoryPoints::new(update(0.0), origin.clone(), attribution);
        points.dynamic = dynamic;
        player.special_victory_points.push(points);
    }
    player.special_victory_points.retain(|p| p.points > 0.0);
}

pub(crate) fn special_victory_points(p: &Player, attribution: VictoryPointAttribution) -> f32 {
    p.special_victory_points
        .iter()
        .filter(|v| v.attribution == attribution)
        .map(|v| v.points)
        .sum()
}

#[must_use]
pub fn victory_points_parts(player: &Player, game: &Game) -> [(&'static str, f32); 6] {
    [
        (
            "City pieces",
            (player.cities.len() + player.owned_buildings(game)) as f32 * BUILDING_VICTORY_POINTS,
        ),
        (
            "Advances",
            (player.advances.len() + player.special_advances.len()) as f32 * ADVANCE_VICTORY_POINTS,
        ),
        (
            "Objectives",
            player.completed_objectives.len() as f32 * OBJECTIVE_VICTORY_POINTS
                + special_victory_points(player, VictoryPointAttribution::Objectives),
        ),
        (
            "Wonders",
            wonders_owned_points(player, game) as f32 + wonders_built_points(player, game),
        ),
        (
            "Events",
            special_victory_points(player, VictoryPointAttribution::Events),
        ),
        (
            "Captured Leaders",
            player.captured_leaders.len() as f32 * CAPTURED_LEADER_VICTORY_POINTS,
        ),
    ]
}

#[must_use]
pub(crate) fn compare_score(player: &Player, other: &Player, game: &Game) -> Ordering {
    use std::cmp::Ordering::{Equal, Greater, Less};

    let parts = victory_points_parts(player, game);
    let other_parts = victory_points_parts(other, game);
    let sum = parts.iter().map(|(_, v)| v).sum::<f32>();
    let other_sum = other_parts.iter().map(|(_, v)| v).sum::<f32>();

    match sum
        .partial_cmp(&other_sum)
        .expect("should be able to compare")
    {
        Less => return Less,
        Equal => (),
        Greater => return Greater,
    }

    for (part, other_part) in parts.iter().zip(other_parts.iter()) {
        match part
            .partial_cmp(other_part)
            .expect("should be able to compare")
        {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }
    }
    Equal
}
