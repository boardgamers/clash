use serde::{Deserialize, Serialize};
use StatusPhaseState::*;

use crate::{
    advance::Advance,
    content::advances,
    game::{Game, GameState::*},
    player::Player,
    position::Position,
    resource_pile::ResourcePile,
};

#[derive(Serialize, Deserialize)]
pub struct StatusPhaseAction {
    pub data: String,
    pub phase: StatusPhaseState,
}

impl StatusPhaseAction {
    pub fn new(data: String, phase: StatusPhaseState) -> Self {
        Self { data, phase }
    }

    pub fn execute(self, game: &mut Game, player_index: usize) {
        match self.phase {
            StatusPhaseState::CompleteObjectives => {
                let mut completed_objectives =
                    serde_json::from_str::<CompleteObjectives>(&self.data)
                        .expect("data should be valid complete objectives json")
                        .objectives;
                //todo legality check
                game.players[player_index]
                    .completed_objectives
                    .append(&mut completed_objectives);
            }
            StatusPhaseState::FreeAdvance => {
                let advance = serde_json::from_str::<FreeAdvance>(&self.data)
                    .expect("data should be valid free advance json")
                    .advance;
                if !game.players[player_index].can_advance_free(&advance) {
                    panic!("Illegal action");
                }
                game.advance(&advance, player_index);
            }
            StatusPhaseState::RaseSize1City => {
                let city = serde_json::from_str::<RaseSize1City>(&self.data)
                    .expect("data should be valid rase city json")
                    .city;
                if let Some(city) = city {
                    game.raze_city(&city, player_index);
                    game.players[player_index].gain_resources(ResourcePile::gold(1));
                }
            }
            StatusPhaseState::ChangeGovernmentType => {
                let new_government_advance =
                    serde_json::from_str::<ChangeGovernmentType>(&self.data)
                        .expect("data should be valid change government type json")
                        .new_government;
                if let Some(new_government) = new_government_advance {
                    if !advances::get_leading_government_advance(&new_government)
                        .expect("government should exist")
                        .required_advance
                        .is_some_and(|required_advance| {
                            !game.players[player_index].has_advance(&required_advance)
                        })
                    {
                        panic!("Illegal action");
                    }
                    let current_player_government = game.players[player_index]
                        .government()
                        .expect("player should have a government");
                    let player_government_advances =
                        advances::get_government_advances(&current_player_government)
                            .into_iter()
                            .enumerate()
                            .filter(|(_, advance)| {
                                game.players[player_index].has_advance(&advance.name)
                            })
                            .collect::<Vec<(usize, Advance)>>();
                    let new_government_advances =
                        advances::get_government_advances(&new_government);
                    for (tier, advance) in player_government_advances.into_iter() {
                        game.remove_advance(&advance.name, player_index);
                        game.advance(&new_government_advances[tier].name, player_index)
                    }
                }
            }
            StatusPhaseState::DetermineFirstPlayer => {
                let player = serde_json::from_str::<DetermineFirstPlayer>(&self.data)
                    .expect("data should be valid determine first player json")
                    .player_index;
                game.starting_player_index = player;
                game.next_age();
                return;
            }
        }
        game.next_player();
        if game.current_player_index == game.starting_player_index {
            next_phase(game, &self.phase);
            return;
        }
        skip_status_phase_players(game, &self.phase);
    }

}

fn next_phase(game: &mut Game, phase: &StatusPhaseState) {
    if let StatusPhaseState::FreeAdvance = phase {
        //draw card phase
        game.draw_new_cards();
    }
    let next_phase = next_status_phase(phase);
    if let StatusPhaseState::DetermineFirstPlayer = next_phase {
        game.current_player_index = player_that_chooses_next_first_player(
            &game.players,
            game.starting_player_index,
            &game.dropped_players,
        );
    }
    game.state = StatusPhase(next_phase.clone());
    skip_status_phase_players(game, &next_phase);
}

fn skip_status_phase_players(game: &mut Game, phase: &StatusPhaseState) {
    while game.current_player_index != game.starting_player_index {
        game.skip_dropped_players();
        if !skip_player(game, game.current_player_index, phase) {
            return;
        }
        game.current_player_index += 1;
        game.current_player_index %= game.players.len();
    }
    next_phase(game, phase);
}

fn skip_player(game: &Game, player_index: usize, state: &StatusPhaseState) -> bool {
    let player = &game.players[player_index];
    match state {
        StatusPhaseState::CompleteObjectives => true, //todo only skip player if the does'nt have objective cards in his hand (don't skip if the can't complete them unless otherwise specified via setting)
        StatusPhaseState::FreeAdvance => advances::get_all_advances().into_iter().all(|advance| !player.can_advance_free(&advance.name)),
        StatusPhaseState::RaseSize1City => !player.cities.iter().any(|city| city.size() == 1),
        StatusPhaseState::ChangeGovernmentType => player.government().is_some() && !advances::get_all_advances().into_iter().any(|advance| !advance.required_advance.is_some_and(|required_advance| !player.has_advance(&required_advance)) && advance.government.is_some_and(|government| government != player.government().expect("player should have government due to previous check"))),
        StatusPhaseState::DetermineFirstPlayer => false,
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum StatusPhaseState {
    CompleteObjectives,
    FreeAdvance,
    //draw new cards (after free advance)
    RaseSize1City,
    ChangeGovernmentType,
    DetermineFirstPlayer,
}

#[derive(Serialize, Deserialize)]
pub struct CompleteObjectives {
    pub objectives: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FreeAdvance {
    pub advance: String,
}

#[derive(Serialize, Deserialize)]
pub struct RaseSize1City {
    pub city: Option<Position>,
}

#[derive(Serialize, Deserialize)]
pub struct ChangeGovernmentType {
    pub new_government: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DetermineFirstPlayer {
    pub player_index: usize,
}

pub fn next_status_phase(phase: &StatusPhaseState) -> StatusPhaseState {
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
    current_start_player_index: usize,
    dropped_players: &[usize],
) -> usize {
    fn score(player: &Player) -> u32 {
        player.resources().mood_tokens + player.resources().culture_tokens
    }

    let best = players.iter().filter(|player| !dropped_players.contains(&player.index)).map(score).max().expect("no player found");
    players
        .iter()
        .filter(|p| score(p) == best && !dropped_players.contains(&p.index))
        .min_by_key(|&p| {
            (p.index as isize - current_start_player_index as isize)
                .rem_euclid(players.len() as isize)
        })
        .expect("there should at least be one player with the most mood and culture tokens")
        .index
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
        expected_player_index: usize,
    ) {
        let mut player0 = Player::new(civ::get_test_civilization(), 0);
        player0.gain_resources(ResourcePile::mood_tokens(player0_mood));
        let mut player1 = Player::new(civ::get_test_civilization(), 1);
        player1.gain_resources(ResourcePile::mood_tokens(player1_mood));
        let mut player2 = Player::new(civ::get_test_civilization(), 2);
        player2.gain_resources(ResourcePile::mood_tokens(player2_mood));
        let players = vec![player0, player1, player2];
        let got = super::player_that_chooses_next_first_player(&players, 1, &[]);
        assert_eq!(got, expected_player_index, "{name}");
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
