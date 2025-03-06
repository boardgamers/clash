use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{advance_with_incident_token, do_advance, remove_advance};
use crate::consts::AGES;
use crate::content::builtin::{status_phase_handler, Builtin, BuiltinBuilder};
use crate::content::custom_phase_actions::{
    new_position_request, AdvanceRequest, ChangeGovernmentRequest, CurrentEventRequest,
    CurrentEventResponse, CurrentEventType, PlayerRequest,
};
use crate::payment::PaymentOptions;
use crate::{
    content::advances,
    game::{Game, GameState::*},
    player::Player,
    resource_pile::ResourcePile,
    utils,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum StatusPhaseState {
    CompleteObjectives,
    FreeAdvance,
    DrawCards,
    RazeSize1City,
    ChangeGovernmentType,
    DetermineFirstPlayer(usize),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ChangeGovernment {
    pub new_government: String,
    pub additional_advances: Vec<String>,
}

// Can't use Option<String> because of mongo stips null values
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ChangeGovernmentType {
    ChangeGovernment(ChangeGovernment),
    KeepGovernment,
}

pub const CHANGE_GOVERNMENT_COST: ResourcePile = ResourcePile::new(0, 0, 0, 0, 0, 1, 1);

pub(crate) fn enter_status_phase(game: &mut Game) {
    game.add_info_log_group(format!(
        "The game has entered the {} status phase",
        utils::ordinal_number(game.age)
    ));
    game.pop_state();
    game.push_state(StatusPhase(StatusPhaseState::CompleteObjectives));
    play_status_phase(game);
}

pub(crate) fn play_status_phase(game: &mut Game) {
    use StatusPhaseState::*;
    game.lock_undo();

    loop {
        let StatusPhase(phase) = game.state().clone() else {
            panic!("invalid state")
        };

        if game.trigger_current_event_with_listener(
            &game.human_players(game.starting_player_index),
            |events| &mut events.on_status_phase,
            &status_phase_handler(&phase).listeners,
            &(),
            |()| CurrentEventType::StatusPhase,
            None,
        ) {
            return;
        }

        let StatusPhase(phase) = game.pop_state() else {
            panic!("invalid state")
        };

        let next_phase = match phase {
            CompleteObjectives => {
                if game.age == AGES
                    || game
                        .players
                        .iter()
                        .filter(|player| player.is_human())
                        .any(|player| player.cities.is_empty())
                {
                    game.end_game();
                    return;
                }
                FreeAdvance
            }
            FreeAdvance => DrawCards,
            DrawCards => RazeSize1City,
            RazeSize1City => ChangeGovernmentType,
            ChangeGovernmentType => DetermineFirstPlayer(player_that_chooses_next_first_player(
                &game
                    .human_players(game.starting_player_index)
                    .into_iter()
                    .map(|p| game.get_player(p))
                    .collect_vec(),
            )),
            DetermineFirstPlayer(_) => {
                game.next_age();
                return;
            }
        };
        game.push_state(StatusPhase(next_phase));
    }
}

pub(crate) fn complete_objectives() -> Builtin {
    // not implemented
    Builtin::builder("Complete Objectives", "Complete objectives").build()
}

pub(crate) fn free_advance() -> Builtin {
    Builtin::builder("Free Advance", "Advance for free")
        .add_advance_reward_request_listener(
            |event| &mut event.on_status_phase,
            0,
            |game, player_index, _player_name| {
                let choices = advances::get_all()
                    .into_iter()
                    .filter(|advance| game.get_player(player_index).can_advance_free(advance))
                    .map(|a| a.name)
                    .collect_vec();
                Some(AdvanceRequest::new(choices))
            },
            |game, c| {
                game.add_info_log_item(&format!(
                    "{} advanced {} for free",
                    c.player_name, c.choice
                ));
                advance_with_incident_token(game, &c.choice, c.player_index, ResourcePile::empty());
            },
        )
        .build()
}

pub(crate) fn draw_cards() -> Builtin {
    Builtin::builder("Draw Cards", "-")
        .add_player_event_listener(
            |event| &mut event.on_status_phase,
            |_game, _p, ()| {
                // every player draws 1 action card and 1 objective card
            },
            0,
        )
        .build()
}

pub(crate) fn raze_city() -> Builtin {
    Builtin::builder("Raze city", "Raze size 1 city for 1 gold")
        .add_position_request(
            |event| &mut event.on_status_phase,
            0,
            |game, player_index, _player_name| {
                let player = game.get_player(player_index);
                let cities = player
                    .cities
                    .iter()
                    .filter(|city| city.size() == 1)
                    .map(|city| city.position)
                    .collect_vec();
                if cities.is_empty() {
                    return None;
                }
                Some(new_position_request(cities, 0..=1, None))
            },
            |game, s| {
                if s.choice.is_empty() {
                    game.add_info_log_item(&format!("{} did not raze a city", s.player_name));
                    return;
                }
                let pos = s.choice[0];
                game.add_info_log_item(&format!(
                    "{} razed the city at {pos} and gained 1 gold",
                    s.player_name
                ));
                game.raze_city(pos, s.player_index);
                game.players[s.player_index].gain_resources(ResourcePile::gold(1));
            },
        )
        .build()
}

pub(crate) fn may_change_government() -> Builtin {
    add_change_government(
        Builtin::builder("Change Government", "Change your government"),
        true,
        |g, player| {
            let p = g.get_player(player);
            p.can_afford(&PaymentOptions::resources(CHANGE_GOVERNMENT_COST))
                && can_change_government_for_free(p)
        },
    )
    .build()
}

pub(crate) fn add_change_government(
    b: BuiltinBuilder,
    optional: bool,
    pred: impl Fn(&Game, usize) -> bool + 'static + Clone,
) -> BuiltinBuilder {
    b.add_current_event_listener(
        |event| &mut event.on_status_phase,
        0,
        move |game, player_index, _, ()| {
            pred(game, player_index).then_some(CurrentEventRequest::ChangeGovernment(
                ChangeGovernmentRequest::new(optional),
            ))
        },
        move |game, player_index, player_name, action, request, ()| {
            if let CurrentEventRequest::ChangeGovernment(r) = &request {
                if let CurrentEventResponse::ChangeGovernmentType(t) = action {
                    match t {
                        ChangeGovernmentType::ChangeGovernment(c) => {
                            game.add_info_log_item(&format!(
                                "{player_name} changed their government from {} to {}",
                                game.players[game.active_player()]
                                    .government()
                                    .expect("player should have a government before changing it"),
                                c.new_government
                            ));
                            game.add_info_log_item(&format!(
                                "Additional advances: {}",
                                c.additional_advances.join(", ")
                            ));
                            change_government_type(game, player_index, &c);
                        }
                        ChangeGovernmentType::KeepGovernment => {
                            assert!(r.optional, "Must change government");
                            game.add_info_log_item(&format!(
                                "{player_name} did not change their government"
                            ));
                        }
                    }
                    return;
                }
            }
            panic!("Illegal action")
        },
    )
}

fn change_government_type(game: &mut Game, player_index: usize, new_government: &ChangeGovernment) {
    game.players[player_index].lose_resources(CHANGE_GOVERNMENT_COST);
    let government = &new_government.new_government;
    let a = advances::get_government(government).expect("government should exist");
    assert!(
        game.players[player_index].can_advance_in_change_government(&a.advances[0]),
        "Cannot advance in change government"
    );

    let current_player_government = game.players[player_index]
        .government()
        .expect("player should have a government");
    let player_government_advances = advances::get_government(&current_player_government)
        .expect("player should have a government")
        .advances
        .into_iter()
        .filter(|advance| game.players[player_index].has_advance(&advance.name))
        .collect_vec();

    assert_eq!(
        player_government_advances.len() - 1,
        new_government.additional_advances.len(),
        "Illegal number of additional advances"
    );

    for a in player_government_advances {
        remove_advance(game, &a, player_index);
    }

    let new_government_advances = advances::get_government(government)
        .expect("government should exist")
        .advances;
    do_advance(game, &new_government_advances[0], player_index);
    for name in &new_government.additional_advances {
        let (pos, advance) = new_government_advances
            .iter()
            .find_position(|a| a.name == *name)
            .unwrap_or_else(|| {
                panic!("Advance with name {name} not found in government advances");
            });
        assert!(
            pos > 0,
            "Additional advances should not include the leading government advance"
        );
        do_advance(game, advance, player_index);
    }
}

#[must_use]
pub(crate) fn can_change_government_for_free(player: &Player) -> bool {
    player.government().is_some_and(|government| {
        advances::get_governments().iter().any(|g| {
            g.government != Some(government.clone())
                && player.can_advance_in_change_government(&g.advances[0])
        })
    })
}

pub(crate) fn determine_first_player() -> Builtin {
    Builtin::builder("Determine First Player", "Determine the first player")
        .add_player_request(
            |event| &mut event.on_status_phase,
            0,
            |game, player_index, ()| {
                if let StatusPhase(StatusPhaseState::DetermineFirstPlayer(want)) = game.state() {
                    (*want == player_index).then_some(PlayerRequest::new(
                        game.human_players(game.starting_player_index),
                        "Determine the first player",
                    ))
                } else {
                    panic!("Illegal state")
                }
            },
            |game, s| {
                game.add_info_log_item(&format!(
                    "{} choose {}",
                    game.players[s.player_index].get_name(),
                    if s.choice == game.starting_player_index {
                        format!(
                            "{} to remain the staring player",
                            if s.choice == game.active_player() {
                                String::new()
                            } else {
                                game.players[s.choice].get_name()
                            }
                        )
                    } else {
                        format!(
                            "{} as the new starting player",
                            if s.choice == game.active_player() {
                                String::from("themselves")
                            } else {
                                game.players[s.choice].get_name()
                            }
                        )
                    }
                ));
                game.starting_player_index = s.choice;
            },
        )
        .build()
}

fn player_that_chooses_next_first_player(players: &[&Player]) -> usize {
    players
        .iter()
        .sorted_by_key(|p| -((p.resources.mood_tokens + p.resources.culture_tokens) as i32))
        .next()
        .expect("no player found")
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
        let players = vec![&player2, &player1, &player0];
        let got = super::player_that_chooses_next_first_player(&players);
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
