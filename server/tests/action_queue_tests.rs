use server::{
    action::Action,
    action_queue, board_history,
    cache::Cache,
    content::persistent_events::{EventResponse, PersistentEventRequest, PersistentEventType},
    events::EventOrigin,
    game::{Game, GameContext},
    game_data::GameData,
    map::Terrain,
    position::Position,
    resource_pile::ResourcePile,
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
    ))
    .unwrap();
    value["events"][0]["event_type"]["Incident"]["incident_id"] = serde_json::json!(id);
    value["events"][0]["handler"]["origin"] = serde_json::json!({"Incident": id});
    value["events"][0]["handler"]["request"]["Payment"][0]["cost"]["origin"] =
        serde_json::json!({"Incident": id});
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
    action_queue::save(
        &mut game,
        0,
        &key,
        vec![action(serde_json::json!({"Playing":"EndTurn"}))],
        true,
    )
    .unwrap();
    game = board_history::execute(
        game,
        action(serde_json::json!({"Response":{"Payment":[{}]}})),
        1,
    )
    .unwrap();
    assert_eq!(game.active_player(), 2);
    assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
    assert!(game.action_plan.as_ref().unwrap().waiting);
    assert_eq!(game.action_plan.as_ref().unwrap().actions.len(), 1);
    assert_eq!(game.current_player_index, 0);
    game = board_history::execute(
        game.clone(),
        action(serde_json::json!({"Response":{"Payment":[{"culture_tokens":2}]}})),
        2,
    )
    .unwrap();
    assert!(game.action_plan.is_none());
    assert_eq!(game.current_player_index, 1);
    assert!(game.player(2).action_cards.contains(&119));
}

#[test]
fn population_boom_and_migration_allow_waiting_for_settler_placement() {
    use server::content::persistent_events::{
        PersistentEventRequest, PersistentEventType, PositionRequest,
    };
    for id in [27, 28, 34, 35] {
        let mut game = pending_incident(id, false);
        let positions = game
            .player(1)
            .cities
            .iter()
            .map(|city| city.position)
            .collect();
        let event = game.events.last_mut().unwrap();
        if let PersistentEventType::Incident(info) = &mut event.event_type {
            info.selected_player = Some(1);
        }
        event.player.last_priority_used = Some(11);
        let handler = event.player.handler.as_mut().unwrap();
        handler.priority = 11;
        handler.request = PersistentEventRequest::SelectPositions(PositionRequest::new(
            positions,
            1..=1,
            "Select a city to gain 1 settler",
        ));
        let (key, label) = action_queue::offer(&game, 0).unwrap();
        assert_eq!(label, "Placing a settler");
        action_queue::save(&mut game, 0, &key, vec![], true).unwrap();
        assert!(game.action_plan.as_ref().unwrap().waiting);
        assert_eq!(game.active_player(), 1);
        action_queue::save(
            &mut game,
            0,
            &key,
            vec![action(serde_json::json!({"Playing":"EndTurn"}))],
            false,
        )
        .unwrap();
        let count = game.player(1).units.len();
        let position = game.player(1).cities[0].position.to_string();
        game = board_history::execute(
            game.clone(),
            action(serde_json::json!({"Response":{"SelectPositions":[position]}})),
            1,
        )
        .unwrap();
        assert_eq!(game.player(1).units.len(), count + 1);
        assert!(game.action_plan.is_none());
        assert_eq!(game.current_player_index, 1);
    }
}

