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

fn pending_incident(id: u8, third_player: bool) -> Game {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "test_games/incidents/great_persons/great_artist.outcome.json"
    )).unwrap();
    value["events"][0]["event_type"]["Incident"]["incident_id"] = serde_json::json!(id);
    value["events"][0]["handler"]["origin"] = serde_json::json!({"Incident": id});
    value["events"][0]["handler"]["request"]["Payment"][0]["cost"]["origin"] = serde_json::json!({"Incident": id});
    if third_player {
        let mut extra = value["players"][1].clone();
        extra["id"] = serde_json::json!(2);
        extra["civilization"] = serde_json::json!("Egypt");
        extra["cities"] = serde_json::json!([]);
        extra["units"] = serde_json::json!([]);
        value["players"][2]["id"] = serde_json::json!(3);
        value["players"].as_array_mut().unwrap().insert(2, extra);
    }
    let data: GameData = serde_json::from_value(value).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

#[test]
fn great_person_queue_and_wait_survive_pass_to_next_player() {
    let mut game = pending_incident(19, true);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    assert!(action_queue::offer(&game, 2).is_none());
    action_queue::save(&mut game, 0, &key,
        vec![action(serde_json::json!({"Playing":"EndTurn"}))], true).unwrap();
    game = board_history::execute(game, action(serde_json::json!({"Response":{"Payment":[{}]}})), 1).unwrap();
    assert_eq!(game.active_player(), 2);
    assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
    assert!(game.action_plan.as_ref().unwrap().waiting);
    assert_eq!(game.action_plan.as_ref().unwrap().actions.len(), 1);
    assert_eq!(game.current_player_index, 0);
    game = board_history::execute(game.clone(), action(serde_json::json!({"Response":{"Payment":[{"culture_tokens":2}]}})), 2).unwrap();
    assert!(game.action_plan.is_none());
    assert_eq!(game.current_player_index, 1);
    assert!(game.player(2).action_cards.contains(&119));
}

#[test]
fn population_boom_and_migration_allow_waiting_for_settler_placement() {
    use server::content::persistent_events::{PersistentEventRequest, PersistentEventType, PositionRequest};
    for id in [27, 28, 34, 35] {
        let mut game = pending_incident(id, false);
        let positions = game.player(1).cities.iter().map(|city| city.position).collect();
        let event = game.events.last_mut().unwrap();
        if let PersistentEventType::Incident(info) = &mut event.event_type {
            info.selected_player = Some(1);
        }
        event.player.last_priority_used = Some(11);
        let handler = event.player.handler.as_mut().unwrap();
        handler.priority = 11;
        handler.request = PersistentEventRequest::SelectPositions(PositionRequest::new(
            positions, 1..=1, "Select a city to gain 1 settler"));
        let (key, label) = action_queue::offer(&game, 0).unwrap();
        assert_eq!(label, "Placing a settler");
        action_queue::save(&mut game, 0, &key, vec![], true).unwrap();
        assert!(game.action_plan.as_ref().unwrap().waiting);
        assert_eq!(game.active_player(), 1);
        action_queue::save(&mut game, 0, &key,
            vec![action(serde_json::json!({"Playing":"EndTurn"}))], false).unwrap();
        let count = game.player(1).units.len();
        let position = game.player(1).cities[0].position.to_string();
        game = board_history::execute(game.clone(), action(serde_json::json!({"Response":{"SelectPositions":[position]}})), 1).unwrap();
        assert_eq!(game.player(1).units.len(), count + 1);
        assert!(game.action_plan.is_none());
        assert_eq!(game.current_player_index, 1);
    }
}

#[test]
fn great_diplomat_partner_choice_keeps_the_queue() {
    let mut game = pending_incident(57, true);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    action_queue::save(&mut game, 0, &key,
        vec![action(serde_json::json!({"Playing":"EndTurn"}))], false).unwrap();
    game = board_history::execute(game, action(serde_json::json!({"Response":{"Payment":[{"culture_tokens":2}]}})), 1).unwrap();
    assert_eq!(game.active_player(), 1);
    assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
    assert_eq!(game.action_plan.as_ref().unwrap().actions.len(), 1);
    game = board_history::execute(game, action(serde_json::json!({"Response":{"SelectPlayer":0}})), 1).unwrap();
    assert!(game.action_plan.is_none());
    assert_eq!(game.current_player_index, 1);
    assert!(!game.permanent_effects.is_empty());
}

#[test]
fn declining_a_great_person_during_research_opens_planning() {
    let data: GameData = serde_json::from_str(include_str!(
        "test_games/incidents/great_persons/great_engineer.json"
    )).unwrap();
    let cache = Cache::new(&data.options);
    let game = Game::from_data(data, cache, GameContext::Play);
    let game = board_history::execute(game, action(serde_json::json!({"Playing":{"Advance":{
        "advance":"Storage", "payment":{"food":2}
    }}})), 0).unwrap();
    assert!(action_queue::offer(&game, 0).is_none());
    let game = board_history::execute(game, action(serde_json::json!({"Response":{"Payment":[{}]}})), 0).unwrap();
    assert_eq!(game.active_player(), 1);
    assert!(action_queue::offer(&game, 0).is_some());
}
