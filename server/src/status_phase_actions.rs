use serde::{Deserialize, Serialize};
use StatusPhaseState::*;

use crate::{
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
                let completed_objectives = serde_json::from_str::<CompleteObjectives>(&self.data)
                    .expect("data should be valid complete objectives json")
                    .objectives;
                todo!()
            }
            StatusPhaseState::FreeAdvance => {
                let advance = serde_json::from_str::<FreeAdvance>(&self.data)
                    .expect("data should be valid free advance json")
                    .advance;
                if !player.can_advance_free(&advance) {
                    panic!("Illegal action");
                }
                player.advance(&advance, game);
            }
            StatusPhaseState::RaseSize1City => {
                let city = serde_json::from_str::<RaseSize1City>(&self.data)
                    .expect("data should be valid rase city json")
                    .city;
                if let Some(city) = city {
                    player.raze_city(&city, game);
                }
            }
            StatusPhaseState::ChangeGovernmentType => {
                let new_government_advance =
                    serde_json::from_str::<ChangeGovernmentType>(&self.data)
                        .expect("data should be valid change government type json")
                        .new_government_advance;
                if let Some(new_government_advance) = new_government_advance {
                    todo!()
                }
            }
            StatusPhaseState::DetermineFirstPlayer => {
                let player = serde_json::from_str::<DetermineFirstPlayer>(&self.data)
                    .expect("data should be valid determine first player json")
                    .player;
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

pub fn next_status_phase(phase: StatusPhaseState) -> StatusPhaseState {
    match phase {
        CompleteObjectives => FreeAdvance,
        FreeAdvance => RaseSize1City,
        RaseSize1City => ChangeGovernmentType,
        ChangeGovernmentType => DetermineFirstPlayer,
        DetermineFirstPlayer => {
            unreachable!("function should return early with this action")
        }
    }
}

pub fn player_that_chooses_next_first_player(
    players: &Vec<Player>,
    current_start_player: usize,
) -> usize {
    let mut potential_deciding_players = Vec::new();
    let mut best_total: Option<u32> = None;
    for (i, player) in players.iter().enumerate() {
        let total = player.resources().mood_tokens + player.resources().culture_tokens;
        match best_total {
            None => {
                potential_deciding_players.push(i);
                best_total = Some(total);
            }
            Some(t) if total > t => {
                potential_deciding_players.clear();
                best_total = Some(total);
                potential_deciding_players.push(i);
            }
            Some(t) if total == t => {
                potential_deciding_players.push(i);
            }
            Some(_) => {}
        }
    }
    potential_deciding_players
        .into_iter()
        .min_by_key(|&index| {
            (index as isize - current_start_player as isize).rem_euclid(players.len() as isize)
        })
        .expect("there should at least be one player with the most mood and culture tokens")
}

#[cfg(test)]
mod tests {
    use crate::content;
    use crate::player::Player;
    use crate::resource_pile::ResourcePile;
    use content::civilizations::tests as civ;

    fn assert_next_player(
        name: &str,
        player0_mood: u32,
        player1_mood: u32,
        player2_mood: u32,
        expected_player: usize,
    ) {
        let mut player0 = Player::new(civ::get_test_civilization(), 0);
        player0.gain_resources(ResourcePile::mood_tokens(player0_mood));
        let mut player1 = Player::new(civ::get_test_civilization(), 1);
        player1.gain_resources(ResourcePile::mood_tokens(player1_mood));
        let mut player2 = Player::new(civ::get_test_civilization(), 2);
        player2.gain_resources(ResourcePile::mood_tokens(player2_mood));
        let players = vec![player0, player1, player2];
        let got = super::player_that_chooses_next_first_player(&players, 1);
        assert_eq!(got, expected_player, "{name}");
    }

    #[test]
    fn test_player_that_chooses_next_first_player() {
        assert_next_player("player 0 has more mood", 1, 0, 0, 0);
        assert_next_player("player 1 has more mood", 0, 1, 0, 1);
        assert_next_player("tie between 0 and 1 - player 1 stays", 1, 1, 0, 1);
        assert_next_player(
            "tie between 0 and 2 - player 2 is the next player after the current first player",
            1,
            0,
            1,
            2,
        );
    }
}