#[test]
fn great_diplomat_partner_choice_keeps_the_queue() {
    let mut game = pending_incident(57, true);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    action_queue::save(
        &mut game,
        0,
        &key,
        vec![action(serde_json::json!({"Playing":"EndTurn"}))],
        false,
    )
    .unwrap();
    game = board_history::execute(
        game,
        action(serde_json::json!({"Response":{"Payment":[{"culture_tokens":2}]}})),
        1,
    )
    .unwrap();
    assert_eq!(game.active_player(), 1);
    assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
    assert_eq!(game.action_plan.as_ref().unwrap().actions.len(), 1);
    game = board_history::execute(
        game,
        action(serde_json::json!({"Response":{"SelectPlayer":0}})),
        1,
    )
    .unwrap();
    assert!(game.action_plan.is_none());
    assert_eq!(game.current_player_index, 1);
    assert!(!game.permanent_effects.is_empty());
}

#[test]
fn declining_a_great_person_during_research_opens_planning() {
    let data: GameData = serde_json::from_str(include_str!(
        "test_games/incidents/great_persons/great_engineer.json"
    ))
    .unwrap();
    let cache = Cache::new(&data.options);
    let game = Game::from_data(data, cache, GameContext::Play);
    let game = board_history::execute(
        game,
        action(serde_json::json!({"Playing":{"Advance":{
            "advance":"Storage", "payment":{"food":2}
        }}})),
        0,
    )
    .unwrap();
    assert!(action_queue::offer(&game, 0).is_none());
    let game = board_history::execute(
        game,
        action(serde_json::json!({"Response":{"Payment":[{}]}})),
        0,
    )
    .unwrap();
    assert_eq!(game.active_player(), 1);
    assert!(action_queue::offer(&game, 0).is_some());
}

fn era_of_stability_game(third_player: bool) -> Game {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "test_games/incidents/trade/era_of_stability.json"
    ))
    .unwrap();
    if third_player {
        let mut extra = value["players"][1].clone();
        extra["id"] = serde_json::json!(2);
        extra["civilization"] = serde_json::json!("Egypt");
        extra["cities"] = serde_json::json!([
            {"position": "B3", "mood_state": "Happy", "city_pieces": {"temple": 2}}
        ]);
        extra["units"] = serde_json::json!([]);
        value["players"][2]["id"] = serde_json::json!(3);
        value["players"].as_array_mut().unwrap().insert(2, extra);
    }
    let data: GameData = serde_json::from_value(value).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

fn trigger_era_of_stability(game: Game) -> Game {
    board_history::execute(
        game,
        action(serde_json::json!({"Playing":{"Advance":{
            "advance":"Storage", "payment":{"gold":2}
        }}})),
        0,
    )
    .unwrap()
}

fn token_reward(game: Game, player: usize, reward: ResourcePile) -> Game {
    board_history::execute(
        game,
        Action::Response(EventResponse::ResourceReward(reward)),
        player,
    )
    .unwrap()
}

fn pending_era_of_stability(third_player: bool) -> Game {
    // Exercise the real incident listeners, including the turn owner's reward,
    // rather than manufacturing a ResourceReward handler for incident 47.
    let game = trigger_era_of_stability(era_of_stability_game(third_player));
    assert_eq!(game.active_player(), 0);
    assert!(action_queue::offer(&game, 0).is_none());
    let game = token_reward(game, 0, ResourcePile::culture_tokens(1));
    assert_eq!(game.active_player(), 1);
    assert_eq!(game.events.len(), 1);
    game
}

fn collect_at_a1() -> Action {
    action(serde_json::json!({"Playing":{"Collect":{
        "city_position":"A1",
        "collections":[{"position":"A1","pile":{"food":1},"times":1}],
        "action_type":"Collect"
    }}}))
}

fn end_turn() -> Action {
    action(serde_json::json!({"Playing":"EndTurn"}))
}

