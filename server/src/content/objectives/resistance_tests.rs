use crate::action::Action;
use crate::cache::Cache;
use crate::card::HandCard;
use crate::city::City;
use crate::combat_listeners::{CombatResult, on_end_combat};
use crate::combat_stats::{Battleground, CombatPlayerStats, CombatStats};
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use crate::game::{Game, GameContext};
use crate::game_api;
use crate::game_data::GameData;
use crate::leader::Leader;
use crate::log::{ActionLogAction, current_turn_log_mut, current_turn_log_without_redo};
use crate::movement::{MoveUnits, MovementAction};
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::{Unit, UnitType, Units};

// Printed Defiance is internally Resistance, not the Defiance/Against All Odds objective.
const CARDS: [u8; 2] = [27, 28];

fn test_game(owner: usize, cards: &[u8], barbarians: bool) -> Game {
    let mut data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/test_games/tactics_cards/heavy_resistance.json"
    ))
    .unwrap();
    for p in data["players"].as_array_mut().unwrap() {
        p["objective_cards"] = serde_json::json!([]);
        p["action_cards"] = serde_json::json!([]);
        p["advances"] = serde_json::json!(["Farming", "Mining", "Tactics"]);
    }
    data["players"][owner]["objective_cards"] = serde_json::json!(cards);
    if barbarians {
        data["players"][1 - owner]["civilization"] = serde_json::json!("Barbarians");
    }
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

fn infantry(amount: u8) -> Units {
    Units {
        infantry: amount,
        ..Units::empty()
    }
}

fn combat_stats(
    defending: bool,
    result: CombatResult,
    losses: Units,
    battleground: Battleground,
) -> CombatStats {
    let mut stats = CombatStats::new(
        battleground,
        false,
        CombatPlayerStats::new(
            usize::from(defending),
            infantry(3),
            Position::from_offset("C2"),
        ),
        CombatPlayerStats::new(
            usize::from(!defending),
            infantry(2),
            Position::from_offset("C1"),
        ),
        Some(result),
        None,
    );
    let opponent = if defending {
        &mut stats.attacker
    } else {
        &mut stats.defender
    };
    opponent.present = losses.clone();
    opponent.present.infantry += 1;
    opponent.losses = losses;
    stats
}

fn finish_combat(mut game: Game, stats: CombatStats) -> Game {
    game.log_index = 1;
    current_turn_log_mut(&mut game)
        .actions
        .push(ActionLogAction::new(
            Action::Playing(PlayingAction::EndTurn),
            0,
            None,
            0,
        ));
    on_end_combat(&mut game, stats);
    // Barbarian rewards resolve before combat objectives are checked.
    while let Some(handler) = game.current_event_handler() {
        let PersistentEventRequest::ResourceReward(request) = &handler.request else {
            break;
        };
        let response = Action::Response(EventResponse::ResourceReward(
            request.reward.payment_options.default.clone(),
        ));
        let player = game.active_player();
        game = game_api::execute(game, response, player);
    }
    game
}

fn has_defiance_prompt(game: &Game) -> bool {
    game.events.iter().any(|event| {
        matches!(&event.event_type, PersistentEventType::SelectObjectives(info)
            if info.shown_objective.as_deref() == Some("Resistance"))
    })
}

fn qualifies(card: u8, stats: CombatStats, barbarians: bool) -> bool {
    let game = finish_combat(test_game(0, &[card], barbarians), stats);
    has_defiance_prompt(&game)
}

