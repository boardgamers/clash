use crate::common::{TestAction, payment_response};
use common::JsonTest;
use server::action::Action;
use server::advance;
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::status_phase::ChangeGovernment;

mod common;

const JSON: JsonTest = JsonTest::new("status_phase");

#[test]
fn test_end_game() {
    JSON.test(
        "end_game",
        vec![TestAction::not_undoable(
            0,
            Action::Playing(PlayingAction::EndTurn),
        )],
    );
}

#[test]
fn test_free_advance() {
    JSON.test(
        "free_advance",
        vec![
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectAdvance(advance::Advance::Storage)),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectAdvance(advance::Advance::Philosophy)),
            ),
        ],
    );
}

#[test]
fn test_wrong_status_phase_action() {
    JSON.test(
        "illegal_free_advance",
        vec![TestAction::illegal(
            0,
            Action::Response(EventResponse::SelectPositions(vec![])),
        )],
    );
}

#[test]
fn test_raze_city() {
    JSON.test(
        "raze_city",
        vec![TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "A1",
            )])),
        )],
    );
}

#[test]
fn test_raze_city_decline() {
    JSON.test(
        "raze_city_decline",
        vec![
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectPositions(vec![]))),
            TestAction::not_undoable(1, Action::Response(EventResponse::SelectPositions(vec![]))),
        ],
    );
}

#[test]
fn test_determine_first_player() {
    JSON.test(
        "determine_first_player",
        vec![TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectPlayer(1)),
        )],
    );
}

#[test]
fn test_change_government() {
    JSON.test(
        "change_government",
        vec![
            TestAction::not_undoable(1, Action::Response(EventResponse::SelectPositions(vec![])))
                .skip_json(),
            TestAction::undoable(
                0,
                payment_response(ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1)),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::ChangeGovernmentType(ChangeGovernment::new(
                    String::from("Theocracy"),
                    vec![advance::Advance::Devotion],
                ))),
            ),
        ],
    );
}

#[test]
fn test_keep_government() {
    JSON.test(
        "keep_government",
        vec![
            TestAction::not_undoable(1, Action::Response(EventResponse::SelectPositions(vec![])))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::empty())),
        ],
    );
}

#[test]
fn epic_continues_after_six_ages_and_ends_after_ten() {
    use server::game::{GameLength, GameState};
    for (age, finished) in [(6, false), (9, false), (10, true)] {
        let mut game = JSON.load_game("end_game");
        game.options.length = GameLength::Epic;
        game.age = age;
        let game = server::game_api::execute(game, Action::Playing(PlayingAction::EndTurn), 0);
        assert_eq!(
            matches!(game.state, GameState::Finished),
            finished,
            "age {age}"
        );
        if !finished {
            assert!(game.events.iter().any(|event| matches!(
                event.event_type,
                server::content::persistent_events::PersistentEventType::StatusPhase(_)
            )));
        }
    }
}

#[test]
fn length_defaults_to_standard_and_survives_save_load() {
    use server::game::{GameLength, GameOptions};
    let options: GameOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(options.length.ages(), 6);
    assert_eq!(
        serde_json::to_value(&options).unwrap(),
        serde_json::json!({})
    );
    let options: GameOptions = serde_json::from_str(r#"{"length":"Epic"}"#).unwrap();
    let game = server::game_api::init(2, "epic-save".to_owned(), options);
    let data = common::to_json(&game);
    let value: serde_json::Value = serde_json::from_str(&data).unwrap();
    assert_eq!(value["options"]["length"], "Epic");
    let data: server::game_data::GameData = serde_json::from_str(&data).unwrap();
    let cache = server::cache::Cache::new(&data.options);
    let restored = server::game::Game::from_data(data, cache, server::game::GameContext::Play);
    assert!(restored.options.length == GameLength::Epic);
    assert_eq!(restored.options.length.ages(), 10);
}

#[test]
fn epic_preserves_early_ending_when_a_player_has_no_cities() {
    let mut game = JSON.load_game("end_game");
    game.options.length = server::game::GameLength::Epic;
    game.players[1].cities.clear();
    let game = server::game_api::execute(game, Action::Playing(PlayingAction::EndTurn), 0);
    assert!(server::game_api::ended(&game));
}
