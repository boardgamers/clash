use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::advance::{Advance, do_advance, gain_advance_without_payment, remove_advance};
use crate::city::raze_city;
use crate::consts::AGES;
use crate::content::ability::Ability;
use crate::content::persistent_events::{
    AdvanceRequest, EventResponse, PaymentRequest, PersistentEventRequest, PersistentEventType,
    PlayerRequest, PositionRequest, TriggerPersistentEventParams,
    trigger_persistent_event_with_listener,
};
use crate::events::{EventOrigin, EventPlayer};
use crate::log::{TurnType, add_turn_log};
use crate::objective_card::{
    gain_objective_card_from_pile, present_objective_cards, status_phase_completable,
};
use crate::payment::PaymentOptions;
use crate::player_events::{PersistentEvent, PersistentEvents};
use crate::wonder::Wonder;
use crate::{game::Game, player::Player, resource_pile::ResourcePile};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum StatusPhaseState {
    CompleteObjectives,
    FreeAdvance,
    DrawCards,
    RazeSize1City,
    ChangeGovernmentType(bool),
    DetermineFirstPlayer(usize),
}

impl Display for StatusPhaseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusPhaseState::CompleteObjectives => write!(f, "Complete Objectives"),
            StatusPhaseState::FreeAdvance => write!(f, "Free Advance"),
            StatusPhaseState::DrawCards => write!(f, "Draw Cards"),
            StatusPhaseState::RazeSize1City => write!(f, "Raze Size 1 City"),
            StatusPhaseState::ChangeGovernmentType(_) => write!(f, "Change Government"),
            StatusPhaseState::DetermineFirstPlayer(_) => write!(f, "Determine First Player"),
        }
    }
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
    add_turn_log(game, TurnType::StatusPhase);
    play_status_phase(game, StatusPhaseState::CompleteObjectives);
}

