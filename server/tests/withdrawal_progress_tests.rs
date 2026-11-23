//! Fixed synthetic fixtures; no production saves or time-expiration inference.
use serde_json::{Value, json};
use server::{
    cache::Cache,
    game::{Game, GameContext},
    game_api_wrapper,
    game_data::GameData,
};

fn load(v: Value) -> Game {
    let d: GameData = serde_json::from_value(v).unwrap();
    let cache = Cache::new(&d.options);
    Game::from_data(d, cache, GameContext::Play)
}
fn data(g: &Game) -> Value {
    serde_json::to_value(g.cloned_data()).unwrap()
}
fn play(g: Game, p: usize, a: Value) -> Game {
    load(
        serde_json::from_str(&game_api_wrapper::execute_move(
            data(&g).to_string(),
            a.to_string(),
            p,
        ))
        .unwrap(),
    )
}
fn drop(g: Game, p: usize) -> Game {
    load(
        serde_json::from_str(&async_std::task::block_on(game_api_wrapper::drop_player(
            data(&g).to_string(),
            p,
        )))
        .unwrap(),
    )
}
fn fixture(raw: &str) -> Value {
    let mut v: Value = serde_json::from_str(raw).unwrap();
    let mut third = v["players"][0].clone();
    third["id"] = json!(2);
    third["civilization"] = json!("Persia");
    third["units"] = json!([]);
    third["cities"] = json!([{"position":"B2","mood_state":"Happy"}]);
    third["objective_cards"] = json!([]);
    third["action_cards"] = json!([]);
    let players = v["players"].as_array_mut().unwrap();
    players.insert(2, third);
    for (index, player) in players.iter_mut().enumerate() {
        player["id"] = json!(index);
    }
    v
}
fn battle() -> Game {
    let mut v = fixture(include_str!("test_games/tactics_cards/peltasts.json"));
    for p in 0..2 {
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
fn defender(g: Game) -> Game {
    let g = play(g, 0, json!({"Response":{"Payment":[{}]}}));
    play(g, 0, json!({"Response":{"SelectHandCards":[]}}))
}
#[test]
fn dropped_defender_declines_steel_and_tactics_but_keeps_passive_board() {
    let g = defender(battle());
    assert_eq!(g.active_player(), 1);
    let ore = g.player(1).resources.ore;
    let cards = g.player(1).action_cards.clone();
    let clocks = server::timing::time_increments(&g);
    let g = drop(g, 1);
    assert_eq!(g.player(1).resources.ore, ore);
    assert_eq!(g.player(1).action_cards, cards);
    assert!(!g.player(1).cities.is_empty());
    assert_eq!(server::timing::time_increments(&g), clocks);
    assert_ne!(g.active_player(), 1);
    assert_ne!(g.state, server::game::GameState::Finished);
    let saved = data(&g);
    assert_eq!(data(&drop(g, 1)), saved);
}
#[test]
fn future_dropped_defender_settles_at_ordinary_exported_move_boundary() {
    let g = battle();
    let before = g.events.clone();
    let g = drop(g, 1);
    assert_eq!(g.events, before); // never submit the living attacker's choices
    let g = defender(g);
    assert_ne!(g.active_player(), 1);
    assert_eq!(server::timing::time_increments(&g)[1], 0);
}
#[test]
fn mandatory_dropped_casualties_are_removed_before_living_retreat() {
    let mut v = fixture(include_str!("test_games/combat/retreat.json"));
    v["players"][0]["units"]
        .as_array_mut()
        .unwrap()
        .retain(|u| u["id"].as_u64().unwrap() < 4);
    for unit in v["players"][0]["units"].as_array_mut().unwrap() {
        unit["unit_type"] = json!("Infantry");
    }
    v["players"][1]["units"] = json!([
        {"id":0,"position":"C1","unit_type":"Cavalry"},
        {"id":1,"position":"C1","unit_type":"Cavalry"},
        {"id":2,"position":"C1","unit_type":"Cavalry"},
        {"id":3,"position":"C1","unit_type":"Infantry"}
    ]);
    let mut g = load(v);
    g.dice_roll_outcomes = vec![4; 8];
    let g = play(
        g,
        0,
        json!({"Movement":{"Move":{"units":[0,1,2,3],"destination":"C1","payment":{}}}}),
    );
    assert_eq!(g.active_player(), 1);
    assert!(matches!(
        g.current_event_handler().unwrap().request,
        server::content::persistent_events::PersistentEventRequest::SelectUnits(_)
    ));
    let g = drop(g, 1);
    assert_eq!(g.player(1).units.len(), 2);
    assert_eq!(g.player(0).units.len(), 2);
    assert_eq!(g.active_player(), 0);
    assert!(matches!(
        g.current_event_handler().unwrap().request,
        server::content::persistent_events::PersistentEventRequest::BoolRequest(_)
    ));
    let saved = data(&g);
    assert_eq!(data(&load(saved.clone())), saved);
}
#[test]
fn pending_status_advance_stays_attributed_to_departed_seat() {
    let mut g = load(fixture(include_str!(
        "test_games/status_phase/free_advance.json"
    )));
    g = play(g, 0, json!({"Playing":"EndTurn"}));
    assert_eq!(g.active_player(), 1);
    let before = g.player(0).advances.clone();
    let g = drop(g, 1);
    assert_eq!(g.player(0).advances, before);
    assert_ne!(g.active_player(), 1);
    assert!(!g.events.is_empty());
}
#[test]
fn withdrawal_after_government_payment_completes_without_paying_twice() {
    let g = load(fixture(include_str!(
        "test_games/status_phase/change_government.json"
    )));
    let g = play(g, 1, json!({"Response":{"SelectPositions":[]}}));
    let g = if g.active_player() == 2 {
        play(g, 2, json!({"Response":{"SelectPositions":[]}}))
    } else {
        g
    };
    let g = play(
        g,
        0,
        json!({"Response":{"Payment":[{"culture_tokens":1,"mood_tokens":1}]}}),
    );
    assert!(matches!(
        g.current_event_handler().unwrap().request,
        server::content::persistent_events::PersistentEventRequest::ChangeGovernment
    ));
    let resources = g.player(0).resources.clone();
    let g = drop(g, 0);
    assert_eq!(g.player(0).resources.mood_tokens, resources.mood_tokens);
    assert_eq!(
        g.player(0).resources.culture_tokens,
        resources.culture_tokens
    );
    assert_ne!(g.active_player(), 0);
}
#[test]
fn withdrawal_during_living_defender_choice_preserves_suspended_turn_owner() {
    let g = defender(battle());
    let events = g.events.clone();
    let g = drop(g, 0);
    assert_eq!(g.current_player_index, 0);
    assert_eq!(g.active_player(), 1);
    assert_eq!(g.events, events);
    let g = play(g, 1, json!({"Response":{"Payment":[{}]}}));
    let g = play(g, 1, json!({"Response":{"SelectHandCards":[]}}));
    assert_ne!(g.active_player(), 0);
    assert!(!g.dropped_players.contains(&g.active_player()));
}
#[test]
fn withdrawal_last_survivor_and_no_survivors_have_terminal_rankings() {
    let g = drop(battle(), 0);
    let g = drop(g, 1);
    assert_eq!(g.state, server::game::GameState::Finished);
    assert!(g.events.is_empty());
    assert_eq!(server::game_api::rankings(&g)[2], 1);
    let g = drop(g, 2);
    assert_eq!(server::game_api::rankings(&g), vec![1, 1, 1]);
}
#[test]
fn captured_city_still_grants_dropped_loser_mandatory_settler() {
    let mut v = fixture(include_str!("test_games/combat/capture_and_raze_city.json"));
    v["players"][1]["cities"]
        .as_array_mut()
        .unwrap()
        .push(json!({"position":"B1","mood_state":"Happy"}));
    let g = play(
        load(v),
        0,
        json!({"Movement":{"Move":{"units":[0,1,2,3],"destination":"C1","payment":{}}}}),
    );
    assert_eq!(g.active_player(), 1);
    assert!(matches!(
        g.current_event_handler().unwrap().request,
        server::content::persistent_events::PersistentEventRequest::SelectPositions(_)
    ));
    let before = g
        .player(1)
        .units
        .iter()
        .filter(|u| u.unit_type == server::unit::UnitType::Settler)
        .count();
    let g = drop(g, 1);
    assert_eq!(
        g.player(1)
            .units
            .iter()
            .filter(|u| u.unit_type == server::unit::UnitType::Settler)
            .count(),
        before + 1
    );
    assert_ne!(g.active_player(), 1);
}
#[test]
fn legacy_advanced_pointer_is_repaired_only_on_authoritative_reconciliation() {
    let g = defender(battle());
    let mut v = data(&g);
    v["dropped_players"] = json!([0]);
    v["current_player_index"] = json!(1);
    let g = load(v);
    assert_eq!(g.current_player_index, 1); // a query/load does not commit a repair
    let events = g.events.clone();
    let g = drop(g, 0); // explicit authorized engine reconciliation, not a viewer read
    assert_eq!(g.current_player_index, 0);
    assert_eq!(g.active_player(), 1);
    assert_eq!(g.events, events);
}
#[test]
fn withdrawal_discards_only_departed_private_plans() {
    let mut v = data(&defender(battle()));
    v["action_plan"] =
        json!({"player":0,"key":"private","actions":[{"Playing":"EndTurn"}],"waiting":true});
    v["early_choices"] = json!([
        {"player":0,"key":"status/private","response":{"Bool":true}},
        {"player":2,"key":"status/live","response":{"Bool":true}}
    ]);
    let g = drop(load(v), 0);
    assert!(g.action_plan.is_none());
    assert_eq!(g.early_choices.len(), 1);
    assert_eq!(g.early_choices[0].player, 2);
    assert_eq!(g.active_player(), 1);
}
