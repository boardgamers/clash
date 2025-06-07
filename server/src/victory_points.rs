use crate::events::EventOrigin;
use crate::player::Player;
use serde::{Deserialize, Serialize};

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
}

pub(crate) fn add_special_victory_points(
    player: &mut Player,
    points: f32,
    origin: &EventOrigin,
    attribution: VictoryPointAttribution,
) {
    if let Some(v) = player
        .special_victory_points
        .iter()
        .position(|p| &p.origin == origin)
    {
        player.special_victory_points[v].points =
            assert_positive(points + player.special_victory_points[v].points);
    } else {
        player.special_victory_points.push(SpecialVictoryPoints {
            points: assert_positive(points),
            origin: origin.clone(),
            attribution,
        });
    }
}

fn assert_positive(points: f32) -> f32 {
    assert!(points >= 0.0, "Victory points cannot be negative: {points}");
    points
}

pub(crate) fn special_victory_points(p: &Player, attribution: VictoryPointAttribution) -> f32 {
    p.special_victory_points
        .iter()
        .filter(|v| v.attribution == attribution)
        .map(|v| v.points)
        .sum()
}