pub(crate) fn play_status_phase(game: &mut Game, mut phase: StatusPhaseState) {
    use StatusPhaseState::*;

    loop {
        phase = match trigger_persistent_event_with_listener(
            game,
            &game.human_players_sorted(game.starting_player_index),
            |events| &mut events.status_phase,
            &game.cache.status_phase_handler(&phase).listeners.clone(),
            phase,
            PersistentEventType::StatusPhase,
            TriggerPersistentEventParams::default(),
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
                    .human_players_sorted(game.starting_player_index)
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

pub(crate) fn complete_objectives() -> Ability {
    Ability::builder(
        "Complete Objectives",
        "Select Status Phase Objectives to Complete",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.status_phase,
        0,
        |game, p, _s| {
            let player = game.player(p.index);
            let opportunities = player
                .objective_cards
                .iter()
                .flat_map(|o| status_phase_completable(game, player, *o))
                .collect_vec();
            present_objective_cards(game, p.index, opportunities);
        },
    )
    .build()
}

pub(crate) fn free_advance() -> Ability {
    Ability::builder("Free Advance", "Advance for free")
        .add_advance_request(
            |event| &mut event.status_phase,
            0,
            |game, p, _| {
                let choices = game
                    .cache
                    .get_advances()
                    .values()
                    .filter(|advance| p.get(game).can_advance_free(advance.advance, game))
                    .map(|a| a.advance)
                    .collect_vec();
                Some(AdvanceRequest::new(choices))
            },
            |game, c, _| {
                gain_advance_without_payment(
                    game,
                    c.choice,
                    &c.player(),
                    ResourcePile::empty(),
                    true,
                );
            },
        )
        .build()
}

pub(crate) fn draw_cards() -> Ability {
    Ability::builder("Draw Cards", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.status_phase,
            0,
            |game, p, _s| {
                gain_action_card_from_pile(game, p);
                gain_objective_card_from_pile(game, p);
            },
        )
        .build()
}

pub(crate) fn use_raze_city() -> Ability {
    Ability::builder("Raze city", "Raze size 1 city for 1 gold")
        .add_position_request(
            |event| &mut event.status_phase,
            0,
            |game, p, _| {
                let player = p.get(game);
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
                    s.log(game, "Did not raze a city");
                    return;
                }
                let player = &s.player();
                raze_city(game, player, s.choice[0]);
                player.gain_resources(game, ResourcePile::gold(1));
            },
        )
        .build()
}

pub(crate) fn may_change_government() -> Ability {
    add_change_government(
        Ability::builder("Change Government", "Change your government"),
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
        move |game, p, v| {
            let player_index = p.index;
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

            let o = change_government_cost(game, player_index, p.origin.clone());
            if !game.player(player_index).can_afford(&o) {
                return None;
            }

            Some(vec![PaymentRequest::optional(
                o,
                "Pay to change government",
            )])
        },
        move |game, s, v| {
            let cost = &s.choice[0];
            if cost.is_empty() {
                s.log(game, "Keep current government");
            } else {
                set_paid2(v, true);
            }
        },
    )
    .add_persistent_event_listener(
        event2,
        0,
        move |game, p, v| {
            if !is_active_player2(v, p.index, game) {
                return None;
            }
            has_paid(v).then_some(PersistentEventRequest::ChangeGovernment)
        },
        move |game, p, action, request, _| {
            if let PersistentEventRequest::ChangeGovernment = &request
                && let EventResponse::ChangeGovernmentType(c) = action
            {
                p.log(
                    game,
                    &format!(
                        "{p} changed their government from {} to {}",
                        p.get(game)
                            .government(game)
                            .expect("player should have a government before changing it"),
                        c.new_government
                    ),
                );
                p.log(
                    game,
                    &format!(
                        "Additional advances: {}",
                        if c.additional_advances.is_empty() {
                            "none".to_string()
                        } else {
                            c.additional_advances
                                .iter()
                                .map(|a| a.name(game))
                                .join(", ")
                        }
                    ),
                );
                change_government_type(game, p, &c);
                return;
            }
            panic!("Illegal action")
        },
    )
}

fn change_government_cost(
    game: &mut Game,
    player_index: usize,
    origin: EventOrigin,
) -> PaymentOptions {
    PaymentOptions::resources(game.player(player_index), origin, CHANGE_GOVERNMENT_COST)
}

fn change_government_type(
    game: &mut Game,
    player: &EventPlayer,
    new_government: &ChangeGovernment,
) {
    let government = &new_government.new_government;
    let a = game.cache.get_government(government);
    assert!(
        player
            .get(game)
            .can_advance_ignore_contradicting(a.advances[0].advance, game),
        "Cannot advance in change government"
    );

    let player_government_advances = government_advances(player.get(game), game);

    assert_eq!(
        player_government_advances.len() - 1,
        new_government.additional_advances.len(),
        "Illegal number of additional advances"
    );

    for a in player_government_advances {
        remove_advance(game, a, player);
    }

    do_advance(
        game,
        game.cache.get_government(government).advances[0].advance,
        player,
        false,
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
        do_advance(game, advance.advance, player, false);
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

pub(crate) fn determine_first_player() -> Ability {
    Ability::builder("Determine First Player", "Determine the first player")
        .add_player_request(
            |event| &mut event.status_phase,
            0,
            |game, p, phase| {
                if let StatusPhaseState::DetermineFirstPlayer(want) = phase {
                    (*want == p.index).then_some(PlayerRequest::new(
                        game.human_players_sorted(game.starting_player_index),
                        "Determine the first player",
                    ))
                } else {
                    panic!("Illegal state")
                }
            },
            |game, s, _| {
                s.log(
                    game,
                    &format!(
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
                    ),
                );
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
    use crate::civilization::Civilization;

    use crate::player::Player;
    use crate::resource_pile::ResourcePile;

    fn assert_next_player(
        name: &str,
        player0_mood: u8,
        player1_mood: u8,
        player2_mood: u8,
        expected_player_index: usize,
    ) {
        let mut player0 = Player::new(get_test_civilization(), 0);
        player0.resources += ResourcePile::mood_tokens(player0_mood);
        let mut player1 = Player::new(get_test_civilization(), 1);
        player1.resources += ResourcePile::mood_tokens(player1_mood);
        let mut player2 = Player::new(get_test_civilization(), 2);
        player2.resources += ResourcePile::mood_tokens(player2_mood);
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

    #[must_use]
    pub fn get_test_civilization() -> Civilization {
        Civilization::new("test", vec![], vec![], None)
    }
}
