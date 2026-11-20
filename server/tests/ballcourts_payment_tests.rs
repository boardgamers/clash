//! Great Arena applies to Ballcourts through the ordinary payment pipeline.
use serde_json::{Value, json};
use server::{
    action::{Action, try_execute_action},
    cache::Cache,
    game::{Game, GameContext},
    game_data::GameData,
    web_view,
};

fn fixture(arena: bool, mood: u8, culture: u8) -> Game {
    let mut data: Value = serde_json::from_str(include_str!(
        "test_games/advances/collect_free_economy.json"
    ))
    .unwrap();
    let fresh = server::game_api::init(2, "ballcourts-npcs".into(), Default::default());
    let npcs = serde_json::to_value(fresh.cloned_data()).unwrap();
    data["players"]
        .as_array_mut()
        .unwrap()
        .extend(npcs["players"].as_array().unwrap().iter().skip(2).cloned());
    let p = &mut data["players"][0];
    p["civilization"] = json!("Maya");
    p["advances"] = json!(["Farming", "Mining", "Arts", "Sports"]);
    p["resources"] = json!({"food":5,"wood":5,"ore":5,"mood_tokens":mood,"culture_tokens":culture});
    p["resource_limit"]["food"] = json!(7);
    p["cities"][0]["mood_state"] = json!("Neutral");
    if arena {
        p["cities"][1]["city_pieces"]["wonders"] = json!(["Colosseum"]);
    }
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}
fn params(game: &Game, recruit: bool) -> Value {
    if recruit {
        return json!({"kind":"recruit","city":"A1","units":{"infantry":2},"replaced":[],"ballcourts":true});
    }
    let view = web_view::view(game, Some(0));
    let choices = view["cities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["position"] == "A1")
        .unwrap()["choices"]
        .as_array()
        .unwrap();
    let first = &choices[0];
    let second = choices
        .iter()
        .find(|c| c["position"] != first["position"])
        .unwrap();
    json!({"kind":"collect","city":"A1","variant":"Collect","ballcourts":true,"selections":[
        {"position":first["position"],"pile":first["pile"],"times":1},
        {"position":second["position"],"pile":second["pile"],"times":1}]})
}
fn play(game: Game, action: Value) -> Game {
    try_execute_action(game, serde_json::from_value::<Action>(action).unwrap(), 0).unwrap()
}
#[test]
fn arena_culture_pays_exactly_once_for_collect_and_recruit() {
    for recruit in [true, false] {
        let game = fixture(true, 0, 3);
        let query = params(&game, recruit);
        let offer = web_view::query(&game, 0, query.clone()).unwrap();
        let before = game.player(0).resources.clone();
        let actions = game.actions_left;
        let units = game.player(0).units.len();
        let after = play(game, offer["action"].clone());
        assert!(after.current_event_handler().is_none());
        assert_eq!(after.player(0).resources.culture_tokens, 2);
        assert_eq!(after.player(0).resources.mood_tokens, 0);
        assert_eq!(after.actions_left, actions - 1);
        assert!(after.player(0).cities[0].is_activated());
        if recruit {
            assert_eq!(after.player(0).units.len(), units + 2);
            assert_eq!(after.player(0).resources.food, before.food - 2);
            assert_eq!(after.player(0).resources.ore, before.ore - 2);
        } else {
            let mut expected = before;
            expected.culture_tokens -= 1;
            expected += serde_json::from_value(offer["total"].clone()).unwrap();
            assert_eq!(after.player(0).resources, expected);
        }
    }
}
#[test]
fn no_arena_or_no_tokens_rejects_bonus_in_preview_and_engine() {
    for recruit in [false, true] {
        let valid = fixture(true, 0, 3);
        let action = web_view::query(&valid, 0, params(&valid, recruit)).unwrap()["action"].clone();
        for (arena, culture) in [(false, 3), (true, 0)] {
            let game = fixture(arena, 0, culture);
            assert!(web_view::query(&game, 0, params(&game, recruit)).is_err());
            assert!(
                try_execute_action(game, serde_json::from_value(action.clone()).unwrap(), 0)
                    .is_err()
            );
        }
    }
}
#[test]
fn arena_collect_automatically_prefers_mood_without_a_decision() {
    let game = fixture(true, 2, 3);
    let offer = web_view::query(&game, 0, params(&game, false)).unwrap();
    let after = play(game, offer["action"].clone());
    assert!(after.current_event_handler().is_none());
    assert_eq!(after.player(0).resources.mood_tokens, 1);
    assert_eq!(after.player(0).resources.culture_tokens, 3);
}

#[test]
fn arena_recruit_allows_either_token_but_not_free_or_excess_capacity() {
    for token in ["mood_tokens", "culture_tokens"] {
        let game = fixture(true, 2, 3);
        let mut query = params(&game, true);
        let mut payment = json!({"food":2,"ore":2});
        payment[token] = json!(1);
        query["payment"] = payment;
        let offer = web_view::query(&game, 0, query).unwrap();
        let after = play(game, offer["action"].clone());
        assert_eq!(
            after.player(0).resources.mood_tokens,
            if token == "mood_tokens" { 1 } else { 2 }
        );
        assert_eq!(
            after.player(0).resources.culture_tokens,
            if token == "culture_tokens" { 2 } else { 3 }
        );
    }
    let game = fixture(true, 2, 3);
    let mut query = params(&game, true);
    query["payment"] = json!({"food":2,"ore":2});
    assert!(web_view::query(&game, 0, query.clone()).is_err());
    query.as_object_mut().unwrap().remove("payment");
    query["units"]["infantry"] = json!(3);
    assert!(web_view::query(&game, 0, query).is_err());
}

#[test]
fn arena_does_not_bypass_angry_cities_tile_limits_or_activation() {
    for recruit in [false, true] {
        let mut game = fixture(true, 2, 3);
        let query = params(&game, recruit);
        game.player_mut(0).cities[0].mood_state = server::city::MoodState::Angry;
        assert!(web_view::query(&game, 0, query).is_err());
    }
    let mut game = fixture(true, 2, 3);
    let query = params(&game, false);
    game.player_mut(0).cities[0].angry_activation = true;
    assert!(web_view::query(&game, 0, query).is_err());
    let game = fixture(true, 2, 3);
    let mut query = params(&game, false);
    query["selections"][0]["times"] = json!(2);
    assert!(web_view::query(&game, 0, query).is_err());
}

#[test]
fn free_economy_and_ballcourts_must_pay_both_fees() {
    for culture in [1, 2] {
        let mut data = serde_json::to_value(fixture(true, 0, culture).cloned_data()).unwrap();
        data["players"][0]["advances"]
            .as_array_mut()
            .unwrap()
            .extend([json!("Voting"), json!("FreeEconomy")]);
        let data: GameData = serde_json::from_value(data).unwrap();
        let cache = Cache::new(&data.options);
        let game = Game::from_data(data, cache, GameContext::Play);
        let mut query = params(&game, false);
        query["variant"] = json!({"Special":{"Modifier":"FreeEconomyCollect"}});
        let result = web_view::query(&game, 0, query);
        if culture == 1 {
            assert!(result.is_err());
        } else {
            let actions = game.actions_left;
            let pending = play(game, result.unwrap()["action"].clone());
            let after = play(
                pending,
                json!({"Response":{"Payment":[{"culture_tokens":1}]}}),
            );
            assert_eq!(after.player(0).resources.culture_tokens, 0);
            assert_eq!(after.actions_left, actions);
            assert_eq!(after.player(0).cities[0].activations, 1);
        }
    }
}

#[test]
fn free_economy_reserves_ballcourts_with_mixed_tokens_and_no_extra_response() {
    for (mood, culture, payment) in [
        (2, 0, json!({"mood_tokens":1})),
        (1, 1, json!({"mood_tokens":1})),
        (1, 1, json!({"culture_tokens":1})),
    ] {
        let mut data = serde_json::to_value(fixture(true, mood, culture).cloned_data()).unwrap();
        data["players"][0]["advances"]
            .as_array_mut()
            .unwrap()
            .extend([json!("Voting"), json!("FreeEconomy")]);
        let data: GameData = serde_json::from_value(data).unwrap();
        let cache = Cache::new(&data.options);
        let game = Game::from_data(data, cache, GameContext::Play);
        let actions = game.actions_left;
        let mut query = params(&game, false);
        query["variant"] = json!({"Special":{"Modifier":"FreeEconomyCollect"}});
        let offer = web_view::query(&game, 0, query).unwrap();
        let pending = play(game, offer["action"].clone());
        assert!(
            pending.current_event_handler().is_some(),
            "existing Free Economy payment"
        );
        let after = play(pending, json!({"Response":{"Payment":[payment]}}));
        assert!(
            after.current_event_handler().is_none(),
            "no new Ballcourts response"
        );
        assert_eq!(after.player(0).resources.mood_tokens, 0);
        assert_eq!(after.player(0).resources.culture_tokens, 0);
        assert_eq!(after.actions_left, actions);
        assert_eq!(after.player(0).cities[0].activations, 1);
    }
}
