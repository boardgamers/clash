use serde_json::{Value, json};
use server::{
    board_history,
    cache::Cache,
    early_choices,
    game::{Game, GameContext},
    game_data::GameData,
};

fn load(value: Value) -> Game {
    let data: GameData = serde_json::from_value(value).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}
fn data(game: &Game) -> Value {
    serde_json::to_value(game.cloned_data()).unwrap()
}
fn play(game: Game, player: usize, action: Value) -> Game {
    board_history::execute(game, serde_json::from_value(action).unwrap(), player).unwrap()
}
fn battle() -> Game {
    let mut v: Value =
        serde_json::from_str(include_str!("test_games/tactics_cards/peltasts.json")).unwrap();
    for p in [0, 1] {
        v["players"][p]["advances"]
            .as_array_mut()
            .unwrap()
            .push(json!("SteelWeapons"));
    }
    v["players"][0]["action_cards"] = json!([3, 6, 11]);
    v["players"][1]["action_cards"] = json!([4, 7, 23]);
    play(
        load(v),
        0,
        json!({"Movement":{"Move":{"units":[0,1,2],"destination":"C1","payment":{}}}}),
    )
}
fn key(g: &Game, p: usize) -> String {
    early_choices::offers(g, p)
        .into_iter()
        .find(|o| o.key.ends_with("/tactics"))
        .unwrap()
        .key
}
fn queue(g: Game, p: usize, card: Option<u8>) -> Game {
    let k = key(&g, p);
    let cards: Vec<Value> = card
        .into_iter()
        .map(|id| json!({"ActionCard":id}))
        .collect();
    play(
        g,
        p,
        json!({"QueueChoice":{"key":k,"response":{"SelectHandCards":cards}}}),
    )
}
fn steel(g: Game, p: usize) -> Game {
    play(g, p, json!({"Response":{"Payment":[{"ore":1}]}}))
}

#[test]
fn private_metadata_edits_leave_payment_hand_rng_history_and_timing_untouched() {
    let g = battle();
    let original = data(&g);
    let enemy = data(&server::game_api::strip_secret(g.clone(), Some(1)));
    let spectator = data(&server::game_api::strip_secret(g.clone(), None));
    let k = key(&g, 0);
    let mut g = queue(g, 0, Some(3));
    for card in [None, Some(6), Some(3)] {
        g = queue(g, 0, card);
    }
    assert_eq!(
        data(&server::game_api::strip_secret(g.clone(), Some(1))),
        enemy
    );
    assert_eq!(
        data(&server::game_api::strip_secret(g.clone(), None)),
        spectator
    );
    let mut edited = data(&g);
    edited.as_object_mut().unwrap().remove("early_choices");
    edited
        .as_object_mut()
        .unwrap()
        .remove("early_choice_update");
    assert_eq!(edited, original);
    assert_eq!(g.active_player(), 0);
    g = play(g, 0, json!({"QueueChoice":{"key":k,"response":null}}));
    let mut cancelled = data(&g);
    cancelled
        .as_object_mut()
        .unwrap()
        .remove("early_choice_update");
    assert_eq!(cancelled, original);
}

#[test]
fn consume_only_at_real_prompt_once_and_keep_card_facedown_until_other_choice() {
    let g = queue(battle(), 0, Some(3));
    let k = key(&g, 0);
    let g = steel(g, 0);
    assert!(g.player(0).action_cards.contains(&3));
    assert_eq!(g.early_choices.len(), 1);
    let g = steel(load(data(&g)), 1);
    assert_eq!(g.active_player(), 1);
    assert!(!g.player(0).action_cards.contains(&3));
    assert!(g.early_choices.is_empty());
    assert!(g.current_event_handler().is_some());
    let hidden = data(&server::game_api::strip_secret(g.clone(), Some(1)));
    assert_eq!(
        hidden["events"][0]["event_type"]["CombatRoundStart"]["attacker_strength"]["tactics_card"],
        json!(0)
    );
    assert!(!early_choices::can_save(&g, 0, &k, None));
    assert!(!early_choices::can_save(&g, 1, &k, None)); // live tactics is not a queued choice
    let done = play(
        g,
        1,
        json!({"Response":{"SelectHandCards":[{"ActionCard":4}]}}),
    );
    assert!(done.early_choices.is_empty());
    assert!(!done.player(1).action_cards.contains(&4));
}

#[test]
fn both_players_queue_and_automatic_execution_matches_ordinary_responses() {
    let g = battle();
    let queued = queue(queue(g.clone(), 0, Some(3)), 1, Some(4));
    let queued = steel(steel(queued, 0), 1);
    let normal = steel(steel(g, 0), 1);
    let normal = play(
        normal,
        0,
        json!({"Response":{"SelectHandCards":[{"ActionCard":3}]}}),
    );
    let normal = play(
        normal,
        1,
        json!({"Response":{"SelectHandCards":[{"ActionCard":4}]}}),
    );
    for field in [
        "players",
        "events",
        "rng",
        "dice_roll_log",
        "log",
        "time_increments",
        "action_cards_discarded",
    ] {
        assert_eq!(data(&queued)[field], data(&normal)[field], "{field}");
    }
    assert!(queued.early_choices.is_empty());
}