#[test]
fn defiance_accepts_defender_wins_losses_and_draws_against_humans_and_barbarians() {
    for card in CARDS {
        for barbarians in [false, true] {
            for result in [
                CombatResult::DefenderWins,
                CombatResult::AttackerWins,
                CombatResult::Draw,
            ] {
                for battleground in [
                    Battleground::Land,
                    Battleground::City,
                    Battleground::CityWithFortress,
                ] {
                    assert!(
                        qualifies(
                            card,
                            combat_stats(true, result.clone(), infantry(2), battleground),
                            barbarians,
                        ),
                        "card {card}, barbarians {barbarians}, {result:?}, {battleground:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn defiance_never_qualifies_as_attacker() {
    for card in CARDS {
        for barbarians in [false, true] {
            for result in [
                CombatResult::DefenderWins,
                CombatResult::AttackerWins,
                CombatResult::Draw,
            ] {
                assert!(!qualifies(
                    card,
                    combat_stats(false, result, infantry(3), Battleground::Land),
                    barbarians,
                ));
            }
        }
    }
}

#[test]
fn defiance_requires_two_army_casualties_not_just_two_units() {
    let insufficient_losses = [
        Units::empty(),
        infantry(1),
        Units {
            settlers: 2,
            ..Units::empty()
        },
        Units {
            ships: 2,
            ..Units::empty()
        },
        Units {
            infantry: 1,
            settlers: 1,
            ..Units::empty()
        },
        Units {
            infantry: 1,
            ships: 1,
            ..Units::empty()
        },
        Units {
            settlers: 2,
            ships: 2,
            leader: Some(Leader::Alexander),
            ..Units::empty()
        },
    ];
    for card in CARDS {
        for barbarians in [false, true] {
            for losses in &insufficient_losses {
                for battleground in [Battleground::Land, Battleground::Sea] {
                    assert!(!qualifies(
                        card,
                        combat_stats(
                            true,
                            CombatResult::DefenderWins,
                            losses.clone(),
                            battleground,
                        ),
                        barbarians,
                    ));
                }
            }
        }
    }
}

#[test]
fn defiance_counts_all_army_types_including_leaders_and_ignores_extra_nonarmy_losses() {
    for card in CARDS {
        for losses in [
            Units {
                cavalry: 1,
                elephants: 1,
                ..Units::empty()
            },
            Units {
                infantry: 1,
                leader: Some(Leader::Alexander),
                ..Units::empty()
            },
            Units {
                infantry: 2,
                settlers: 1,
                ships: 1,
                ..Units::empty()
            },
        ] {
            // The card has no land-battle restriction: count lost army units, not
            // fighter_losses(), which counts only ships in a sea battle.
            for battleground in [Battleground::Land, Battleground::Sea] {
                assert!(qualifies(
                    card,
                    combat_stats(true, CombatResult::Draw, losses.clone(), battleground),
                    false,
                ));
            }
        }
    }
}

#[test]
fn defiance_does_not_count_own_losses_or_nonbattle_events() {
    for card in CARDS {
        let mut stats = combat_stats(
            true,
            CombatResult::AttackerWins,
            infantry(1),
            Battleground::Land,
        );
        stats.defender.losses = infantry(2);
        assert!(!qualifies(card, stats, false));

        let mut stats = combat_stats(
            true,
            CombatResult::AttackerWins,
            infantry(2),
            Battleground::Land,
        );
        stats.defender.present = Units::empty();
        assert!(!stats.is_battle());
        assert!(!qualifies(card, stats, false));
    }
}

#[test]
fn defiance_description_and_both_card_mappings_match_the_printed_rule() {
    let game = test_game(0, &CARDS, false);
    for card in CARDS {
        let objective = game
            .cache
            .get_objective_card(card)
            .objectives
            .iter()
            .find(|o| o.name == "Resistance")
            .expect("both printed Defiance cards must use Resistance");
        assert_eq!(
            objective.description,
            "Immediately after a battle where you were attacked and eliminated at least 2 army units (including barbarian army units)."
        );
        assert!(!objective.requires_player_combat);
        assert!(objective.status_phase_check.is_none());
    }
}

#[test]
fn defiance_can_be_claimed_after_a_real_defender_loss_or_draw_and_save_reload() {
    for card in CARDS {
        for result in [CombatResult::AttackerWins, CombatResult::Draw] {
            let mut game = test_game(1, &CARDS, false);
            let attacker_count = if result == CombatResult::AttackerWins {
                3
            } else {
                2
            };
            game.players[0].units = (0..attacker_count)
                .map(|id| Unit::new(0, Position::from_offset("C2"), UnitType::Infantry, id))
                .collect();
            game.players[0].next_unit_id = attacker_count;
            // A lost city offers a replacement settler before the objective prompt.
            for position in ["B1", "B3"] {
                game.players[1]
                    .cities
                    .push(City::new(1, Position::from_offset(position)));
            }
            // Two defending infantry eliminate two attackers, whether or not
            // an attacker survives to win. The defenders both die in either case.
            game.dice_roll_outcomes = vec![11; attacker_count as usize + 2];
            assert!(game.players[1].objective_opportunities.is_empty());
            let mut game = game_api::execute(
                game,
                Action::Movement(MovementAction::Move(MoveUnits::new(
                    (0..attacker_count).collect(),
                    Position::from_offset("C1"),
                    None,
                    ResourcePile::empty(),
                ))),
                0,
            );
            let turn_log = current_turn_log_without_redo(&game);
            let stats = turn_log
                .actions
                .iter()
                .find_map(|action| action.combat_stats.as_ref())
                .expect("the actual battle must record its result and casualties");
            assert_eq!(stats.result, Some(result.clone()));
            assert_eq!(stats.attacker.losses, infantry(2));
            assert_eq!(stats.defender.losses, infantry(2));

            if result == CombatResult::AttackerWins {
                assert!(matches!(
                    game.current_event_handler().map(|h| &h.request),
                    Some(PersistentEventRequest::SelectPositions(_))
                ));
                assert!(!has_defiance_prompt(&game));
                assert!(game.players[1].objective_opportunities.is_empty());
                game = game_api::execute(
                    game.clone(),
                    Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                        "B1",
                    )])),
                    1,
                );
            }

            assert!(has_defiance_prompt(&game));
            assert_eq!(game.active_player(), 1);
            let PersistentEventRequest::SelectHandCards(request) =
                &game.current_event_handler().unwrap().request
            else {
                panic!("expected a Defiance objective card choice");
            };
            assert_eq!(
                request.choices,
                vec![HandCard::ObjectiveCard(27), HandCard::ObjectiveCard(28)]
            );
            assert!(game.players[1].completed_objectives.is_empty());

            let game = game_api::execute(
                game.clone(), // persistence must preserve the pending objective choice
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(card),
                ])),
                1,
            );
            let completed = &game.players[1].completed_objectives;
            assert_eq!(completed.len(), 1);
            assert_eq!(completed[0].card, card);
            assert_eq!(completed[0].name, "Resistance");
            assert_eq!(
                game.players[1].objective_cards,
                CARDS
                    .into_iter()
                    .filter(|&id| id != card)
                    .collect::<Vec<_>>(),
                "one battle only completes one copy of Defiance"
            );
            assert!(game.events.is_empty());
        }
    }
}

