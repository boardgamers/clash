use server::action::{Action, execute_action};
use server::collect::{Collect, PositionCollection};
use server::game::Game;
use server::game_api::{log_length, log_slice, platform_log_slice};
use server::log::{LogSliceOptions, current_turn_log_mut};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;
mod common;

fn options(start: usize, end: Option<usize>) -> LogSliceOptions {
    LogSliceOptions {
        player: None,
        start,
        end,
    }
}

#[test]
fn platform_delta_has_readable_text_and_preserves_replay_action() {
    let game = common::JsonTest::new("advances").load_game("collect_free_economy");
    let before = log_length(&game);
    let action = Action::Playing(PlayingAction::Collect(Collect::new(
        Position::from_offset("C2"),
        vec![PositionCollection::new(
            Position::from_offset("B1"),
            ResourcePile::ore(1),
        )],
        PlayingActionType::Collect,
    )));
    let game = execute_action(game, action.clone(), 0);
    let entries = platform_log_slice(&game, &options(before, None));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, &action);
    assert_eq!(entries[0].player, 0);
    assert!(entries[0].text.contains("1 ore"));
    assert!(
        entries[0]
            .text
            .starts_with(&format!("{}: ", game.players[0].civilization.name))
    );
    assert_eq!(log_slice(&game, &options(before, None)), vec![action]);
    assert_eq!(
        platform_log_slice(&game, &options(before, Some(before))).len(),
        1
    );
    assert!(platform_log_slice(&game, &options(before + 1, Some(before))).is_empty());
    let undone = execute_action(game, Action::Undo, 0);
    assert!(platform_log_slice(&undone, &options(before, None)).is_empty());
    assert!(platform_log_slice(&undone, &options(usize::MAX, None)).is_empty());
}

fn append(game: &mut Game, entry: serde_json::Value) {
    let index = game.log_index;
    let log = current_turn_log_mut(game);
    log.actions.truncate(index);
    log.actions.push(serde_json::from_value(entry).unwrap());
    game.log_index += 1;
}

#[test]
fn public_summary_does_not_reveal_drawn_cards_or_private_log_prose() {
    let mut game = common::JsonTest::new("advances").load_game("collect_free_economy");
    let start = log_length(&game);
    append(
        &mut game,
        serde_json::json!({"player":0,"action":{"Response":{"Bool":true}},
      "items":[{"player":0,"origin":{"Ability":"Draw"},"HandCard":{"card":{"ActionCard":1},"from":"DrawPile","to":{"Hand":0}}}],
      "log":["Private card: Peltasts"]}),
    );
    assert_eq!(
        platform_log_slice(&game, &options(start, None))[0].text,
        format!("{}: Drew an action card", game.players[0].civilization.name)
    );
    let mut private = options(start, None);
    private.player = Some(0);
    assert_eq!(
        platform_log_slice(&game, &private)[0].text,
        format!("{}: Drew an action card", game.players[0].civilization.name)
    );
}

#[test]
fn latest_response_describes_the_actual_public_outcome() {
    let mut game = common::JsonTest::new("advances").load_game("collect_free_economy");
    let start = log_length(&game);
    append(
        &mut game,
        serde_json::json!({"player":0,"action":{"Response":{"SelectAdvance":"SteelWeapons"}},
      "items":[{"player":0,"origin":{"CivilCard":1},"Advance":{"advance":"SteelWeapons","balance":"Gain"}}]}),
    );
    assert_eq!(
        platform_log_slice(&game, &options(start, None))[0].text,
        format!(
            "{}: Researched Steel Weapons",
            game.players[0].civilization.name
        )
    );
    append(
        &mut game,
        serde_json::json!({"player":0,"action":{"Playing":"EndTurn"}}),
    );
    assert_eq!(
        platform_log_slice(&game, &options(start + 1, None))[0].text,
        format!("{}: Ended turn", game.players[0].civilization.name)
    );
    append(
        &mut game,
        serde_json::json!({"player":1,"action":"StartTurn"}),
    );
    assert_eq!(
        platform_log_slice(&game, &options(start + 2, None))[0].text,
        format!("{}: Started turn", game.players[1].civilization.name)
    );
    assert_eq!(
        platform_log_slice(&game, &options(start + 2, None))[0].kind,
        Some("phase")
    );
}

#[test]
fn founding_summary_does_not_describe_spending_the_settler_as_a_casualty() {
    let mut game = common::JsonTest::new("advances").load_game("collect_free_economy");
    let start = log_length(&game);
    append(
        &mut game,
        serde_json::json!({"player":0,"action":{"Playing":{"FoundCity":{"settler":1}}},
        "items":[
          {"player":0,"origin":{"Ability":"Found City"},"Units":{"units":{"settlers":1},"balance":"Pay","position":"D9"}},
          {"player":0,"origin":{"Ability":"Found City"},"Structure":{"structure":"CityCenter","balance":"Gain","position":"D9"}}
        ]}),
    );
    assert_eq!(
        platform_log_slice(&game, &options(start, None))[0].text,
        format!("{}: Founded city at D9", game.players[0].civilization.name)
    );
}

#[test]
fn resource_summary_names_the_actor_and_other_recipients() {
    let mut game = common::JsonTest::new("advances").load_game("collect_free_economy");
    let start = log_length(&game);
    append(
        &mut game,
        serde_json::json!({
            "player": 0, "action": {"Response": {"Bool": true}},
            "items": [
                {"player": 0, "origin": {"Ability": "Reward"}, "Resources": {"resources": {"gold": 1}, "balance": "Gain"}},
                {"player": 1, "origin": {"Ability": "Reward"}, "Resources": {"resources": {"gold": 2}, "balance": "Gain"}}
            ]
        }),
    );
    assert_eq!(
        platform_log_slice(&game, &options(start, None))[0].text,
        format!(
            "{}: Gained 1 gold · {}: Gained 2 gold",
            game.players[0].civilization.name, game.players[1].civilization.name
        )
    );
}
