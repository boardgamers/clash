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
fn steel(g: Game, p: usize) -> Game {
    play(g, p, json!({"Response":{"Payment":[{"ore":1}]}}))
}
fn tactics(g: Game, p: usize, card: Option<u8>) -> Game {
    let cards: Vec<_> = card
        .into_iter()
        .map(|id| json!({"ActionCard":id}))
        .collect();
    play(g, p, json!({"Response":{"SelectHandCards":cards}}))
}
fn request(g: &Game) -> Value {
    serde_json::to_value(&g.current_event_handler().unwrap().request).unwrap()
}
#[test]
fn attacker_finishes_before_defender_and_reload_keeps_hidden_card() {
    let g = battle();
    assert_eq!(g.active_player(), 0);
    assert!(request(&g).get("Payment").is_some());
    assert!(
        early_choices::offers(&g, 0)
            .iter()
            .all(|o| !o.key.ends_with("/tactics"))
    );
    let g = steel(load(data(&g)), 0);
    assert_eq!(g.active_player(), 0);
    assert!(request(&g).get("SelectHandCards").is_some());
    let g = tactics(load(data(&g)), 0, Some(3));
    assert_eq!(g.active_player(), 1);
    assert!(request(&g).get("Payment").is_some());
    assert!(!g.player(0).action_cards.contains(&3));
    let hidden = data(&server::game_api::strip_secret(g.clone(), Some(1)));
    assert_eq!(
        hidden["events"][0]["event_type"]["CombatRoundStart"]["attacker_strength"]["tactics_card"],
        json!(0)
    );
    assert!(
        !hidden["action_cards_discarded"]
            .as_array()
            .is_some_and(|cards| cards.contains(&json!(3)))
    );
    let g = steel(load(data(&g)), 1);
    assert_eq!(g.active_player(), 1);
    assert!(request(&g).get("SelectHandCards").is_some());
    let g = tactics(load(data(&g)), 1, Some(4));
    assert!(!g.player(1).action_cards.contains(&4));
}
#[test]
fn skipping_is_an_ordinary_committed_response() {
    let g = tactics(steel(battle(), 0), 0, None);
    assert_eq!(g.active_player(), 1);
    assert!(request(&g).get("Payment").is_some());
    assert!(g.player(0).action_cards.contains(&3));
    let _ = tactics(steel(g, 1), 1, None);
}
#[test]
fn obsolete_combat_plans_are_discarded_but_other_plans_survive_load() {
    let mut v = data(&battle());
    v["early_choices"] = json!([
        {"player":0,"key":"old/tactics","response":{"SelectHandCards":[]}},
        {"player":1,"key":"1:status/advance","response":{"SelectAdvance":"Fishing"}}
    ]);
    let g = load(v);
    assert_eq!(g.early_choices.len(), 1);
    assert_eq!(g.early_choices[0].key, "1:status/advance");
    assert!(!early_choices::can_save(&g, 0, "old/tactics", None));
}
#[test]
fn legacy_battle_continuation_keeps_both_payments_before_tactics() {
    let mut v = data(&battle());
    let round = v["events"][0]["event_type"]["CombatRoundStart"].clone();
    let mut combat = round["combat"].clone();
    combat
        .as_object_mut()
        .unwrap()
        .remove("asynchronous_preparation");
    v["events"][0]["event_type"] = json!({"CombatStart":combat});
    let g = steel(load(v), 0);
    assert_eq!(g.active_player(), 1);
    assert!(request(&g).get("Payment").is_some());
    let g = steel(g, 1);
    assert_eq!(g.active_player(), 0);
    assert!(request(&g).get("SelectHandCards").is_some());
}

