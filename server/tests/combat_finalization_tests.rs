//! Synthetic fixed-die regressions: no production save data.
use serde_json::{Value, json};
use server::{
    board_history,
    cache::Cache,
    game::{Game, GameContext},
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
    board_history::execute(g, serde_json::from_value(a).unwrap(), p).unwrap()
}
fn response(g: Game, r: Value) -> Game {
    let p = g.active_player();
    play(load(data(&g)), p, json!({"Response":r}))
}
fn end(g: &Game) -> Value {
    data(g)["events"].as_array().unwrap().last().unwrap()["event_type"]["CombatRoundEnd"].clone()
}
fn request(g: &Game) -> Value {
    serde_json::to_value(&g.current_event_handler().unwrap().request).unwrap()
}
fn last_report(g: &Game, player: usize) -> Value {
    fn visit(v: &Value, player: usize, result: &mut Value) {
        match v {
            Value::Array(values) => {
                for v in values {
                    visit(v, player, result);
                }
            }
            Value::Object(fields) => {
                if v["player"] == player && fields.contains_key("CombatHitReport") {
                    *result = v["CombatHitReport"].clone();
                }
                for v in fields.values() {
                    visit(v, player, result);
                }
            }
            _ => (),
        }
    }
    let mut result = Value::Null;
    visit(&data(g)["log"], player, &mut result);
    result
}

fn setup(legacy: bool) -> Game {
    let mut v: Value =
        serde_json::from_str(include_str!("test_games/combat/retreat.json")).unwrap();
    v["players"][0]["civilization"] = json!("India");
    v["players"][1]["civilization"] = json!("Huns");
    for p in 0..2 {
        v["players"][p]["advances"] = json!(["Farming", "Mining", "Tactics"]);
        v["players"][p]["units"] = json!([
            {"id":0,"position":if p==0 {"C2"} else {"C1"},"unit_type":if p==0 {json!("Infantry")} else {json!({"Leader":"Attila"})}},
            {"id":1,"position":if p==0 {"C2"} else {"C1"},"unit_type":if p==0 {"Infantry"} else {"Cavalry"}}
        ]);
        v["players"][p]["next_unit_id"] = json!(2);
    }
    v["players"][1]["advances"]
        .as_array_mut()
        .unwrap()
        .push(json!("SteelWeapons"));
    v["players"][0]["action_cards"] = json!([31]); // Flanking
    v["players"][1]["action_cards"] = json!([9]); // High Ground
    v["dice_roll_outcomes"] = json!([10, 2, 9, 3, 9]); // 5 Inf, 2 Elephant, 5 Inf; 2 Cavalry, 6 Inf
    if legacy {
        v["finalized_combat_from"] = json!(10000);
    }
    load(v)
}
fn horse_master(legacy: bool) -> Game {
    let g = play(
        setup(legacy),
        0,
        json!({"Movement":{"Move":{"units":[0,1],"destination":"C1","payment":{}}}}),
    );
    let g = response(g, json!({"SelectHandCards":[{"ActionCard":31}]}));
    let g = response(g, json!({"Payment":[{"ore":1}]}));
    let g = response(g, json!({"SelectHandCards":[{"ActionCard":9}]}));
    assert!(
        request(&g)["BoolRequest"]
            .as_str()
            .unwrap()
            .starts_with("Horse Master")
    );
    assert_eq!(end(&g)["attacker"]["combat_value"], 12);
    assert_eq!(end(&g)["defender"]["combat_value"], 12);
    g
}
#[test]
fn defender_conversion_precedes_both_casualties_and_survivors_continue() {
    let g = horse_master(false);
    assert_eq!(
        g.player(0).units.len(),
        2,
        "neither attacker may die before defender's conversion"
    );
    assert_eq!(g.player(1).units.len(), 2);
    assert_eq!(end(&g)["phase"], "RoundEndModifiers");
    let g = response(g, json!({"Bool":true}));
    assert_eq!(end(&g)["defender"]["combat_value"], 8);
    assert_eq!(end(&g)["attacker"]["opponent_hit_cancels"], 1);
    assert_eq!(g.player(0).units.len(), 1);
    assert_eq!(request(&g)["SelectUnits"]["needed"]["start"], 1);
    let g = response(g, json!({"SelectUnits":[1]}));
    assert_eq!(g.player(1).units.len(), 1);
    assert_eq!(end(&g)["phase"], "Retreat");
    let mut v = data(&g);
    v["dice_roll_outcomes"] = json!([3, 3, 3, 3]);
    let g = response(load(v), json!({"Bool":false}));
    assert_eq!(end(&g)["combat"]["stats"]["round"], 2);
    assert_eq!(end(&g)["phase"], "Retreat");
    assert_eq!(g.player(0).units.len(), 1);
    assert_eq!(g.player(1).units.len(), 1);
}
#[test]
fn decline_keeps_original_values_and_draw() {
    let g = response(horse_master(false), json!({"Bool":false}));
    assert!(g.player(0).units.is_empty());
    assert!(g.player(1).units.is_empty());
}
#[test]
fn legacy_pending_conversion_is_not_repaired_or_reinterpreted() {
    let g = horse_master(true);
    // Baseline bug reproduced using the preserved old execution mode. Changing
    // this pending battle to the new phase cannot resurrect already killed units.
    assert!(g.player(0).units.is_empty());
    assert_eq!(end(&g)["phase"], json!(null)); // Default is omitted
    let g = load(data(&g));
    assert!(g.player(0).units.is_empty());
    let accepted = response(load(data(&g)), json!({"Bool":true}));
    assert!(
        accepted.player(0).units.is_empty(),
        "legacy acceptance must not silently resurrect attackers"
    );
    let g = response(g, json!({"Bool":false}));
    assert!(g.player(0).units.is_empty());
    assert!(g.player(1).units.is_empty());
}

