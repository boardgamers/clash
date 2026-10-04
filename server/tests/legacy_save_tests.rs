//! Frozen fixtures from 5fdd60e3 (engine 0.4.16), outside the regenerated game
//! snapshots: engine upgrades must keep reading games already stored on BGS.
use serde_json::{Value, json};
use server::action::{Action, try_execute_action};
use server::cache::Cache;
use server::content::custom_actions::{CustomActionType, PlayingActionModifier, SpecialAction};
use server::game::{Game, GameContext};
use server::game_data::GameData;
use server::log::{ActionLogTurn, TurnType};
use server::playing_actions::PlayingActionType;

fn load(raw: &str) -> Game {
    let data: GameData = serde_json::from_str(raw).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

#[test]
fn previous_release_saves_load_without_changing_the_position() {
    for raw in [
        include_str!("fixtures/legacy_0_4_16/setup.json"),
        include_str!("fixtures/legacy_0_4_16/free_economy.json"),
        include_str!("fixtures/legacy_0_4_16/incident.json"),
    ] {
        let before: Value = serde_json::from_str(raw).unwrap();
        let data: GameData = serde_json::from_str(raw).unwrap();
        let saved = serde_json::to_value(&data).unwrap();
        for key in [
            "map",
            "state",
            "current_player_index",
            "actions_left",
            "age",
            "round",
            "log_index",
            "undo_limit",
        ] {
            assert_eq!(before[key], saved[key], "{key}");
        }
        for (before, after) in before["players"]
            .as_array()
            .unwrap()
            .iter()
            .zip(saved["players"].as_array().unwrap())
        {
            for key in [
                "cities",
                "units",
                "resources",
                "advances",
                "action_cards",
                "objective_cards",
                "incident_tokens",
            ] {
                assert_eq!(before[key], after[key], "player field {key}");
            }
        }
        // The new format can be read again without repeatedly migrating it.
        let reloaded: GameData = serde_json::from_value(saved.clone()).unwrap();
        assert_eq!(saved, serde_json::to_value(reloaded).unwrap());
        let game = load(raw);
        for seat in [None, Some(0), Some(1)] {
            let visible = server::game_api::strip_secret(game.clone(), seat);
            // The rules fixtures omit NPC players; only setup is a full game.
            if game.players.iter().any(|p| p.civilization.is_pirates()) {
                server::web_view::view(&visible, seat);
            }
        }
    }
}

#[test]
fn legacy_setup_retains_journal_and_does_not_invent_unit_positions() {
    let original: Value =
        serde_json::from_str(include_str!("fixtures/legacy_0_4_16/setup.json")).unwrap();
    let data: GameData = serde_json::from_value(original.clone()).unwrap();
    let saved = serde_json::to_value(data).unwrap();
    let old = &original["log"][0]["rounds"][0]["turns"][0]["actions"][0];
    let new = &saved["log"][0]["rounds"][0]["turns"][0]["actions"][0];
    assert_eq!(old["log"], new["log"]);
    assert_eq!(new["action"], "StartTurn");
    let unit_entries: Vec<_> = new["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i.get("Units"))
        .collect();
    assert!(!unit_entries.is_empty());
    assert!(unit_entries.iter().all(|i| i.get("position").is_none()));
}

#[test]
fn legacy_turns_infer_action_owner_and_accept_status_phase() {
    let turn: ActionLogTurn = serde_json::from_value(json!({
        "turn_type": {"Player": 1},
        "actions": [
            {"action": {"Playing": "EndTurn"}},
            {"action": {"Response": {"Bool": false}}, "items": [
                {"player": 2, "Text": "Decline", "origin": {"Ability": "Test"}}
            ]}
        ]
    }))
    .unwrap();
    assert_eq!(turn.actions[0].player, 1);
    assert_eq!(turn.actions[1].player, 2);
    let status: ActionLogTurn =
        serde_json::from_value(json!({"turn_type": "StatusPhase"})).unwrap();
    assert!(matches!(status.turn_type, TurnType::StatusPhase(_)));
    assert!(status.actions.is_empty());
}

#[test]
fn old_special_actions_keep_their_once_per_turn_identity() {
    for (old, expected) in [
        (
            "FreeEconomyCollect",
            SpecialAction::Modifier(PlayingActionModifier::FreeEconomyCollect),
        ),
        (
            "VotingIncreaseHappiness",
            SpecialAction::Modifier(PlayingActionModifier::VotingIncreaseHappiness),
        ),
        (
            "Bartering",
            SpecialAction::Custom(CustomActionType::Bartering),
        ),
        (
            "AbsolutePower",
            SpecialAction::Custom(CustomActionType::AbsolutePower),
        ),
    ] {
        assert_eq!(
            serde_json::from_value::<SpecialAction>(json!(old)).unwrap(),
            expected
        );
        assert_eq!(
            serde_json::from_value::<SpecialAction>(serde_json::to_value(expected).unwrap())
                .unwrap(),
            expected
        );
        assert_eq!(
            serde_json::from_value::<PlayingActionType>(json!({"Custom": old})).unwrap(),
            PlayingActionType::Special(expected)
        );
    }
}

#[test]
fn legacy_recruitment_undo_and_redo_preserve_units_resources_and_history() {
    let before = load(include_str!("test_games/base/recruit_leader.json"));
    let action: Action = serde_json::from_value(json!({"Playing": {"Recruit": {
        "units": {"leader": "Augustus"}, "city_position": "A1",
        "payment": {"mood_tokens": 1, "culture_tokens": 1}
    }}}))
    .unwrap();
    let recruited = try_execute_action(before.clone(), action, 0).unwrap();
    let mut saved = serde_json::to_value(recruited.cloned_data()).unwrap();
    // Simulate the previous engine's log while retaining the real undo patch.
    let last_turn = saved["log"].as_array_mut().unwrap().last_mut().unwrap()["rounds"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["turns"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    let last = last_turn["actions"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    last.as_object_mut().unwrap().remove("player");
    last["log"] = json!(["Player1: Recruit: Gain Augustus at A1"]);
    for item in last["items"].as_array_mut().unwrap() {
        if let Some(units) = item.get_mut("Units") {
            units.as_object_mut().unwrap().remove("position");
        }
    }
    let migrated = load(&saved.to_string());
    assert!(migrated.can_undo());
    let undone = try_execute_action(migrated, Action::Undo, 0).unwrap();
    assert!(undone.can_redo());
    assert_eq!(undone.players[0].resources, before.players[0].resources);
    assert_eq!(
        serde_json::to_value(undone.cloned_data()).unwrap()["players"][0]["units"],
        serde_json::to_value(before.cloned_data()).unwrap()["players"][0]["units"]
    );
    assert!(
        !serde_json::to_string(&undone.cloned_data())
            .unwrap()
            .contains("Player1: Recruit: Gain Augustus")
    );
    let redone = try_execute_action(undone, Action::Redo, 0).unwrap();
    assert_eq!(redone.players[0].resources, recruited.players[0].resources);
    assert_eq!(
        serde_json::to_value(redone.cloned_data()).unwrap()["players"][0]["units"],
        serde_json::to_value(recruited.cloned_data()).unwrap()["players"][0]["units"]
    );
    assert_eq!(
        server::game_api::log_length(&redone),
        server::game_api::log_length(&recruited)
    );
}
