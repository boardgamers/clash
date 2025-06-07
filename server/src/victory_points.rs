use serde::{Deserialize, Serialize};
use crate::events::EventOrigin;
use crate::player::Player;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Copy)]
pub enum VictoryPointAttribution {
    Events,
    Objectives,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialVictoryPoints {
    pub points: f32,
    pub origin: EventOrigin,
    pub attribution: VictoryPointAttribution,
}

pub(crate) fn add_special_victory_points(
    player: &mut Player,
    points: f32,
    origin: EventOrigin,
    attribution: VictoryPointAttribution,
) {
    player.special_victory_points.iter().position(|p| p.origin == origin).map_or_else(
        || player.special_victory_points.push(SpecialVictoryPoints {
            points: assert_positive(points),
            origin,
            attribution,
        }),
        |v| player.special_victory_points[v].points = assert_positive(points + player.special_victory_points[v].points),
    );
}

fn assert_positive(points: f32) -> f32 {
    if points <= 0.0 {
        panic!("Victory points cannot be negative: {}", points);
    }
}