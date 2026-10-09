use crate::common::{JsonTest, TestAction, custom_action, move_action, payment_response};
use server::action::Action;
use server::city_pieces::Building;
use server::content::custom_actions::{CustomActionType, PlayingActionModifier, SpecialAction};
use server::content::persistent_events::{EventResponse, SelectedStructure};
use server::cultural_influence::{InfluenceCultureAttempt, affordable_start_city};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::recruit::Recruit;
use server::resource_pile::ResourcePile;
use server::structure::Structure;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "greece");

#[test]
fn sparta_draft() {
    JSON.test(
        "sparta_draft",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Recruit(Recruit::new(
                &Units::new(0, 1, 0, 0, 0, None),
                Position::from_offset("A1"),
                ResourcePile::culture_tokens(1),
            ))),
        )],
    );
}

#[test]
fn sparta_battle() {
    JSON.test(
        "sparta_battle",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn sparta_uses_unit_counts_in_both_combat_roles() {
    use server::cache::Cache;
    use server::content::persistent_events::PersistentEventType;
    use server::game::{Game, GameContext};
    use server::game_data::GameData;

    for greek_player in [0, 1] {
        for (greek_count, opponent_count) in [(1, 2), (2, 2), (4, 2), (4, 4)] {
            for greek_ids_first in [false, true] {
                let mut data: serde_json::Value = serde_json::from_str(include_str!(
                    "test_games/civilizations/greece/sparta_battle.json"
                ))
                .unwrap();
                for player in 0..2 {
                    let greek = player == greek_player;
                    let count = if greek { greek_count } else { opponent_count };
                    let first_id = if greek == greek_ids_first { 0 } else { 100 };
                    data["players"][player]["advances"] = if greek {
                        serde_json::json!(["Farming", "Mining", "Tactics", "Draft"])
                    } else {
                        serde_json::json!(["Farming", "Mining", "Tactics"])
                    };
                    data["players"][player]["action_cards"] = serde_json::json!([18]);
                    data["players"][player]["units"] = serde_json::json!(
                        (0..count)
                            .map(|i| serde_json::json!({
                                "id": first_id + i,
                                "position": if player == 0 { "C2" } else { "C1" },
                                "unit_type": "Infantry"
                            }))
                            .collect::<Vec<_>>()
                    );
                    data["players"][player]["next_unit_id"] = serde_json::json!(first_id + count);
                }
                let data: GameData = serde_json::from_value(data).unwrap();
                let cache = Cache::new(&data.options);
                let game = Game::from_data(data, cache, GameContext::Play);
                let attackers = game.player(0).units.iter().map(|u| u.id).collect();
                let game = server::game_api::execute(
                    game,
                    move_action(attackers, Position::from_offset("C1")),
                    0,
                );
                let PersistentEventType::CombatRoundStart(round) = &game.current_event().event_type
                else {
                    panic!("Expected battle card selection");
                };
                let opponent_strength = if greek_player == 0 {
                    &round.defender_strength
                } else {
                    &round.attacker_strength
                };
                assert_eq!(
                    opponent_strength.deny_tactics_card,
                    greek_count < opponent_count,
                    "Greek player {greek_player}, {greek_count} vs {opponent_count}, Greek IDs first: {greek_ids_first}"
                );
            }
        }
    }
}

#[test]
fn hellenistic_culture_staring_point() {
    let game = &JSON.load_game("hellenistic_culture");

    let action_type = &PlayingActionType::Special(SpecialAction::Modifier(
        PlayingActionModifier::HellenisticInfluenceCultureAttempt,
    ));
    assert_eq!(
        affordable_start_city(
            game,
            0,
            game.get_any_city(Position::from_offset("D1")),
            action_type,
            false,
        )
        .unwrap(),
        (Position::from_offset("C2"), 0)
    );
    assert_eq!(
        affordable_start_city(
            game,
            0,
            game.get_any_city(Position::from_offset("C2")),
            action_type,
            false,
        )
        .unwrap(),
        (Position::from_offset("C2"), 0)
    );
}

#[test]
fn hellenistic_culture_cost() {
    JSON.test(
        "hellenistic_culture",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::InfluenceCultureAttempt(
                    InfluenceCultureAttempt::new(
                        SelectedStructure::new(
                            Position::from_offset("C2"),
                            Structure::Building(Building::Port),
                        ),
                        PlayingActionType::Special(SpecialAction::Modifier(
                            PlayingActionModifier::HellenisticInfluenceCultureAttempt,
                        )),
                    ),
                )),
            )
            .skip_json(),
            TestAction::not_undoable(0, payment_response(ResourcePile::mood_tokens(2))),
        ],
    )
}

#[test]
fn city_states() {
    JSON.test(
        "city_states",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Recruit(Recruit::new(
                    &Units::new(0, 1, 0, 0, 0, None),
                    Position::from_offset("A1"),
                    ResourcePile::culture_tokens(1),
                ))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B3",
                )])),
            ),
        ],
    )
}

#[test]
fn idol() {
    JSON.test(
        "idol",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::Idol)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::not_undoable(0, payment_response(ResourcePile::culture_tokens(1))),
        ],
    )
}

#[test]
fn ruler_of_the_world() {
    JSON.test(
        "ruler_of_the_world",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("D8")),
        )],
    );
}

#[test]
fn master() {
    JSON.test(
        "master",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::Master)).skip_json(),
            TestAction::not_undoable(0, payment_response(ResourcePile::mood_tokens(1))),
        ],
    );
}