#[test]
fn postcombat_queue_survives_settler_objective_reload_and_claim_or_skip() {
    use crate::{action_queue, board_history};
    for result in [
        CombatResult::AttackerWins,
        CombatResult::Draw,
        CombatResult::DefenderWins,
    ] {
        for claim in [false, true] {
            let mut game = test_game(1, &CARDS, false);
            let attackers = if result == CombatResult::AttackerWins {
                3
            } else {
                2
            };
            game.players[0].units = (0..attackers)
                .map(|id| Unit::new(0, Position::from_offset("C2"), UnitType::Infantry, id))
                .collect();
            game.players[0].units.push(Unit::new(
                0,
                Position::from_offset("C2"),
                UnitType::Settler,
                30,
            ));
            game.players[0].next_unit_id = 31;
            if result == CombatResult::DefenderWins {
                game.players[1].units.push(Unit::new(
                    1,
                    Position::from_offset("C1"),
                    UnitType::Infantry,
                    20,
                ));
            }
            for position in ["B1", "B3"] {
                game.players[1]
                    .cities
                    .push(City::new(1, Position::from_offset(position)));
            }
            game.dice_roll_outcomes = vec![11; 8];
            let mut game = board_history::execute(
                game,
                Action::Movement(MovementAction::Move(MoveUnits::new(
                    (0..attackers).collect(),
                    Position::from_offset("C1"),
                    None,
                    ResourcePile::empty(),
                ))),
                0,
            )
            .unwrap();
            let (key, _) = action_queue::offer(&game, 0).expect("postcombat off-turn choice");
            let actions = vec![Action::Movement(MovementAction::Stop)];
            let before_log = crate::game_api::log_length(&game);
            game = board_history::execute(
                game,
                Action::QueueMoves {
                    key: key.clone(),
                    actions: actions.clone(),
                    waiting: true,
                },
                0,
            )
            .unwrap();
            assert_eq!(crate::game_api::log_length(&game), before_log);
            if result == CombatResult::AttackerWins {
                game = board_history::execute(
                    game,
                    Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                        "B1",
                    )])),
                    1,
                )
                .unwrap();
            }
            assert!(has_defiance_prompt(&game));
            let mut fresh = game.clone();
            fresh.action_plan = None;
            let stripped = game_api::strip_secret(fresh.clone(), Some(0));
            assert_eq!(
                action_queue::offer(&fresh, 0),
                action_queue::offer(&stripped, 0)
            );
            assert!(action_queue::project(&stripped, 0, &actions).is_ok());
            assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
            assert_eq!(
                action_queue::offer(&game, 0).unwrap().1,
                "Choosing an objective"
            );
            assert!(action_queue::offer(&game, 1).is_none());
            assert!(action_queue::save(&mut game, 0, "stale", vec![], false).is_err());
            assert!(action_queue::save(&mut game, 1, &key, vec![], false).is_err());
            let data =
                serde_json::from_str(&serde_json::to_string(&game.cloned_data()).unwrap()).unwrap();
            game = Game::from_data(data, game.cache.clone(), GameContext::Play);
            assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
            assert!(game.action_plan.as_ref().unwrap().waiting);
            assert_eq!(game.action_plan.as_ref().unwrap().actions, actions);
            for viewer in [None, Some(1)] {
                assert!(
                    game_api::strip_secret(game.clone(), viewer)
                        .action_plan
                        .is_none()
                );
            }
            assert!(
                matches!(&game.state, crate::game::GameState::Movement(m) if m.movement_actions_left == 2)
            );
            let projected = action_queue::project(&game, 0, &[]).unwrap();
            assert_eq!(projected.state, game.state);
            assert!(game.players[1].completed_objectives.is_empty());
            for variant in 0..4 {
                let mut blocked = game.clone();
                match variant {
                    0 => blocked.round = 4,
                    1 => {
                        if let PersistentEventType::SelectObjectives(i) =
                            &mut blocked.events[0].event_type
                        {
                            i.combat_turn_owner = None;
                        }
                    }
                    2 => blocked.events.push(blocked.events[0].clone()),
                    _ => blocked.events[0].player.index = 0,
                }
                assert!(action_queue::offer(&blocked, 0).is_none());
            }
            game = board_history::execute(
                game,
                Action::Response(EventResponse::SelectHandCards(if claim {
                    vec![HandCard::ObjectiveCard(27)]
                } else {
                    vec![]
                })),
                1,
            )
            .unwrap();
            assert!(game.events.is_empty());
            assert!(game.action_plan.is_none());
            assert_eq!(game.state, crate::game::GameState::Playing);
            assert_eq!(game.current_player_index, 0);
            assert_eq!(
                game.players[1].completed_objectives.len(),
                usize::from(claim)
            );
        }
    }
}