#[test]
fn era_of_stability_off_turn_reward_only_after_exhausted_land_and_own_choice() {
    let mut game = era_of_stability_game(false);
    // The original fixture has only one eligible land tile (auto-resolved).
    // Add a second so the actual exhausted-land choice must be answered.
    game.map
        .tiles
        .insert(Position::from_offset("A2"), Terrain::Fertile);
    let game = trigger_era_of_stability(game);
    let handler = game.current_event_handler().unwrap();
    assert!(matches!(
        handler.request,
        PersistentEventRequest::SelectPositions(_)
    ));
    assert_eq!(handler.origin, EventOrigin::Incident(47));
    assert!(handler.priority > 0);
    assert!(action_queue::offer(&game, 0).is_none());
    // Even if it were another player's exhausted-land choice, it is not safe
    // to project past a map-changing base effect.
    let mut off_turn = game.clone();
    off_turn.events[0].player.index = 1;
    assert!(action_queue::offer(&off_turn, 0).is_none());

    let game = board_history::execute(
        game,
        Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
            "A2",
        )])),
        0,
    )
    .unwrap();
    assert!(matches!(
        game.map.get(Position::from_offset("A2")),
        Some(Terrain::Exhausted(_))
    ));
    assert_eq!(game.active_player(), 0);
    assert!(matches!(
        game.current_event_handler().unwrap().request,
        PersistentEventRequest::ResourceReward(_)
    ));
    assert!(action_queue::offer(&game, 0).is_none());
    assert!(action_queue::project(&game, 0, &[]).is_err());

    let game = token_reward(game, 0, ResourcePile::culture_tokens(1));
    let (_, label) = action_queue::offer(&game, 0).expect("another player's Era reward");
    assert_eq!(label, "Choose resources");
    assert_eq!(game.current_event_handler().unwrap().priority, 0);
    assert!(action_queue::offer(&game, 1).is_none());
    assert!(action_queue::offer(&game, 2).is_none());
    let projected = action_queue::project(&game, 0, &[collect_at_a1()]).unwrap();
    assert_eq!(
        projected.player(0).resources.food,
        game.player(0).resources.food + 1
    );
    assert!(projected.events.is_empty());
    assert_eq!(
        game.events.len(),
        1,
        "the authoritative incident stays pending"
    );
}

#[test]
fn era_of_stability_queue_keeps_narrow_incident_and_game_guards() {
    let game = pending_era_of_stability(false);
    assert!(action_queue::offer(&game, 0).is_some());
    let value = serde_json::to_value(game.cloned_data()).unwrap();
    // Each variant retains an off-turn reward but breaks one required guard.
    for (pointer, replacement) in [
        (
            "/events/0/event_type/Incident/incident_id",
            serde_json::json!(54),
        ),
        (
            "/events/0/event_type/Incident/active_player",
            serde_json::json!(1),
        ),
        (
            "/events/0/handler/origin",
            serde_json::json!({"Incident": 54}),
        ),
        (
            "/events/0/handler/origin",
            serde_json::json!({"Advance": "Nationalism"}),
        ),
        ("/events/0/handler/priority", serde_json::json!(1)),
        ("/events/0/handler/priority", serde_json::json!(10)),
        (
            "/events/0/event_type",
            serde_json::json!({"CityActivationMoodDecreased":"C1"}),
        ),
        ("/round", serde_json::json!(4)),
    ] {
        let mut changed = value.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        let data: GameData = serde_json::from_value(changed).unwrap();
        let changed = Game::from_data(data, game.cache.clone(), GameContext::Play);
        assert!(
            action_queue::offer(&changed, 0).is_none(),
            "guard: {pointer}"
        );
    }
    for field in ["consumed", "replaced_by_sacrifice", "selected_player"] {
        let mut changed = value.clone();
        changed["events"][0]["event_type"]["Incident"][field] = if field == "selected_player" {
            serde_json::json!(1)
        } else {
            serde_json::json!(true)
        };
        let data: GameData = serde_json::from_value(changed).unwrap();
        let changed = Game::from_data(data, game.cache.clone(), GameContext::Play);
        assert!(action_queue::offer(&changed, 0).is_none(), "guard: {field}");
    }
    let mut nested = game.clone();
    nested.events.push(nested.events[0].clone());
    assert!(action_queue::offer(&nested, 0).is_none());
    let mut drafting = game;
    drafting.civilization_draft = Some(
        serde_json::from_value(serde_json::json!({
            "offers":[], "choices":[], "ready":[], "random_map":false
        }))
        .unwrap(),
    );
    assert!(action_queue::offer(&drafting, 0).is_none());
}

