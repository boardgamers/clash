use server::{
    action::Action,
    action_queue, board_history,
    cache::Cache,
    game::{Game, GameContext},
    game_data::GameData,
};
fn action(value: serde_json::Value) -> Action {
    serde_json::from_value(value).unwrap()
}
fn setup() -> Game {
    let mut data: serde_json::Value =
        serde_json::from_str(include_str!("test_games/action_cards/tech_trade.json")).unwrap();
    data["round"] = serde_json::json!(2);
    data["players"][0]["advances"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!("Irrigation"));
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let game = Game::from_data(data, cache, GameContext::Play);
    board_history::execute(
        game,
        action(serde_json::json!({"Playing":{"ActionCard":39}})),
        0,
    )
    .unwrap()
}
#[test]
fn tech_trade_plan_is_private_and_replays_after_response() {
    let mut game = setup();
    let (key, _) = action_queue::offer(&game, 0).expect("trade recipient still choosing");
    let original = game.player(0).resources.clone();
    action_queue::save(
        &mut game,
        0,
        &key,
        vec![action(serde_json::json!({"Playing":"EndTurn"}))],
        false,
    )
    .unwrap();
    assert_eq!(game.player(0).resources, original);
    assert_eq!(game.active_player(), 1);
    assert!(
        server::game_api::strip_secret(game.clone(), Some(1))
            .action_plan
            .is_none()
    );
    assert!(
        server::game_api::strip_secret(game.clone(), Some(0))
            .action_plan
            .is_some()
    );
    // Persist/reload: execution does not require the planning player's browser.
    let data: GameData =
        serde_json::from_str(&serde_json::to_string(&game.cloned_data()).unwrap()).unwrap();
    game = Game::from_data(data, game.cache.clone(), GameContext::Play);
    game = board_history::execute(
        game,
        action(serde_json::json!({"Response":{"SelectAdvance":"Fishing"}})),
        1,
    )
    .unwrap();
    assert!(game.action_plan.is_none());
    assert!(game.events.is_empty());
    assert_eq!(game.current_player_index, 1);
}
#[test]
fn queue_rejects_wrong_player_stale_key_and_nonterminal_end_turn() {
    let mut game = setup();
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    assert!(action_queue::save(&mut game, 1, &key, vec![], false).is_err());
    assert!(action_queue::save(&mut game, 0, "stale", vec![], false).is_err());
    let end = action(serde_json::json!({"Playing":"EndTurn"}));
    assert!(action_queue::save(&mut game, 0, &key, vec![end.clone(), end], false).is_err());
}
#[test]
fn waiting_and_clearing_do_not_advance_the_game() {
    let mut game = setup();
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    action_queue::save(&mut game, 0, &key, vec![], true).unwrap();
    assert!(game.action_plan.as_ref().unwrap().waiting);
    assert_eq!(game.active_player(), 1);
    assert_eq!(game.current_player_index, 0);
    action_queue::save(&mut game, 0, &key, vec![], false).unwrap();
    assert!(!game.action_plan.as_ref().unwrap().waiting);
}
#[test]
fn replacement_settler_resolves_before_queued_collection() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../web-client/scripts/async-preview-fixture.json"
    ))
    .unwrap();
    value["state"] = serde_json::json!("Playing");
    value["actions_left"] = serde_json::json!(3);
    value["players"][0]["cities"][0]["activations"] = serde_json::json!(0);
    value["players"][0]["resources"]["food"] = serde_json::json!(1);
    let data: GameData = serde_json::from_value(value).unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    let collect = action(
        serde_json::json!({"Playing":{"Collect":{"city_position":"A1","collections":[{"position":"A1","pile":{"food":1},"times":1}],"action_type":"Collect"}}}),
    );
    action_queue::save(&mut game, 0, &key, vec![collect], false).unwrap();
    assert_eq!(game.player(0).resources.food, 1);
    let units = game.player(1).units.len();
    game = board_history::execute(
        game,
        action(serde_json::json!({"Response":{"SelectPositions":["B2"]}})),
        1,
    )
    .unwrap();
    assert_eq!(game.player(1).units.len(), units + 1);
    assert_eq!(game.player(0).resources.food, 2);
    assert!(game.action_plan.is_none());
}
#[test]
fn battles_and_illegal_moves_cannot_enter_the_plan() {
    let mut game = setup();
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    let enemy = game.players[1]
        .units
        .iter()
        .find(|u| u.unit_type != server::unit::UnitType::Settler)
        .unwrap()
        .position
        .to_string();
    let attack = action(serde_json::json!({"Movement":{"Move":{"units":[0],"destination":enemy}}}));
    assert!(action_queue::save(&mut game, 0, &key, vec![attack], false).is_err());
    assert!(game.action_plan.is_none());
    assert!(action_queue::save(&mut game, 0, &key, vec![Action::Undo], false).is_err());
}
#[test]
fn event_triggering_research_is_not_queued() {
    let mut game = setup();
    game.player_mut(0).incident_tokens = 1;
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    let research = action(
        serde_json::json!({"Playing":{"Advance":{"advance":"Husbandry","payment":{"food":2}}}}),
    );
    let projected = action_queue::project(&game, 0, &[]).unwrap();
    assert!(server::action::try_execute_action(projected, research.clone(), 0).is_ok());
    assert!(action_queue::save(&mut game, 0, &key, vec![research], false).is_err());
    assert!(game.action_plan.is_none());
}
