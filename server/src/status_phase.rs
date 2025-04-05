use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::advance::{do_advance, gain_advance_without_payment, remove_advance};
use crate::consts::AGES;
use crate::content::builtin::{Builtin, status_phase_handler};
use crate::content::persistent_events::{
    AdvanceRequest, ChangeGovernmentRequest, EventResponse, PersistentEventRequest,
    PersistentEventType, PlayerRequest, PositionRequest,
};
use crate::objective_card::gain_objective_card_from_pile;
use crate::payment::PaymentOptions;
use crate::player_events::{PersistentEvent, PersistentEvents};
use crate::{content::advances, game::Game, player::Player, resource_pile::ResourcePile, utils};
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

#[must_use]
pub fn get_status_phase(game: &Game) -> Option<&StatusPhaseState> {
    game.events.iter().find_map(|e| match &e.event_type {
        PersistentEventType::StatusPhase(s) => Some(s),
        _ => None,
    })
}

pub(crate) fn enter_status_phase(game: &mut Game) {
    game.add_info_log_group(format!(
        "The game has entered the {} status phase",
        utils::ordinal_number(game.age)
    ));
    play_status_phase(game, StatusPhaseState::CompleteObjectives);
}

pub(crate) fn play_status_phase(game: &mut Game, mut phase: StatusPhaseState) {
    use StatusPhaseState::*;

    loop {
        phase = match game.trigger_persistent_event_with_listener(
            &game.human_players(game.starting_player_index),
            |events| &mut events.status_phase,
            &status_phase_handler(&phase).listeners,
            phase,
            PersistentEventType::StatusPhase,
            None,
            |_| {},
        ) {
            Some(s) => s,
            None => return,
        };

        phase = match phase {
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
                    .map(|p| game.player(p))
                    .collect_vec(),
            )),
            DetermineFirstPlayer(_) => {
                game.next_age();
                return;
            }
        };
    }
}

pub(crate) fn complete_objectives() -> Builtin {
    // todo not implemented
    Builtin::builder("Complete Objectives", "Complete objectives").build()
}

pub(crate) fn free_advance() -> Builtin {
    Builtin::builder("Free Advance", "Advance for free")
        .add_advance_request(
            |event| &mut event.status_phase,
            0,
            |game, player_index, _player_name| {
                let choices = advances::get_all()
                    .into_iter()
                    .filter(|advance| game.player(player_index).can_advance_free(advance))
                    .map(|a| a.name)
                    .collect_vec();
                Some(AdvanceRequest::new(choices))
            },
            |game, c, _| {
                game.add_info_log_item(&format!(
                    "{} advanced {} for free",
                    c.player_name, c.choice
                ));
                gain_advance_without_payment(
                    game,
                    &c.choice,
                    c.player_index,
                    ResourcePile::empty(),
                    true,
                );
            },
        )
        .build()
}

pub(crate) fn draw_cards() -> Builtin {
    Builtin::builder("Draw Cards", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.status_phase,
            0,
            |game, p, _name, _s| {
                gain_action_card_from_pile(game, p);
                gain_objective_card_from_pile(game, p);
            },
        )
        .build()
}

pub(crate) fn raze_city() -> Builtin {
    Builtin::builder("Raze city", "Raze size 1 city for 1 gold")
        .add_position_request(
            |event| &mut event.status_phase,
            0,
            |game, player_index, _player_name| {
                let player = game.player(player_index);
                let cities = player
                    .cities
                    .iter()
                    .filter(|city| city.size() == 1)
                    .map(|city| city.position)
                    .collect_vec();
                if cities.is_empty() {
                    return None;
                }
                let needed = 0..=1;
                Some(PositionRequest::new(
                    cities,
                    needed,
                    "May raze a size 1 city for 1 gold",
                ))
            },
            |game, s, _| {
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
        |event| &mut event.status_phase,
        true,
        CHANGE_GOVERNMENT_COST,
    )
    .build()
}

pub(crate) fn add_change_government<A, E, V>(
    a: A,
    event: E,
    optional: bool,
    cost: ResourcePile,
) -> A
where
    E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone,
    A: AbilityInitializerSetup,
    V: Clone + PartialEq,
{
    let cost2 = cost.clone();
    a.add_persistent_event_listener(
        event,
        0,
        move |game, player_index, _, _| {
            let p = game.player(player_index);
            (can_change_government_for_free(p)
                && p.can_afford(&PaymentOptions::resources(cost.clone())))
            .then_some(PersistentEventRequest::ChangeGovernment(
                ChangeGovernmentRequest::new(optional, cost.clone()),
            ))
        },
        move |game, player_index, player_name, action, request, _| {
            if let PersistentEventRequest::ChangeGovernment(r) = &request {
                if let EventResponse::ChangeGovernmentType(t) = action {
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
                                if c.additional_advances.is_empty() {
                                    "none".to_string()
                                } else {
                                    c.additional_advances.join(", ")
                                }
                            ));
                            game.players[player_index].lose_resources(cost2.clone());
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
            |event| &mut event.status_phase,
            0,
            |game, player_index, phase| {
                if let StatusPhaseState::DetermineFirstPlayer(want) = phase {
                    (*want == player_index).then_some(PlayerRequest::new(
                        game.human_players(game.starting_player_index),
                        "Determine the first player",
                    ))
                } else {
                    panic!("Illegal state")
                }
            },
            |game, s, _| {
                game.add_info_log_item(&format!(
                    "{} choose {}",
                    game.player_name(s.player_index),
                    if s.choice == game.starting_player_index {
                        format!(
                            "{} to remain the staring player",
                            if s.choice == game.active_player() {
                                String::new()
                            } else {
                                game.player_name(s.choice)
                            }
                        )
                    } else {
                        format!(
                            "{} as the new starting player",
                            if s.choice == game.active_player() {
                                String::from("themselves")
                            } else {
                                game.player_name(s.choice)
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
