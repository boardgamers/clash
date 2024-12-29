use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    advance::Advance,
    content::advances,
    game::{Game, GameState::*},
    player::Player,
    position::Position,
    resource_pile::ResourcePile,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct ChangeGovernment {
    pub new_government: String,
    pub additional_advances: Vec<String>,
}

// Can't use Option<String> because of mongo stips null values
#[derive(Serialize, Deserialize, Clone)]
pub enum ChangeGovernmentType {
    ChangeGovernment(ChangeGovernment),
    KeepGovernment,
}

// Can't use Option<String> because of mongo stips null values
#[derive(Serialize, Deserialize, Clone)]
pub enum RazeSize1City {
    None,
    Position(Position),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum StatusPhaseAction {
    CompleteObjectives(Vec<String>),
    FreeAdvance(String),
    RazeSize1City(RazeSize1City),
    ChangeGovernmentType(ChangeGovernmentType),
    DetermineFirstPlayer(usize),
}

impl StatusPhaseAction {
    #[must_use]
    pub fn phase(&self) -> StatusPhaseState {
        match self {
            StatusPhaseAction::CompleteObjectives(_) => StatusPhaseState::CompleteObjectives,
            StatusPhaseAction::FreeAdvance(_) => StatusPhaseState::FreeAdvance,
            StatusPhaseAction::RazeSize1City(_) => StatusPhaseState::RazeSize1City,
            StatusPhaseAction::ChangeGovernmentType(_) => StatusPhaseState::ChangeGovernmentType,
            StatusPhaseAction::DetermineFirstPlayer(_) => StatusPhaseState::DetermineFirstPlayer,
        }
    }

    /// # Panics
    /// Panics if the action is not legal
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match self {
            StatusPhaseAction::CompleteObjectives(ref completed_objectives) => {
                //todo legality check
                game.players[player_index]
                    .completed_objectives
                    .append(&mut completed_objectives.clone());
            }
            StatusPhaseAction::FreeAdvance(ref advance) => {
                assert!(
                    game.players[player_index].can_advance_free(advance),
                    "Illegal action"
                );
                game.advance(advance, player_index);
            }
            StatusPhaseAction::RazeSize1City(ref city) => {
                if let RazeSize1City::Position(city) = *city {
                    assert!(
                        game.players[player_index].can_raze_city(city),
                        "Illegal action"
                    );
                    game.raze_city(city, player_index);
                    game.players[player_index].gain_resources(ResourcePile::gold(1));
                }
            }
            StatusPhaseAction::ChangeGovernmentType(ref new_government_advance) => {
                if let ChangeGovernmentType::ChangeGovernment(new_government) =
                    new_government_advance
                {
                    change_government_type(game, player_index, new_government);
                }
            }
            StatusPhaseAction::DetermineFirstPlayer(ref player) => {
                game.starting_player_index = *player;
                game.next_age();
                return;
            }
        }
        game.next_player();
        skip_status_phase_players(game);
    }
}

fn change_government_type(game: &mut Game, player_index: usize, new_government: &ChangeGovernment) {
    let government = &new_government.new_government;
    if advances::get_leading_government_advance(government)
        .expect("government should exist")
        .required
        .is_some_and(|required_advance| !game.players[player_index].has_advance(&required_advance))
    {
        panic!("Player doesn't have the required advance for the government");
    }
    let current_player_government = game.players[player_index]
        .government()
        .expect("player should have a government");
    let player_government_advances = advances::get_government(&current_player_government)
        .into_iter()
        .filter(|advance| game.players[player_index].has_advance(&advance.name))
        .collect::<Vec<Advance>>();

    assert_eq!(
        player_government_advances.len() - 1,
        new_government.additional_advances.len(),
        "Illegal number of additional advances"
    );

    for advance in player_government_advances {
        game.remove_advance(&advance.name, player_index);
    }

    let new_government_advances = advances::get_government(government);
    game.advance(&new_government_advances[0].name, player_index);
    for advance in &new_government.additional_advances {
        let (pos, advance) = new_government_advances
            .iter()
            .find_position(|a| a.name == *advance)
            .expect("advance should exist");
        assert!(
            pos > 0,
            "Additional advances should not include the leading government advance"
        );
        game.advance(&advance.name, player_index);
    }
}

fn next_phase(game: &mut Game, phase: Option<StatusPhaseState>) -> StatusPhaseState {
    if let Some(StatusPhaseState::FreeAdvance) = phase {
        //draw card phase
        game.draw_new_cards();
    }
    let next_phase = next_status_phase(phase);
    if let StatusPhaseState::DetermineFirstPlayer = next_phase {
        game.set_player_index(player_that_chooses_next_first_player(
            &game.players,
            game.starting_player_index,
            &game.dropped_players,
        ));
    }
    game.state = StatusPhase(next_phase.clone());
    next_phase
}

/// # Panics
///
/// Panics if the game state is not valid
pub fn skip_status_phase_players(game: &mut Game) {
    let mut phase = match game.state {
        StatusPhase(ref phase) => Some(phase.clone()),
        _ => None,
    };

    loop {
        if game.active_player() == game.starting_player_index {
            phase = Some(next_phase(game, phase));
        }

        game.skip_dropped_players();

        if !skip_player(
            game,
            game.active_player(),
            phase.as_ref().expect("phase should be set"),
        ) {
            return;
        }
        game.increment_player_index();
    }
}

fn skip_player(game: &Game, player_index: usize, state: &StatusPhaseState) -> bool {
    let player = &game.players[player_index];
    match state {
        StatusPhaseState::CompleteObjectives => true, //todo only skip player if the doesn't have objective cards in his hand (don't skip if the can't complete them unless otherwise specified via setting)
        StatusPhaseState::FreeAdvance => !advances::get_all()
            .into_iter()
            .any(|advance| player.can_advance_free(&advance.name)),
        StatusPhaseState::RazeSize1City => !player.cities.iter().any(|city| city.size() == 1),
        StatusPhaseState::ChangeGovernmentType => {
            player.government().is_none()
                || player.government().is_some_and(|government| {
                    !advances::get_governments()
                        .iter()
                        .any(|(g, a)| g != &government && player.can_advance(a))
                })
        }
        StatusPhaseState::DetermineFirstPlayer => false,
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum StatusPhaseState {
    CompleteObjectives,
    FreeAdvance,
    //draw new cards (after free advance)
    RazeSize1City,
    ChangeGovernmentType,
    DetermineFirstPlayer,
}

#[must_use]
pub fn next_status_phase(phase: Option<StatusPhaseState>) -> StatusPhaseState {
    use StatusPhaseState::*;
    if let Some(phase) = phase {
        match phase {
            CompleteObjectives => FreeAdvance,
            FreeAdvance => RazeSize1City,
            RazeSize1City => ChangeGovernmentType,
            ChangeGovernmentType => DetermineFirstPlayer,
            DetermineFirstPlayer => {
                unreachable!("function should return early with this action")
            }
        }
    } else {
        CompleteObjectives
    }
}

/// # Panics
/// Panics if the game state is not valid
pub fn player_that_chooses_next_first_player(
    players: &[Player],
    current_start_player_index: usize,
    dropped_players: &[usize],
) -> usize {
    fn score(player: &Player) -> u32 {
        player.resources.mood_tokens + player.resources.culture_tokens
    }

    let best = players
        .iter()
        .filter(|player| !dropped_players.contains(&player.index))
        .map(score)
        .max()
        .expect("no player found");
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