fn fixture(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("tests/test_games/{name}.json")).unwrap())
        .unwrap()
}
fn attack(v: Value) -> Game {
    play(
        load(v),
        0,
        json!({"Movement":{"Move":{"units":[0,1,2],"destination":"C1","payment":{}}}}),
    )
}
#[test]
fn no_steel_and_free_metallurgy_do_not_add_a_payment_step() {
    for metallurgy in [false, true] {
        let mut v = fixture("tactics_cards/peltasts");
        v["players"][1]["action_cards"] = json!([4]);
        if metallurgy {
            v["players"][0]["advances"]
                .as_array_mut()
                .unwrap()
                .extend([json!("SteelWeapons"), json!("Metallurgy")]);
        }
        let g = attack(v);
        assert_eq!(g.active_player(), 0);
        assert!(request(&g).get("SelectHandCards").is_some());
        if metallurgy {
            let round = &data(&g)["events"][0]["event_type"]["CombatRoundStart"];
            assert!(
                round["combat"]["modifiers"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("SteelWeaponsAttacker"))
            );
        }
        let g = tactics(g, 0, None);
        assert_eq!(g.active_player(), 1);
        assert!(request(&g).get("SelectHandCards").is_some());
    }
}
#[test]
fn siegecraft_stays_before_attacker_card_and_defender_steel() {
    let mut v = fixture("tactics_cards/peltasts");
    v["players"][0]["advances"]
        .as_array_mut()
        .unwrap()
        .extend([json!("SteelWeapons"), json!("Siegecraft")]);
    v["players"][1]["advances"]
        .as_array_mut()
        .unwrap()
        .push(json!("SteelWeapons"));
    v["players"][1]["cities"][0]["city_pieces"] = json!({"fortress":1});
    let g = steel(attack(v), 0);
    assert_eq!(g.active_player(), 0);
    assert_eq!(request(&g)["Payment"].as_array().unwrap().len(), 2);
    let g = play(
        load(data(&g)),
        0,
        json!({"Response":{"Payment":[{"wood":2},{"ore":2}]}}),
    );
    assert_eq!(g.active_player(), 0);
    assert!(request(&g).get("SelectHandCards").is_some());
    let g = tactics(g, 0, None);
    assert_eq!(g.active_player(), 1);
    assert_eq!(request(&g)["Payment"].as_array().unwrap().len(), 1);
}
#[test]
fn naval_steel_is_not_offered_and_martyr_still_requires_sacrifice_after_both_cards() {
    let mut v = fixture("tactics_cards/martyr");
    for p in [0, 1] {
        v["players"][p]["advances"]
            .as_array_mut()
            .unwrap()
            .push(json!("SteelWeapons"));
    }
    let g = play(
        load(v),
        0,
        json!({"Movement":{"Move":{"units":[7,8],"destination":"D2","payment":{}}}}),
    );
    assert!(request(&g).get("SelectHandCards").is_some());
    let g = tactics(g, 0, Some(6));
    assert_eq!(g.active_player(), 1);
    assert!(request(&g).get("SelectHandCards").is_some());
    let g = tactics(g, 1, Some(24));
    assert_eq!(g.active_player(), 0);
    assert!(request(&g).get("SelectUnits").is_some());
    let g = play(load(data(&g)), 0, json!({"Response":{"SelectUnits":[7]}}));
    assert!(request(&g).get("SelectUnits").is_some());
    let _ = play(g, 0, json!({"Response":{"SelectUnits":[2]}}));
}
#[test]
fn replay_format_boundary_is_explicit_and_absent_in_historical_records() {
    let g = server::game_api::init(
        2,
        "combat-replay-format".into(),
        server::game::GameOptions::default(),
    );
    let player = g.active_player();
    let g = play(g, player, json!({"Playing":"EndTurn"}));
    let mut v = data(&g);
    let replayed =
        server::replay::replay(serde_json::from_value(v.clone()).unwrap(), None).unwrap();
    assert_eq!(replayed.asynchronous_combat_from, Some(0));
    v.as_object_mut()
        .unwrap()
        .remove("asynchronous_combat_from");
    let legacy = server::replay::replay(serde_json::from_value(v).unwrap(), None).unwrap();
    assert!(legacy.asynchronous_combat_from.unwrap() > server::game_api::log_length(&legacy));
}

#[test]
fn own_payment_undo_redo_is_deterministic_but_committed_tactics_cannot_be_revised() {
    let start = battle();
    let paid = steel(start, 0);
    assert!(paid.can_undo());
    let before = data(&paid);
    let undone = play(paid, 0, json!("Undo"));
    assert_eq!(undone.active_player(), 0);
    assert!(request(&undone).get("Payment").is_some());
    let redone = play(undone, 0, json!("Redo"));
    for field in ["players", "events", "rng", "asynchronous_combat_from"] {
        assert_eq!(data(&redone)[field], before[field], "{field}");
    }
    let committed = tactics(redone, 0, Some(3));
    assert!(!committed.can_undo());
    let rejected = server::action::try_execute_action(
        committed,
        serde_json::from_value(json!({"Response":{"SelectHandCards":[]}})).unwrap(),
        0,
    );
    assert!(rejected.is_err());
}

#[test]
fn later_rounds_do_not_repeat_battle_start_payments() {
    let mut g = battle();
    g.dice_roll_outcomes = vec![0; 40];
    let g = tactics(steel(tactics(steel(g, 0), 0, None), 1), 1, None);
    assert!(request(&g).get("BoolRequest").is_some(), "{}", request(&g));
    let g = play(g, 0, json!({"Response":{"Bool":false}}));
    assert_eq!(g.active_player(), 0);
    assert!(request(&g).get("SelectHandCards").is_some());
    let round = &data(&g)["events"][0]["event_type"]["CombatRoundStart"];
    assert_eq!(round["combat"]["stats"]["round"], json!(2));
    assert_eq!(round["preparation"], Value::Null);
    assert_eq!(round["combat"]["modifiers"].as_array().unwrap().len(), 2);
    let g = tactics(g, 0, None);
    assert_eq!(g.active_player(), 1);
    assert!(request(&g).get("SelectHandCards").is_some());
}

