use serde_json::{Value, json};
use server::action::Action;
use server::cache::Cache;
use server::content::persistent_events::PersistentEventRequest;
use server::game::{Game, GameContext};
use server::game_data::GameData;
use server::playing_actions::PlayingAction;
use server::resource_pile::ResourcePile;
use server::wonder::Wonder;
use server::{game_api, web_view};
use std::time::Instant;

fn wonder_game(resources: ResourcePile) -> Game {
    let mut data: Value =
        serde_json::from_str(include_str!("test_games/wonders/colosseum.json")).unwrap();
    // Engineering activates Architecture: mood may replace any construction
    // resource or culture, in addition to the ordinary gold substitutions.
    data["players"][0]["civilization"] = json!("Egypt");
    data["players"][0]["resources"] = json!(resources);
    let setup =
        server::game_setup::setup_game(&server::game_setup::GameSetupBuilder::new(2).build());
    let setup = serde_json::to_value(setup.data()).unwrap();
    data["players"]
        .as_array_mut()
        .unwrap()
        .extend(setup["players"].as_array().unwrap().iter().skip(2).cloned());
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

#[test]
#[ignore = "manual release-mode performance benchmark; prints the construct-to-payment path"]
fn benchmark_wonder_payment_view() {
    for resources in [
        ResourcePile::new(3, 4, 5, 0, 2, 0, 5),
        ResourcePile::new(7, 7, 7, 5, 7, 7, 7),
    ] {
        let game = wonder_game(resources.clone());
        let start = Instant::now();
        let before = web_view::view(&game, Some(0));
        let availability = start.elapsed();
        assert!(!before["wonderCards"][0]["action"].is_null());
        let start = Instant::now();
        let game = game_api::execute(
            game,
            Action::Playing(PlayingAction::WonderCard(Wonder::Colosseum)),
            0,
        );
        let construct = start.elapsed();
        assert!(matches!(
            game.current_event_handler().unwrap().request,
            PersistentEventRequest::Payment(_)
        ));
        let start = Instant::now();
        let view = web_view::view(&game, Some(0));
        let payment_view = start.elapsed();
        let start = Instant::now();
        let query = web_view::query(&game, 0, json!({"kind":"decision", "values":[], "payments":[view["decision"]["fields"][0]["initial"]]})).unwrap();
        let validation = start.elapsed();
        assert!(!query["action"].is_null());
        eprintln!(
            "stock={resources:?}: availability={availability:?}, construct={construct:?}, payment view={payment_view:?}, query={validation:?}; choices={}",
            view["decision"]["fields"][0]["choices"]
                .as_array()
                .map_or_else(|| "selectors".to_string(), |c| c.len().to_string())
        );
    }
}

#[test]
fn architecture_preserves_all_bounded_wonder_payments() {
    let stock = ResourcePile::new(3, 4, 5, 0, 2, 0, 5);
    let game = game_api::execute(
        wonder_game(stock.clone()),
        Action::Playing(PlayingAction::WonderCard(Wonder::Colosseum)),
        0,
    );
    let view = web_view::view(&game, Some(0));
    let choices: Vec<ResourcePile> =
        serde_json::from_value(view["decision"]["fields"][0]["choices"].clone()).unwrap();
    let mut expected = Vec::new();
    for food_gold in 0..=2 {
        for wood_gold in 0..=2 - food_gold {
            for ore_gold in 0..=2 - food_gold - wood_gold {
                expected.push(ResourcePile::new(
                    3 - food_gold,
                    4 - wood_gold,
                    5 - ore_gold,
                    0,
                    food_gold + wood_gold + ore_gold,
                    0,
                    5,
                ));
            }
        }
    }
    assert_eq!(choices.len(), 10);
    assert!(expected.iter().all(|payment| choices.contains(payment)));
    for payment in choices {
        assert!(
            web_view::query(
                &game,
                0,
                json!({"kind":"decision", "values":[], "payments":[payment]})
            )
            .is_ok()
        );
    }
    assert_eq!(
        game.player(0).resources,
        stock,
        "Opening and quoting payments does not spend anything"
    );
}

#[test]
fn late_game_architecture_keeps_direct_mixed_payments_legal() {
    let stock = ResourcePile::new(7, 7, 7, 5, 7, 7, 7);
    let game = game_api::execute(
        wonder_game(stock.clone()),
        Action::Playing(PlayingAction::WonderCard(Wonder::Colosseum)),
        0,
    );
    let view = web_view::view(&game, Some(0));
    let field = &view["decision"]["fields"][0];
    assert!(
        field["choices"].is_null(),
        "Large spaces use direct amount selectors, not a truncated legal-payment list"
    );
    assert!(
        field["resources"]
            .as_array()
            .unwrap()
            .contains(&json!("mood_tokens"))
    );
    assert_eq!(game.player(0).resources, stock);
    // Mood replaces two culture and two base resources, with four further
    // resources replaced by gold. None of these alternatives may be capped.
    let payment = ResourcePile::new(1, 2, 3, 0, 4, 4, 3);
    let quote = web_view::query(
        &game,
        0,
        json!({"kind":"decision", "values":[], "payments":[payment]}),
    )
    .unwrap();
    let mut underpayment = payment.clone();
    underpayment.mood_tokens -= 1;
    assert!(
        web_view::query(
            &game,
            0,
            json!({"kind":"decision", "values":[], "payments":[underpayment]})
        )
        .is_err()
    );
    let action = serde_json::from_value(quote["action"].clone()).unwrap();
    let paid = game_api::execute(game, action, 0);
    let mut expected = stock;
    expected -= payment;
    assert_eq!(paid.player(0).resources, expected);
    assert!(
        paid.player(0)
            .cities
            .iter()
            .any(|city| city.pieces.wonders.contains(&Wonder::Colosseum))
    );
}
