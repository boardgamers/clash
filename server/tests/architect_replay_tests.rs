use serde_json::{Value, json};
use server::action::Action;
use server::game_api;
use server::playing_actions::PlayingAction;
use server::replay::replay;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/legacy_architect_170.json"))
        .expect("valid Architect replay fixture")
}

#[test]
fn old_architect_history_replays_free_wonder_and_three_following_actions() {
    let fixture = fixture();
    for (cursor, actions_left) in [("pending", 3), ("built", 3), ("full", 0)] {
        let game = replay(
            serde_json::from_value(fixture["history"].clone())
                .expect("valid Architect replay fixture"),
            Some(
                fixture[cursor]
                    .as_u64()
                    .expect("valid Architect replay fixture") as usize,
            ),
        )
        .expect("valid Architect replay fixture");
        assert_eq!(game.actions_left, actions_left);
        if cursor != "pending" {
            assert!(
                game.players[1]
                    .wonders_built
                    .contains(&server::wonder::Wonder::GreatWall)
            );
        }
        // Both independent rule boundaries preserve precisely the retained prefix.
        assert_eq!(
            game.aggressor_eligibility_from,
            Some(game_api::log_length(&game) + 1)
        );
        assert_eq!(
            game.great_architect_action_cost_from,
            Some(game_api::log_length(&game) + 1)
        );
    }
}

#[test]
fn legacy_pending_payment_resumes_free_but_branch_before_card_pays_once() {
    let fixture = fixture();
    let history = || {
        serde_json::from_value(fixture["history"].clone()).expect("valid Architect replay fixture")
    };
    let payment: Action = serde_json::from_value(
        json!({"Response":{"Payment":[{"food":3,"wood":2,"ore":7,"culture_tokens":2}]}}),
    )
    .expect("valid Architect replay fixture");
    let pending = replay(
        history(),
        Some(
            fixture["pending"]
                .as_u64()
                .expect("valid Architect replay fixture") as usize,
        ),
    )
    .expect("valid Architect replay fixture");
    let resumed = game_api::execute(pending, payment.clone(), 1);
    assert_eq!(resumed.actions_left, 3);
    let before = replay(
        history(),
        Some(
            fixture["before"]
                .as_u64()
                .expect("valid Architect replay fixture") as usize,
        ),
    )
    .expect("valid Architect replay fixture");
    let new = game_api::execute(before, Action::Playing(PlayingAction::ActionCard(155)), 1);
    assert_eq!(new.actions_left, 2);
    let new = game_api::execute(new, payment, 1);
    assert_eq!(new.actions_left, 2);
    let saved = serde_json::to_value(new.cloned_data()).expect("valid Architect replay fixture");
    let replayed = replay(
        serde_json::from_value(saved).expect("valid Architect replay fixture"),
        None,
    )
    .expect("valid Architect replay fixture");
    assert_eq!(replayed.actions_left, 2);
    assert_eq!(replayed.players[1].resources, new.players[1].resources);
}

#[test]
fn independent_aggressor_boundary_survives_full_partial_and_new_history_replay() {
    let fixture = fixture();
    let before = fixture["before"].as_u64().unwrap() as usize;
    for boundary in [0, before, before + 4] {
        let mut history = fixture["history"].clone();
        history["aggressor_eligibility_from"] = json!(boundary);
        let partial = replay(
            serde_json::from_value(history.clone()).unwrap(),
            Some(before),
        )
        .unwrap();
        assert_eq!(
            partial.aggressor_eligibility_from,
            Some(boundary.min(before + 1))
        );
        let full = replay(serde_json::from_value(history).unwrap(), None).unwrap();
        assert_eq!(full.aggressor_eligibility_from, Some(boundary));
        let saved = serde_json::to_value(full.cloned_data()).unwrap();
        let replayed = replay(serde_json::from_value(saved).unwrap(), None).unwrap();
        assert_eq!(replayed.aggressor_eligibility_from, Some(boundary));
    }
}

#[test]
fn independent_provinces_boundary_survives_full_partial_and_new_history_replay() {
    let fixture = fixture();
    let before = fixture["before"].as_u64().unwrap() as usize;
    for boundary in [0, before, before + 4] {
        let mut history = fixture["history"].clone();
        history["provinces_from"] = json!(boundary);
        let partial = replay(
            serde_json::from_value(history.clone()).unwrap(),
            Some(before),
        )
        .unwrap();
        assert_eq!(partial.provinces_from, Some(boundary.min(before + 1)));
        let full = replay(serde_json::from_value(history).unwrap(), None).unwrap();
        assert_eq!(full.provinces_from, Some(boundary));
        let saved = serde_json::to_value(full.cloned_data()).unwrap();
        let replayed = replay(serde_json::from_value(saved).unwrap(), None).unwrap();
        assert_eq!(replayed.provinces_from, Some(boundary));
    }
}

#[test]
fn architect_redo_keeps_recorded_cost_policy_but_replacing_future_charges() {
    let fixture = fixture();
    let history = || serde_json::from_value(fixture["history"].clone()).unwrap();
    for legacy in [false, true] {
        let pending = if legacy {
            replay(
                history(),
                Some(fixture["pending"].as_u64().unwrap() as usize),
            )
            .unwrap()
        } else {
            let before = replay(
                history(),
                Some(fixture["before"].as_u64().unwrap() as usize),
            )
            .unwrap();
            game_api::execute(before, Action::Playing(PlayingAction::ActionCard(155)), 1)
        };
        let boundary = pending.great_architect_action_cost_from;
        assert_eq!(pending.actions_left, if legacy { 3 } else { 2 });
        let undone = game_api::execute(pending, Action::Undo, 1);
        assert_eq!(undone.actions_left, 3);
        assert_eq!(undone.great_architect_action_cost_from, boundary);
        let redone = game_api::execute(undone.clone(), Action::Redo, 1);
        assert_eq!(redone.actions_left, if legacy { 3 } else { 2 });
        assert_eq!(redone.great_architect_action_cost_from, boundary);
        let branch = game_api::execute(undone, Action::Playing(PlayingAction::ActionCard(155)), 1);
        assert_eq!(branch.actions_left, 2);
        assert_eq!(
            branch.great_architect_action_cost_from,
            Some(fixture["pending"].as_u64().unwrap() as usize)
        );
    }
}
