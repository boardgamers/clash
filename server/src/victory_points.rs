use crate::consts::{
    ADVANCE_VICTORY_POINTS, BUILDING_VICTORY_POINTS, CAPTURED_LEADER_VICTORY_POINTS,
    OBJECTIVE_VICTORY_POINTS,
};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::player::Player;
use crate::position::Position;
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
pub struct VictoryPointsWithTags {
    pub points: f32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub explorer_blocks: Vec<Position>,
}

impl VictoryPointsWithTags {
    fn assert_positive(&self) {
        assert!(
            self.points >= 0.0,
            "Victory points cannot be negative: {self:?}"
        );
    }

    #[must_use]
    fn set_points(mut self, points: f32) -> Self {
        self.points = points;
        self.assert_positive();
        self
    }

    #[must_use]
    fn add_points(mut self, points: f32) -> Self {
        self.points += points;
        self.assert_positive();
        self
    }
}

impl Default for VictoryPointsWithTags {
    fn default() -> Self {
        Self {
            points: 0.0,
            explorer_blocks: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecialVictoryPoints {
    #[serde(flatten)]
    pub points: VictoryPointsWithTags,
    pub origin: EventOrigin,
    pub attribution: VictoryPointAttribution,
}

impl SpecialVictoryPoints {
    #[must_use]
    pub fn new(
        points: VictoryPointsWithTags,
        origin: EventOrigin,
        attribution: VictoryPointAttribution,
    ) -> Self {
        points.assert_positive();
        Self {
            points,
            origin,
            attribution,
        }
    }
}

pub(crate) fn add_special_victory_points(
    player: &mut Player,
    points: f32,
    origin: &EventOrigin,
    attribution: VictoryPointAttribution,
) {
    update_special_victory_points(player, origin, attribution, |v| v.add_points(points));
}

pub(crate) fn set_special_victory_points(
    player: &mut Player,
    points: f32,
    origin: &EventOrigin,
    attribution: VictoryPointAttribution,
) {
    update_special_victory_points(player, origin, attribution, |v| v.set_points(points));
}

pub(crate) fn update_special_victory_points(
    player: &mut Player,
    origin: &EventOrigin,
    attribution: VictoryPointAttribution,
    update: impl Fn(VictoryPointsWithTags) -> VictoryPointsWithTags,
) {
    if let Some(v) = player
        .special_victory_points
        .iter()
        .position(|p| &p.origin == origin)
    {
        player.special_victory_points[v].points =
            update(mem::take(&mut player.special_victory_points[v].points));
    } else {
        player
            .special_victory_points
            .push(SpecialVictoryPoints::new(
                update(VictoryPointsWithTags::default()),
                origin.clone(),
                attribution,
            ));
    }
    player
        .special_victory_points
        .retain(|p| p.points.points > 0.0);
}

pub(crate) fn special_victory_points(p: &Player, attribution: VictoryPointAttribution) -> f32 {
    p.special_victory_points
        .iter()
        .filter(|v| v.attribution == attribution)
        .map(|v| v.points.points)
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