#[test]
fn other_incident_rewards_and_mood_loss_choices_do_not_open_queue() {
    let game = pending_era_of_stability(false);
    // Other incidents' resource rewards are not covered by the Era exception,
    // even at the same priority and with a matching incident origin.
    for id in [1, 26, 45, 46, 54] {
        let mut other = game.clone();
        if let PersistentEventType::Incident(i) = &mut other.events[0].event_type {
            i.incident_id = id;
        }
        let handler = other.events[0].player.handler.as_mut().unwrap();
        handler.origin = EventOrigin::Incident(id);
        if let PersistentEventRequest::ResourceReward(r) = &mut handler.request {
            r.reward.payment_options.origin = EventOrigin::Incident(id);
        }
        assert!(action_queue::offer(&other, 0).is_none(), "incident {id}");
    }
    // Real off-turn Myths payment protects cities from Pestilence mood loss.
    let data: GameData = serde_json::from_str(include_str!(
        "test_games/incidents/famine/pestilence.outcome2.json"
    ))
    .unwrap();
    let cache = Cache::new(&data.options);
    let game = Game::from_data(data, cache, GameContext::Play);
    assert_eq!(game.current_player_index, 0);
    assert_eq!(game.active_player(), 1);
    assert!(matches!(
        game.current_event_handler().unwrap().request,
        PersistentEventRequest::Payment(_)
    ));
    assert!(action_queue::offer(&game, 0).is_none());
    assert!(action_queue::project(&game, 0, &[collect_at_a1()]).is_err());
}

#[test]
fn era_of_stability_queue_and_wait_survive_all_recipients_then_replay() {
    let mut game = pending_era_of_stability(true);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    let before = game.player(0).resources.clone();
    let actions_left = game.actions_left;
    let actions = vec![collect_at_a1(), end_turn()];
    action_queue::save(&mut game, 0, &key, actions.clone(), true).unwrap();
    let saved = game.action_plan.clone();
    assert_eq!(game.player(0).resources, before);
    assert_eq!(game.actions_left, actions_left);
    assert!(
        server::game_api::strip_secret(game.clone(), Some(1))
            .action_plan
            .is_none()
    );

    let reward_before = game.player(1).resources.culture_tokens;
    game = token_reward(game, 1, ResourcePile::culture_tokens(1));
    assert_eq!(game.player(1).resources.culture_tokens, reward_before + 1);
    assert_eq!(game.active_player(), 2);
    assert_eq!(game.current_player_index, 0);
    assert_eq!(game.events.len(), 1);
    assert_eq!(
        game.action_plan, saved,
        "handoff must not replay or reset Wait"
    );
    assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
    assert_eq!(game.player(0).resources, before);
    assert_eq!(game.actions_left, actions_left);
    assert!(action_queue::offer(&game, 1).is_none());
    assert!(action_queue::offer(&game, 2).is_none());

    // Reload during the second recipient's choice: server replay must not
    // depend on the planning player being connected.
    let data = serde_json::from_str(&serde_json::to_string(&game.cloned_data()).unwrap()).unwrap();
    game = Game::from_data(data, game.cache.clone(), GameContext::Play);
    assert_eq!(game.action_plan, saved);
    assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
    let reward_before = game.player(2).resources.mood_tokens;
    game = token_reward(game, 2, ResourcePile::mood_tokens(1));
    assert_eq!(game.player(2).resources.mood_tokens, reward_before + 1);
    assert_eq!(game.player(0).resources.food, before.food + 1);
    assert_eq!(
        game.current_player_index, 1,
        "End turn replays only after all rewards"
    );
    assert!(game.events.is_empty());
    assert!(game.action_plan.is_none());
}