#[test]
fn attacker_conversion_and_defender_paid_cancel_both_resolve_before_any_losses() {
    let mut v = data(&setup(false));
    v["players"][0]["civilization"] = json!("Huns");
    v["players"][1]["civilization"] = json!("China");
    v["players"][0]["units"][0]["unit_type"] = json!({"Leader":"Attila"});
    v["players"][0]["units"][1]["unit_type"] = json!("Cavalry");
    v["players"][1]["units"][0]["unit_type"] = json!({"Leader":"Qin"});
    v["players"][1]["units"][1]["unit_type"] = json!("Infantry");
    v["players"][0]["advances"] = json!(["Tactics", "SteelWeapons"]);
    v["players"][1]["advances"] = json!(["Tactics"]);
    for p in 0..2 {
        v["players"][p]["action_cards"] = json!([]);
    }
    v["dice_roll_outcomes"] = json!([10, 9, 10, 2]);
    let g = play(
        load(v),
        0,
        json!({"Movement":{"Move":{"units":[0,1],"destination":"C1","payment":{}}}}),
    );
    let g = response(g, json!({"Payment":[{"ore":1}]}));
    assert!(
        request(&g)["BoolRequest"]
            .as_str()
            .unwrap()
            .starts_with("Horse Master")
    );
    assert_eq!(end(&g)["attacker"]["combat_value"], 12);
    assert_eq!(end(&g)["defender"]["combat_value"], 12);
    let g = response(g, json!({"Bool":true}));
    assert_eq!(g.active_player(), 1);
    assert!(request(&g).get("Payment").is_some());
    assert_eq!(g.player(0).units.len(), 2);
    assert_eq!(g.player(1).units.len(), 2);
    let g = response(g, json!({"Payment":[{"culture_tokens":2}]}));
    assert_eq!(end(&g)["attacker"]["combat_value"], 8);
    assert_eq!(end(&g)["attacker"]["opponent_hit_cancels"], 1);
    assert_eq!(
        last_report(&g, 0),
        json!({"combat_value":8,"hit_cancels":1,"extra_hits":0,"casualties":0})
    );
    assert_eq!(
        last_report(&g, 1),
        json!({"combat_value":12,"hit_cancels":1,"extra_hits":0,"casualties":1})
    );
    assert_eq!(request(&g)["SelectUnits"]["needed"]["start"], 1);
    let g = response(g, json!({"SelectUnits":[1]}));
    assert_eq!(g.player(0).units.len(), 1);
    assert_eq!(g.player(1).units.len(), 2);
    assert_eq!(end(&g)["phase"], "Retreat");
}

#[test]
fn later_round_activated_cavalry_does_not_offer_conversion() {
    let mut g = setup(false);
    g.dice_roll_outcomes = vec![10, 2, 3, 3, 3];
    let g = play(
        g,
        0,
        json!({"Movement":{"Move":{"units":[0,1],"destination":"C1","payment":{}}}}),
    );
    let g = response(g, json!({"SelectHandCards":[{"ActionCard":31}]}));
    let g = response(g, json!({"Payment":[{"ore":1}]}));
    let g = response(g, json!({"SelectHandCards":[{"ActionCard":9}]}));
    assert!(
        request(&g)["BoolRequest"]
            .as_str()
            .unwrap()
            .starts_with("Horse Master")
    );
    let g = response(g, json!({"Bool":true}));
    assert_eq!(end(&g)["phase"], "Retreat");
    assert_eq!(g.player(0).units.len(), 1);
    assert_eq!(g.player(1).units.len(), 2); // Attila AND cavalry still present.
    let mut v = data(&g);
    v["dice_roll_outcomes"] = json!([10, 2, 3]);
    let g = response(load(v), json!({"Bool":false}));
    // Round two's activated cavalry kills the last attacker without another offer.
    assert!(g.player(0).units.is_empty());
    assert_eq!(g.player(1).units.len(), 2);
    assert!(g.events.is_empty());
}
