//! Tributes is city-scoped; a legal variant in one city must not leak to another.
use serde_json::{Value, json};
use server::{
    action::{Action, try_execute_action},
    ai_actions::AiActions,
    cache::Cache,
    game::{Game, GameContext},
    game_data::GameData,
    playing_actions::{PlayingAction, PlayingActionType},
};

fn fixture(large: bool) -> Game {
    let mut data: Value = serde_json::from_str(include_str!(
        "test_games/advances/collect_free_economy.json"
    ))
    .unwrap();
    let fresh = server::game_api::init(2, "tributes-regression".into(), Default::default());
    let npcs = serde_json::to_value(fresh.cloned_data()).unwrap();
    data["players"]
        .as_array_mut()
        .unwrap()
        .extend(npcs["players"].as_array().unwrap().iter().skip(2).cloned());
    let p = &mut data["players"][0];
    p["civilization"] = json!("Aztecs");
    p["advances"] = json!(["Farming", "Mining"]);
    p["resources"] = json!({"food":1,"wood":1,"mood_tokens":6});
    p["resource_limit"]["food"] = json!(7);
    p["units"] = json!([{"id":0,"position":"A1","unit_type":{"Leader":"Acamapichtli"}}]);
    p["next_unit_id"] = json!(1);
    p["recruited_leaders"] = json!(["Acamapichtli"]);
    if large {
        p["cities"][0]["city_pieces"] = json!({"temple":0,"academy":0,"fortress":0});
    }
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::AI)
}

#[test]
fn tributes_ai_selects_variant_per_city_and_every_offer_executes() {
    for large in [false, true] {
        let game = fixture(large);
        let offers = AiActions::new().get_available_actions(&game);
        let collections: Vec<_> = offers
            .iter()
            .flat_map(|(_, actions)| actions)
            .filter_map(|action| {
                if let Action::Playing(PlayingAction::Collect(collect)) = action {
                    Some((action, collect))
                } else {
                    None
                }
            })
            .collect();
        assert!(
            collections
                .iter()
                .any(|(_, c)| c.city_position.to_string() == "A1")
        );
        assert!(
            collections
                .iter()
                .any(|(_, c)| c.city_position.to_string() == "C2")
        );
        for (action, collect) in collections {
            let ordinary = large || collect.city_position.to_string() != "A1";
            assert_eq!(collect.action_type == PlayingActionType::Collect, ordinary);
            let before = game.player(0).resources.mood_tokens;
            let mut result = try_execute_action(game.clone(), action.clone(), 0).unwrap();
            if !ordinary && result.current_event_handler().is_some() {
                let payment =
                    serde_json::from_value(json!({"Response":{"Payment":[{"mood_tokens":2}]}}))
                        .unwrap();
                result = try_execute_action(result, payment, 0).unwrap();
            }
            assert_eq!(
                result.player(0).resources.mood_tokens,
                before - if ordinary { 0 } else { 2 }
            );
            assert_eq!(result.actions_left, game.actions_left - u32::from(ordinary));
        }
    }
}
