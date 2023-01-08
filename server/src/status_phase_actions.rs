use serde::{Deserialize, Serialize};

use crate::{
    city::CityData,
    game::{Game, StatusPhaseState},
    hexagon::Position,
    player::Player,
};

#[derive(Serialize, Deserialize)]
pub struct StatusPhaseAction {
    data: String,
    phase: StatusPhaseState,
}

impl StatusPhaseAction {
    pub fn new(data: String, phase: StatusPhaseState) -> Self {
        Self { data, phase }
    }

    pub fn execute(self, player: &mut Player, game: &mut Game) {
        match self.phase {
            StatusPhaseState::CompleteObjectives => {
                let completed_objectives =
                    serde_json::from_str::<CompleteObjectives>(&self.data)
                        .expect("data should be valid complete objectives json")
                        .objectives;
                todo!()
            }
            StatusPhaseState::FreeAdvance => {
                let advance = serde_json::from_str::<FreeAdvance>(&self.data).expect("data should be valid free advance json").advance;
                if !player.can_advance_free(&advance) {
                    panic!("Illegal action");
                }
                player.advance(&advance);
            }
            StatusPhaseState::RaseSize1City => {
                let city = serde_json::from_str::<RaseSize1City>(&self.data).expect("data should be valid rase city json").city;
                if let Some(city) = city {
                    player.raze_city(&city, game);
                }
            }
            StatusPhaseState::ChangeGovernmentType => {
                let new_government_advance = serde_json::from_str::<ChangeGovernmentType>(&self.data).expect("data should be valid change government type json").new_government_advance;
                if let Some(new_government_advance) = new_government_advance {
                    todo!()
                }
            }
            StatusPhaseState::DetermineFirstPlayer => {
                let player = serde_json::from_str::<DetermineFirstPlayer>(&self.data).expect("data should be valid determine first player json").player;
                game.starting_player = player;
                game.current_player = player;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CompleteObjectives {
    objectives: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FreeAdvance {
    advance: String,
}

#[derive(Serialize, Deserialize)]
pub struct RaseSize1City {
    city: Option<Position>,
}

#[derive(Serialize, Deserialize)]
pub struct ChangeGovernmentType {
    new_government_advance: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DetermineFirstPlayer {
    player: usize,
}