#[test]
fn era_of_stability_wait_only_survives_handoff_without_ending_turn() {
    let mut game = pending_era_of_stability(true);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    let before = game.player(0).resources.clone();
    let actions_left = game.actions_left;
    action_queue::save(&mut game, 0, &key, vec![], true).unwrap();
    game = token_reward(game, 1, ResourcePile::mood_tokens(1));
    assert_eq!(action_queue::offer(&game, 0).unwrap().0, key);
    assert!(game.action_plan.as_ref().unwrap().waiting);
    assert!(game.action_plan.as_ref().unwrap().actions.is_empty());
    game = token_reward(game, 2, ResourcePile::culture_tokens(1));
    assert!(game.events.is_empty());
    assert!(game.action_plan.is_none());
    assert_eq!(game.current_player_index, 0);
    assert_eq!(game.actions_left, actions_left);
    assert_eq!(game.player(0).resources, before);
}

#[test]
fn era_of_stability_end_turn_is_terminal_and_queued_collection_can_be_undone() {
    let mut game = pending_era_of_stability(false);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    assert!(
        action_queue::save(&mut game, 0, &key, vec![end_turn(), collect_at_a1()], false).is_err()
    );
    action_queue::save(&mut game, 0, &key, vec![collect_at_a1(), end_turn()], false).unwrap();
    let before = game.player(0).resources.clone();
    let actions_left = game.actions_left;
    // The UI's queued Undo replaces the plan, not the authoritative history.
    game = board_history::execute(
        game,
        Action::QueueMoves {
            key: key.clone(),
            actions: vec![collect_at_a1()],
            waiting: false,
        },
        0,
    )
    .unwrap();
    assert_eq!(game.active_player(), 1);
    assert_eq!(game.player(0).resources, before);
    game = token_reward(game, 1, ResourcePile::mood_tokens(1));
    assert_eq!(game.current_player_index, 0);
    assert_eq!(game.actions_left, actions_left - 1);
    assert_eq!(game.player(0).resources.food, before.food + 1);
    assert!(game.events.is_empty());
    assert!(game.action_plan.is_none());
    assert!(game.can_undo());
    game = board_history::execute(game, Action::Undo, 0).unwrap();
    assert_eq!(game.player(0).resources, before);
    assert_eq!(game.actions_left, actions_left);
    assert!(
        game.events.is_empty(),
        "undo collection must not reopen the incident"
    );
    assert!(
        game.action_plan.is_none(),
        "undo must not immediately replay collection"
    );
}

#[test]
fn era_of_stability_revalidates_after_last_reward_and_retains_failed_suffix() {
    let mut game = pending_era_of_stability(true);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    let actions = vec![collect_at_a1(), end_turn()];
    action_queue::save(&mut game, 0, &key, actions.clone(), false).unwrap();
    game = token_reward(game, 1, ResourcePile::culture_tokens(1));
    assert!(game.action_plan.as_ref().unwrap().error.is_none());
    // Simulate an authoritative change since planning. Replay must validate
    // the real board again rather than trust a previously valid projection.
    game.actions_left = 0;
    let before = game.player(0).resources.clone();
    game = token_reward(game, 2, ResourcePile::mood_tokens(1));
    assert!(game.events.is_empty());
    assert_eq!(
        game.current_player_index, 0,
        "failed collection must not run End turn"
    );
    assert_eq!(game.player(0).resources, before);
    let plan = game.action_plan.as_ref().unwrap();
    assert_eq!(plan.key, key);
    assert_eq!(plan.actions, actions);
    assert!(plan.error.is_some());
    assert!(action_queue::offer(&game, 0).is_none());
    action_queue::save(&mut game, 0, &key, vec![], false).unwrap();
    assert!(game.action_plan.is_none());
}
