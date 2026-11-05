use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::advance::{Advance, do_advance, gain_advance_without_payment, remove_advance};
use crate::city::raze_city;
use crate::content::ability::Ability;
use crate::content::persistent_events::{
    AdvanceRequest, EventResponse, PaymentRequest, PersistentEventRequest, PersistentEventType,
    PlayerRequest, PositionRequest, TriggerPersistentEventParams,
    trigger_persistent_event_with_listener,
};
use crate::events::{EventOrigin, EventPlayer};
use crate::log::{TurnType, add_start_turn_action_if_needed, add_turn_log};
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash, Copy)]
pub enum StatusPhaseStateType {
    CompleteObjectives,
    FreeAdvance,
    DrawCards,
    RazeSize1City,
    ChangeGovernmentType,
    DetermineFirstPlayer,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum StatusPhaseState {
    CompleteObjectives,
    FreeAdvance,
    SecondFreeAdvance,
    DrawCards,
    RazeSize1City,
    ChangeGovernmentType(bool),
    DetermineFirstPlayer(usize),
}

impl Display for StatusPhaseStateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusPhaseStateType::CompleteObjectives => write!(f, "Complete Objectives"),
            StatusPhaseStateType::FreeAdvance => write!(f, "Free Advance"),
            StatusPhaseStateType::DrawCards => write!(f, "Draw Cards"),
            StatusPhaseStateType::RazeSize1City => write!(f, "Raze Size 1 City"),
            StatusPhaseStateType::ChangeGovernmentType => write!(f, "Change Government"),
            StatusPhaseStateType::DetermineFirstPlayer => write!(f, "Determine First Player"),
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
    play_status_phase(game, StatusPhaseState::CompleteObjectives, false);
}

pub(crate) fn status_phase_response(game: &mut Game, phase: StatusPhaseState) {
    play_status_phase(game, phase, true);
}