fn historical_replay(to: usize) -> Game {
    server::replay::replay(
        serde_json::from_str(include_str!("fixtures/legacy_combat_history.json")).unwrap(),
        Some(to),
    )
    .unwrap()
}
#[test]
fn historical_combat_replay_branches_enable_new_battles() {
    // Generated from a seeded game using legacy sequencing, not a mutated board
    // fixture. Includes completed battles and an open human-vs-human battle.
    let open = historical_replay(217);
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/legacy_combat_history.expected.json")).unwrap();
    for (key, value) in expected.as_object().unwrap() {
        assert_eq!(&data(&open)[key], value, "0.4.153 compatibility: {key}");
    }
    let round = &data(&open)["events"][0]["event_type"]["CombatRoundStart"];
    assert_eq!(round["combat"]["asynchronous_preparation"], Value::Null);
    assert_eq!(open.asynchronous_combat_from, Some(218));
    let before = historical_replay(216);
    assert_eq!(before.asynchronous_combat_from, Some(217));
    let next = play(
        before,
        0,
        json!({"Movement":{"Move":{"destination":"E5","units":[3]}}}),
    );
    let round = &data(&next)["events"][0]["event_type"]["CombatRoundStart"];
    assert_eq!(round["combat"]["asynchronous_preparation"], json!(true));
    // The newly recorded switch survives replay of the branched history.
    let again = server::replay::replay(serde_json::from_value(data(&next)).unwrap(), None).unwrap();
    assert_eq!(data(&again)["events"], data(&next)["events"]);
    let continued = tactics(open, 1, None);
    assert!(
        continued.events.is_empty(),
        "legacy battle finishes normally"
    );

    // Full replay of a legacy save ending before the next battle is playable too.
    let mut saved: Value =
        serde_json::from_str(include_str!("fixtures/legacy_combat_history.json")).unwrap();
    let turn = saved["log"].as_array_mut().unwrap().last_mut().unwrap()["rounds"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["turns"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    turn["actions"].as_array_mut().unwrap().pop();
    saved["log_index"] = json!(saved["log_index"].as_u64().unwrap() - 1);
    let full = server::replay::replay(serde_json::from_value(saved).unwrap(), None).unwrap();
    let next = play(
        full,
        0,
        json!({"Movement":{"Move":{"destination":"E5","units":[3]}}}),
    );
    assert_eq!(
        data(&next)["events"][0]["event_type"]["CombatRoundStart"]["combat"]["asynchronous_preparation"],
        json!(true)
    );

    // A branch earlier than the original history's switch cannot inherit it.
    let earlier = historical_replay(167);
    let next = play(
        earlier,
        1,
        json!({"Movement":{"Move":{"destination":"F7","units":[1]}}}),
    );
    assert_eq!(
        data(&next)["events"][0]["event_type"]["CombatRoundStart"]["combat"]["asynchronous_preparation"],
        json!(true)
    );
}

#[test]
fn undo_redo_preserves_history_boundary_but_replacing_future_clamps_it() {
    let mut g = server::game_api::init(2, "combat-undo-boundary".into(), Default::default());
    g.asynchronous_combat_from = Some(100);
    let player = g.active_player();
    let action = json!({"Playing":{"Advance":{"advance":"Storage","payment":{"food":2}}}});
    let paid = play(g, player, action.clone());
    let undone = play(paid, player, json!("Undo"));
    assert_eq!(undone.asynchronous_combat_from, Some(100));
    let redone = play(undone, player, json!("Redo"));
    assert_eq!(redone.asynchronous_combat_from, Some(100));
    let undone = play(redone, player, json!("Undo"));
    let next = server::game_api::log_length(&undone) + 1;
    let branched = play(undone, player, action);
    assert_eq!(branched.asynchronous_combat_from, Some(next));
    let undone = play(branched, player, json!("Undo"));
    assert_eq!(undone.asynchronous_combat_from, Some(next));
    let redone = play(undone, player, json!("Redo"));
    assert_eq!(redone.asynchronous_combat_from, Some(next));
}

#[test]
fn undo_redo_of_a_branched_battle_keeps_new_preparation() {
    let mut v = fixture("tactics_cards/peltasts");
    v["asynchronous_combat_from"] = json!(100);
    for p in [0, 1] {
        v["players"][p]["advances"]
            .as_array_mut()
            .unwrap()
            .push(json!("SteelWeapons"));
    }
    let action = json!({"Movement":{"Move":{"units":[0,1,2],"destination":"C1","payment":{}}}});
    let legacy = play(load(v), 0, action.clone());
    assert!(
        data(&legacy)["events"][0]["event_type"]
            .get("CombatStart")
            .is_some()
    );
    let undone = play(legacy, 0, json!("Undo"));
    let branch = play(undone, 0, action);
    let expected = data(&branch);
    assert_eq!(
        expected["events"][0]["event_type"]["CombatRoundStart"]["combat"]["asynchronous_preparation"],
        json!(true)
    );
    let undone = play(branch, 0, json!("Undo"));
    let redone = play(undone, 0, json!("Redo"));
    assert_eq!(data(&redone)["events"], expected["events"]);
    assert_eq!(
        data(&redone)["asynchronous_combat_from"],
        expected["asynchronous_combat_from"]
    );
}