#[test]
fn missing_card_and_scout_after_enemy_pass_fall_back_to_real_prompt() {
    let mut g = queue(battle(), 0, Some(3));
    g.players[0].action_cards.retain(|c| *c != 3);
    let g = steel(steel(g, 0), 1);
    assert_eq!(g.active_player(), 0);
    assert!(g.early_choices.is_empty());
    let g = queue(queue(battle(), 0, None), 1, Some(23));
    let g = steel(steel(g, 0), 1);
    assert_eq!(g.active_player(), 1);
    assert!(g.early_choices.is_empty());
    assert!(g.player(1).action_cards.contains(&23));
}

#[test]
fn skipped_tactics_ability_expires_instead_of_surviving_battle() {
    let mut g = queue(battle(), 1, Some(4));
    if let server::content::persistent_events::PersistentEventType::CombatStart(c) =
        &mut g.events[0].event_type
    {
        c.modifiers
            .push(server::combat::CombatModifier::TrojanHorse);
    }
    let g = queue(g, 0, None);
    let g = steel(steel(g, 0), 1);
    assert!(g.early_choices.is_empty());
    assert!(g.player(1).action_cards.contains(&4));
}

#[test]
fn undoing_battle_start_removes_plan_and_redo_does_not_resurrect_it() {
    let g = queue(battle(), 0, Some(3));
    let g = play(g, 0, json!("Undo"));
    assert!(g.early_choices.is_empty());
    let g = play(g, 0, json!("Redo"));
    assert!(g.early_choices.is_empty());
    assert!(g.player(0).action_cards.contains(&3));
}

#[test]
fn hidden_hand_during_seat_switch_never_becomes_an_offer() {
    let g = server::game_api::strip_secret(battle(), Some(0));
    assert!(
        early_choices::offers(&g, 1)
            .iter()
            .all(|o| !o.key.ends_with("/tactics"))
    );
}

#[test]
fn martyr_queue_does_not_choose_sacrifices_or_other_effect_responses() {
    let mut g = battle();
    g.players[0].action_cards.push(24);
    let g = queue(queue(g, 0, Some(24)), 1, Some(4));
    let g = steel(steel(g, 0), 1);
    assert!(g.early_choices.is_empty());
    let handler = g.current_event_handler().unwrap();
    assert!(matches!(
        handler.request,
        server::content::persistent_events::PersistentEventRequest::SelectUnits(_)
    ));
    assert_eq!(g.active_player(), 0);
}

#[test]
fn waiting_defender_can_queue_during_attackers_normal_tactics_prompt() {
    let g = steel(steel(battle(), 0), 1);
    assert_eq!(g.active_player(), 0);
    let k = key(&g, 1);
    let g = queue(g, 1, Some(4));
    assert_eq!(g.active_player(), 0);
    assert!(g.player(1).action_cards.contains(&4));
    let g = play(
        g,
        0,
        json!({"Response":{"SelectHandCards":[{"ActionCard":3}]}}),
    );
    assert!(g.early_choices.is_empty());
    assert!(!g.player(1).action_cards.contains(&4));
    assert!(!early_choices::can_save(&g, 1, &k, None));
}

#[test]
fn viewer_offers_tactics_only_after_own_steel_and_only_while_waiting() {
    let offered = |g: &Game, p| {
        server::web_view::view(g, Some(p))["earlyChoices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["key"].as_str().unwrap().ends_with("/tactics"))
    };
    let g = battle();
    assert!(!offered(&g, 0));
    assert!(!offered(&g, 1));
    let g = steel(g, 0);
    assert!(offered(&g, 0));
    assert!(!offered(&g, 1));
    let g = queue(g, 0, Some(3));
    let g = steel(g, 1);
    assert!(!offered(&g, 1)); // defender now has the ordinary live tactics request
}

#[test]
fn stale_round_and_finished_battle_plans_are_pruned() {
    let mut g = queue(battle(), 0, Some(3));
    let old_key = key(&g, 0);
    if let server::content::persistent_events::PersistentEventType::CombatStart(c) =
        &mut g.events[0].event_type
    {
        c.stats.round += 1;
    }
    assert_ne!(key(&g, 0), old_key);
    early_choices::prune_objectives(&mut g);
    assert!(g.early_choices.is_empty());
    assert!(!early_choices::can_save(&g, 0, &old_key, None));
    let mut g = queue(g, 0, Some(3));
    g.events.clear();
    early_choices::prune_objectives(&mut g);
    assert!(g.early_choices.is_empty());
    assert!(g.player(0).action_cards.contains(&3));
}