fn play_status_phase(game: &mut Game, mut phase: StatusPhaseState, response: bool) {
    use StatusPhaseState::*;

    loop {
        let t = status_phase_type(&phase);
        if game.events.is_empty() {
            add_turn_log(game, TurnType::StatusPhase(t));
            // needed for draw cards
            if response {
                add_start_turn_action_if_needed(game, game.active_player());
            }
            game.player_changed();
        }
        phase = match trigger_persistent_event_with_listener(
            game,
            &game.human_players_sorted(game.starting_player_index),
            |events| &mut events.status_phase,
            &game.cache.status_phase_handler(&t).listeners.clone(),
            phase,
            PersistentEventType::StatusPhase,
            TriggerPersistentEventParams::default(),
        ) {
            Some(s) => s,
            None => return,
        };

        phase = match phase {
            CompleteObjectives => {
                if game.variable_end.is_some()
                    || game.age >= game.options.length.ages()
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
            FreeAdvance if game.options.length == crate::game::GameLength::Shorter => {
                SecondFreeAdvance
            }
            FreeAdvance | SecondFreeAdvance => DrawCards,
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
            |game, c, phase| {
                gain_advance_without_payment(
                    game,
                    c.choice,
                    &c.player(),
                    ResourcePile::empty(),
                    *phase != StatusPhaseState::SecondFreeAdvance,
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
                        "{} changed their government from {} to {}",
                        p.get(game)
                            .government(game)
                            .expect("player should have a government before changing it"),
                        p.name(game),
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

pub(crate) fn player_that_chooses_next_first_player(players: &[&Player]) -> usize {
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

#[must_use]
pub fn status_phase_type(p: &StatusPhaseState) -> StatusPhaseStateType {
    match p {
        StatusPhaseState::CompleteObjectives => StatusPhaseStateType::CompleteObjectives,
        StatusPhaseState::FreeAdvance | StatusPhaseState::SecondFreeAdvance => {
            StatusPhaseStateType::FreeAdvance
        }
        StatusPhaseState::DrawCards => StatusPhaseStateType::DrawCards,
        StatusPhaseState::RazeSize1City => StatusPhaseStateType::RazeSize1City,
        StatusPhaseState::ChangeGovernmentType(_) => StatusPhaseStateType::ChangeGovernmentType,
        StatusPhaseState::DetermineFirstPlayer(_) => StatusPhaseStateType::DetermineFirstPlayer,
    }
}

#[cfg(test)]
mod tests {
    use super::{ChangeGovernment, change_government_type};
    use crate::advance::{Advance, do_advance, find_special_advance};
    use crate::city_pieces::Building;
    use crate::civilization::Civilization;
    use crate::events::{EventOrigin, EventPlayer};
    use crate::game::{Game, GameContext};
    use crate::game_setup::{GameSetupBuilder, setup_game};
    use crate::player::{CostTrigger, Player};
    use crate::resource_pile::ResourcePile;
    use crate::special_advance::SpecialAdvance;

    fn babylonia_before_dogma() -> (Game, EventPlayer) {
        let mut game = setup_game(
            &GameSetupBuilder::new(2)
                .seed("ziggurats-government-regression".into())
                .assigned_civilizations(vec!["Babylonia".into(), "Rome".into()])
                .build(),
        );
        let player = EventPlayer::new(0, EventOrigin::Ability("Change Government".into()));
        crate::log::add_start_turn_action_if_needed(&mut game, player.index);
        for advance in [
            Advance::Myths,
            Advance::StateReligion,
            Advance::Writing,
            Advance::Philosophy,
        ] {
            assert!(player.get(&game).can_advance_free(advance, &game));
            do_advance(&mut game, advance, &player, false);
        }
        player.get_mut(&mut game).resources.ideas = 5;
        assert!(player.get(&game).can_advance_free(Advance::Dogma, &game));
        (game, player)
    }

    fn assert_ziggurats(game: &Game, active: bool) {
        let player = game.player(0);
        assert_eq!(
            player.special_advances.contains(SpecialAdvance::Ziggurats),
            active
        );
        assert_eq!(player.resource_limit.ideas, 7);
        assert_eq!(
            player.resources.ideas, 5,
            "Dogma must not discard Babylonia's ideas"
        );
        assert_eq!(player.wasted_resources.ideas, 0);

        // Exercise the live building-cost listeners and payment conversion, not
        // just the special-advance flag. State Religion waives the Temple's food.
        let temple = player.building_cost(game, Building::Temple, CostTrigger::NoModifiers);
        assert!(
            temple
                .cost
                .is_valid_payment(&(ResourcePile::wood(1) + ResourcePile::ore(1)))
        );
        assert_eq!(
            temple.cost.is_valid_payment(&ResourcePile::ideas(2)),
            active
        );
        assert_eq!(
            temple
                .cost
                .is_valid_payment(&(ResourcePile::wood(1) + ResourcePile::ideas(1))),
            active
        );
        let fortress = player.building_cost(game, Building::Fortress, CostTrigger::NoModifiers);
        assert!(!fortress.cost.is_valid_payment(&ResourcePile::ideas(3)));
    }

    #[test]
    fn ziggurats_and_any_government_special_advance_mapping() {
        use SpecialAdvance::*;

        for (civilization, expected) in [
            ("Aztecs", [Some(TributeEmpire); 3]),
            ("China", [Some(ImperialArmy); 3]),
            ("Greece", [Some(CityStates); 3]),
            ("Huns", [Some(HunnicTribes); 3]),
            ("Japan", [Some(Shogunate); 3]),
            ("Rome", [Some(Provinces); 3]),
            ("Babylonia", [None, None, Some(Ziggurats)]),
        ] {
            let game = setup_game(
                &GameSetupBuilder::new(2)
                    .seed("government-special-mapping-regression".into())
                    .assigned_civilizations(vec![civilization.into(), "Rome".into()])
                    .build(),
            );
            for (advance, special) in [Advance::Voting, Advance::Nationalism, Advance::Dogma]
                .into_iter()
                .zip(expected)
            {
                assert_eq!(
                    find_special_advance(advance, &game, 0),
                    special,
                    "{civilization}: {advance:?}"
                );
            }
        }
    }

    #[test]
    fn ziggurats_activates_immediately_on_dogma() {
        let (mut game, player) = babylonia_before_dogma();
        assert_ziggurats(&game, false);
        do_advance(&mut game, Advance::Dogma, &player, false);
        assert_ziggurats(&game, true);

        // Loading a save reconstructs specials from prerequisites. It must not
        // be required to initialize the ability that the live purchase grants.
        let reloaded = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
        assert_eq!(
            game.player(0).special_advances,
            reloaded.player(0).special_advances
        );
        assert_ziggurats(&reloaded, true);
    }

    #[test]
    fn ziggurats_live_government_cycle() {
        let (mut game, player) = babylonia_before_dogma();
        do_advance(&mut game, Advance::Dogma, &player, false);

        // Deliberately reach removal without a reload (or an acquisition
        // assertion, covered above): this used to panic removing a listener
        // that the live Dogma purchase had never registered.
        change_government_type(
            &mut game,
            &player,
            &ChangeGovernment::new("Democracy".into(), vec![]),
        );
        assert!(player.get(&game).has_advance(Advance::Voting));
        assert!(!player.get(&game).has_advance(Advance::Dogma));
        assert_ziggurats(&game, false);

        change_government_type(
            &mut game,
            &player,
            &ChangeGovernment::new("Theocracy".into(), vec![]),
        );
        assert!(player.get(&game).has_advance(Advance::Dogma));
        assert!(!player.get(&game).has_advance(Advance::Voting));
        assert_ziggurats(&game, true);
    }

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
