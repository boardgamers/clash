use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::advance::{Advance, do_advance, gain_advance_without_payment, remove_advance};
use crate::consts::AGES;
use crate::content::builtin::Builtin;
use crate::content::persistent_events::{
    AdvanceRequest, EventResponse, PaymentRequest, PersistentEventRequest, PersistentEventType,
    PlayerRequest, PositionRequest,
};
use crate::objective_card::{
    gain_objective_card_from_pile, present_objective_cards, status_phase_completable,
};
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player_events::{PersistentEvent, PersistentEvents};
use crate::wonder::Wonder;
use crate::{game::Game, player::Player, resource_pile::ResourcePile, utils};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum StatusPhaseState {
    CompleteObjectives,
    FreeAdvance,
    DrawCards,
    RazeSize1City,
    ChangeGovernmentType(bool),
    DetermineFirstPlayer(usize),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ChangeGovernment {
    pub new_government: String,
    pub additional_advances: Vec<Advance>,
}

impl ChangeGovernment {
    #[must_use]
    pub fn new(government: String, additional_advances: Vec<Advance>) -> Self {
        Self {
            new_government: government,
            additional_advances,
        }
    }
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
            &game.cache.status_phase_handler(&phase).listeners.clone(),
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
            RazeSize1City => ChangeGovernmentType(false),
            ChangeGovernmentType(_) => DetermineFirstPlayer(player_that_chooses_next_first_player(
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
    Builtin::builder(
        "Complete Objectives",
        "Select Status Phase Objectives to Complete",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.status_phase,
        0,
        |game, player_index, _name, _s| {
            let player = game.player(player_index);
            let opportunities = player
                .objective_cards
                .iter()
                .flat_map(|o| status_phase_completable(game, player, *o))
                .collect_vec();
            present_objective_cards(game, player_index, opportunities);
        },
    )
    .build()
}

pub(crate) fn free_advance() -> Builtin {
    Builtin::builder("Free Advance", "Advance for free")
        .add_advance_request(
            |event| &mut event.status_phase,
            0,
            |game, player_index, _player_name, _| {
                let choices = game
                    .cache
                    .get_advances()
                    .iter()
                    .filter(|advance| {
                        game.player(player_index)
                            .can_advance_free(advance.advance, game)
                    })
                    .map(|a| a.advance)
                    .collect_vec();
                Some(AdvanceRequest::new(choices))
            },
            |game, c, _, _| {
                game.add_info_log_item(&format!(
                    "{} advanced {} for free",
                    c.player_name,
                    c.choice.name(game)
                ));
                gain_advance_without_payment(
                    game,
                    c.choice,
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
            |game, player_index, _player_name, _| {
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
            |game, s, _, _| {
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
        ChangeGovernmentOption::NotFreeAndOptional,
        |_, _, _| true,
        |v, b| {
            if let StatusPhaseState::ChangeGovernmentType(paid) = v {
                *paid = b;
            } else {
                panic!("Illegal state")
            }
        },
        |v| {
            if let StatusPhaseState::ChangeGovernmentType(paid) = v {
                *paid
            } else {
                panic!("Illegal state")
            }
        },
    )
    .build()
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum ChangeGovernmentOption {
    FreeAndMandatory,
    NotFreeAndOptional,
}

pub(crate) fn add_change_government<A, E, V>(
    a: A,
    event: E,
    option: ChangeGovernmentOption,
    is_active_player: impl Fn(&V, usize, &Game) -> bool + 'static + Clone + Sync + Send,
    set_paid: impl Fn(&mut V, bool) + 'static + Clone + Sync + Send,
    has_paid: impl Fn(&mut V) -> bool + 'static + Clone + Sync + Send,
) -> A
where
    E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
    A: AbilityInitializerSetup,
    V: Clone + PartialEq,
{
    let event2 = event.clone();
    let set_paid2 = set_paid.clone();
    let is_active_player2 = is_active_player.clone();
    a.add_payment_request_listener(
        event,
        1,
        move |game, player_index, v, _| {
            set_paid(v, false);

            if !is_active_player(v, player_index, game) {
                return None;
            }

            if option == ChangeGovernmentOption::FreeAndMandatory {
                return None;
            }

            if !can_change_government_for_free(game.player(player_index), game) {
                return None;
            }

            let o = change_government_cost(game, player_index);
            if !game.player(player_index).can_afford(&o) {
                return None;
            }

            Some(vec![PaymentRequest::mandatory(
                o,
                "Pay to change government",
            )])
        },
        move |game, s, v, _| {
            let name = &s.player_name;
            let cost = &s.choice[0];
            game.add_info_log_item(&format!("{name} paid {cost} to change the government"));
            set_paid2(v, true);
        },
    )
    .add_persistent_event_listener(
        event2,
        0,
        move |game, player_index, _, v, _| {
            if !is_active_player2(v, player_index, game) {
                return None;
            }
            has_paid(v).then_some(PersistentEventRequest::ChangeGovernment)
        },
        move |game, player_index, player_name, action, request, _, _| {
            if let PersistentEventRequest::ChangeGovernment = &request {
                if let EventResponse::ChangeGovernmentType(c) = action {
                    game.add_info_log_item(&format!(
                        "{player_name} changed their government from {} to {}",
                        game.players[game.active_player()]
                            .government(game)
                            .expect("player should have a government before changing it"),
                        c.new_government
                    ));
                    game.add_info_log_item(&format!(
                        "Additional advances: {}",
                        if c.additional_advances.is_empty() {
                            "none".to_string()
                        } else {
                            c.additional_advances
                                .iter()
                                .map(|a| a.name(game))
                                .join(", ")
                        }
                    ));
                    change_government_type(game, player_index, &c);
                    return;
                }
            }
            panic!("Illegal action")
        },
    )
}

fn change_government_cost(game: &mut Game, player_index: usize) -> PaymentOptions {
    PaymentOptions::resources(
        game.player(player_index),
        PaymentReason::ChangeGovernment,
        CHANGE_GOVERNMENT_COST,
    )
}

fn change_government_type(game: &mut Game, player_index: usize, new_government: &ChangeGovernment) {
    let government = &new_government.new_government;
    let a = game.cache.get_government(government);
    assert!(
        game.player(player_index)
            .can_advance_ignore_contradicting(a.advances[0].advance, game),
        "Cannot advance in change government"
    );

    let player_government_advances = government_advances(game.player(player_index), game);

    assert_eq!(
        player_government_advances.len() - 1,
        new_government.additional_advances.len(),
        "Illegal number of additional advances"
    );

    for a in player_government_advances {
        remove_advance(game, a, player_index);
    }

    do_advance(
        game,
        game.cache.get_government(government).advances[0].advance,
        player_index,
    );
    for name in &new_government.additional_advances {
        let (pos, advance) = game
            .cache
            .get_government(government)
            .advances
            .iter()
            .find_position(|a| a.advance == *name)
            .unwrap_or_else(|| {
                panic!(
                    "Advance with name {} not found in government advances",
                    name.name(game)
                );
            });
        assert!(
            pos > 0,
            "Additional advances should not include the leading government advance"
        );
        do_advance(game, advance.advance, player_index);
    }
}

pub(crate) fn government_advances(p: &Player, game: &Game) -> Vec<Advance> {
    let current = p.government(game).expect("player should have a government");

    game.cache
        .get_government(&current)
        .advances
        .iter()
        .filter(|a| p.has_advance(a.advance))
        .map(|a| a.advance)
        .collect_vec()
}

#[must_use]
pub(crate) fn can_change_government_for_free(player: &Player, game: &Game) -> bool {
    player.government(game).is_some_and(|government| {
        game.cache.get_governments().iter().any(|g| {
            g.government != Some(government.clone())
                && player.can_advance_ignore_contradicting(g.advances[0].advance, game)
        })
    })
}

pub(crate) fn determine_first_player() -> Builtin {
    Builtin::builder("Determine First Player", "Determine the first player")
        .add_player_request(
            |event| &mut event.status_phase,
            0,
            |game, player_index, phase, _| {
                if let StatusPhaseState::DetermineFirstPlayer(want) = phase {
                    (*want == player_index).then_some(PlayerRequest::new(
                        game.human_players(game.starting_player_index),
                        "Determine the first player",
                    ))
                } else {
                    panic!("Illegal state")
                }
            },
            |game, s, _, _| {
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
        .find(|p| p.wonders_owned.contains(Wonder::GreatLighthouse))
        .or_else(|| {
            players
                .iter()
                .sorted_by_key(|p| -((p.resources.mood_tokens + p.resources.culture_tokens) as i32))
                .next()
        })
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
        player0_mood: u8,
        player1_mood: u8,
        player2_mood: u8,
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
