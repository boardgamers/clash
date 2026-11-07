use common::JsonTest;
use serde_json::json;
use server::action::Action;
use server::advance::Advance;
use server::content::persistent_events::{EventResponse, PersistentEventRequest};
use server::game::Game;
use server::game_api;
use server::map::Terrain;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::{Unit, UnitType};
use std::panic::{AssertUnwindSafe, catch_unwind};

mod common;

fn setup(civilization: &str, irrigation: bool, resources: ResourcePile) -> Game {
    let base = JsonTest::child("incidents", "great_persons").load_game("great_explorer");
    let mut data = serde_json::to_value(base.cloned_data()).unwrap();
    data["players"][1]["civilization"] = json!(civilization);
    data["players"][1]["action_cards"] = json!([118]);
    data["players"][1]["resources"] = json!(resources);
    if irrigation {
        data["players"][1]["advances"]
            .as_array_mut()
            .unwrap()
            .push(json!("Irrigation"));
    }
    Game::from_data(
        serde_json::from_value(data).unwrap(),
        base.cache,
        base.context,
    )
}

fn respond(game: Game, response: EventResponse) -> Game {
    game_api::execute(game, Action::Response(response), 1)
}

fn positions(game: Game, choices: &[&str]) -> Game {
    respond(
        game,
        EventResponse::SelectPositions(choices.iter().map(|p| Position::from_offset(p)).collect()),
    )
}

fn reveal(game: Game) -> Game {
    let game = game_api::execute(game, Action::Playing(PlayingAction::ActionCard(118)), 1);
    assert!(
        game.player(1).has_advance(Advance::Fishing),
        "free advance retained"
    );
    let game = positions(game, &["B6"]);
    if game
        .current_event_handler()
        .is_some_and(|h| matches!(h.request, PersistentEventRequest::ExploreResolution))
    {
        respond(game, EventResponse::ExploreResolution(0))
    } else {
        game
    }
}

fn restore(game: &Game) -> Game {
    Game::from_data(
        serde_json::from_str(&common::to_json(game)).unwrap(),
        game.cache.clone(),
        game.context.clone(),
    )
}

fn choices(game: &Game) -> Vec<Position> {
    let PersistentEventRequest::SelectPositions(request) = &game
        .current_event_handler()
        .expect("placement request")
        .request
    else {
        panic!("Expected positions");
    };
    assert_eq!(request.description, "Place a city for 2 food");
    let view = server::web_view::view(game, Some(1));
    let from_view: Vec<_> = view["decision"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| Position::from_offset(o["position"].as_str().unwrap()))
        .collect();
    assert_eq!(from_view, request.choices, "viewer uses engine eligibility");
    request.choices.clone()
}

fn pay(game: Game, payment: ResourcePile) -> Game {
    respond(game, EventResponse::Payment(vec![payment]))
}

#[test]
fn egypt_flood_plains_can_place_on_barren_without_consuming_a_settler() {
    let game = reveal(setup("Egypt", true, ResourcePile::food(2)));
    let pos = Position::from_offset("B7");
    assert!(choices(&game).contains(&pos));
    assert!(Unit::new(1, pos, UnitType::Settler, 99).can_found_city(&game));
    let units = game.player(1).units.len();
    let game = positions(restore(&game), &["B7"]);
    assert_eq!(game.player(1).cities.len(), 1, "no city before paying");
    let PersistentEventRequest::Payment(requests) = &game.current_event_handler().unwrap().request
    else {
        panic!("Expected payment");
    };
    assert!(!requests[0].optional);
    let game = pay(restore(&game), ResourcePile::food(2));
    assert!(game.player(1).try_get_city(pos).is_some());
    assert_eq!(game.player(1).units.len(), units);
    assert_eq!(game.player(1).resources.food, 0);
    assert!(game.current_event_handler().is_none());
}

#[test]
fn barren_requires_egypt_and_irrigation() {
    for (civ, irrigation) in [("Egypt", false), ("Rome", true), ("Rome", false)] {
        let game = reveal(setup(civ, irrigation, ResourcePile::food(2)));
        let pos = Position::from_offset("B7");
        assert!(!choices(&game).contains(&pos));
        assert!(!Unit::new(1, pos, UnitType::Settler, 99).can_found_city(&game));
        assert!(catch_unwind(AssertUnwindSafe(|| positions(game, &["B7"]))).is_err());
    }
}

#[test]
fn unaffordable_placement_is_not_offered_and_keeps_exploration_and_advance() {
    let game = reveal(setup("Egypt", true, ResourcePile::gold(1)));
    assert!(game.current_event_handler().is_none());
    assert_eq!(
        game.player(1).resources,
        ResourcePile::gold(1) + ResourcePile::mood_tokens(1)
    );
    assert_eq!(game.player(1).cities.len(), 1);
    assert_eq!(
        game.map.get(Position::from_offset("B7")),
        Some(&Terrain::Barren)
    );
    let log = common::to_json(&game);
    assert!(log.contains("Player cannot afford to build a city"));
    assert!(!log.contains("Decided to build a city"));
    assert!(server::web_view::view(&game, Some(1))["decision"].is_null());
}

#[test]
fn exact_food_and_gold_substitutions_are_required() {
    for cost in [
        ResourcePile::food(2),
        ResourcePile::food(1) + ResourcePile::gold(1),
        ResourcePile::gold(2),
    ] {
        let game = positions(reveal(setup("Egypt", true, cost.clone())), &["B7"]);
        for invalid in [
            ResourcePile::empty(),
            ResourcePile::food(1),
            ResourcePile::gold(1),
            ResourcePile::food(3),
            ResourcePile::wood(2),
        ] {
            assert!(catch_unwind(AssertUnwindSafe(|| pay(game.clone(), invalid))).is_err());
        }
        let game = pay(game, cost);
        assert_eq!(
            game.player(1).resources,
            ResourcePile::mood_tokens(1),
            "only the free advance bonus remains"
        );
        assert_eq!(game.player(1).cities.len(), 2);
    }
}

#[test]
fn decline_placement_or_exploration_never_charges() {
    let game = positions(reveal(setup("Egypt", true, ResourcePile::food(2))), &[]);
    assert_eq!(game.player(1).resources.food, 2);
    assert_eq!(game.player(1).cities.len(), 1);
    assert!(game.current_event_handler().is_none());
    let game = game_api::execute(
        setup("Egypt", true, ResourcePile::food(2)),
        Action::Playing(PlayingAction::ActionCard(118)),
        1,
    );
    let game = positions(game, &[]);
    assert!(game.player(1).has_advance(Advance::Fishing));
    assert_eq!(game.player(1).resources.food, 2);
    assert!(game.current_event_handler().is_none());
}

#[test]
fn no_city_pieces_suppresses_placement_including_destroyed_supply() {
    let mut game = setup("Egypt", true, ResourcePile::food(2));
    game.player_mut(1).destroyed_structures.cities = 6;
    assert!(!game.player(1).is_city_available());
    let game = reveal(game);
    assert!(game.current_event_handler().is_none());
    assert_eq!(game.player(1).cities.len(), 1);
    assert_eq!(game.player(1).resources.food, 2);
    assert!(common::to_json(&game).contains("You have reached your city limit"));
}

#[test]
fn water_exhausted_and_outside_region_remain_ineligible() {
    let mut game = setup("Egypt", true, ResourcePile::food(2));
    let block = game
        .map
        .unexplored_blocks
        .iter_mut()
        .find(|b| b.position.top_tile == Position::from_offset("B6"))
        .unwrap();
    block.block = server::map::Block::new([
        Terrain::Fertile,
        Terrain::Water,
        Terrain::Exhausted(Box::new(Terrain::Fertile)),
        Terrain::Barren,
    ]);
    let game = reveal(game);
    let eligible = choices(&game);
    assert_eq!(eligible.len(), 2);
    for pos in ["A7", "C7", "D7"] {
        let pos = Position::from_offset(pos);
        assert!(!eligible.contains(&pos));
        if pos != Position::from_offset("D7") {
            assert!(!Unit::new(1, pos, UnitType::Settler, 99).can_found_city(&game));
        }
    }
    assert!(catch_unwind(AssertUnwindSafe(|| positions(game, &["D7"]))).is_err());
}

#[test]
fn legacy_unaffordable_position_finishes_without_claiming_to_build() {
    let mut game = reveal(setup("Egypt", true, ResourcePile::food(2)));
    game.player_mut(1).resources = ResourcePile::gold(1);
    let game = positions(restore(&game), &["B6"]);
    assert!(game.current_event_handler().is_none());
    assert_eq!(game.player(1).resources, ResourcePile::gold(1));
    assert_eq!(game.player(1).cities.len(), 1);
    let log = common::to_json(&game);
    assert!(log.contains("Player cannot afford to build a city"));
    assert!(!log.contains("Decided to build a city"));
}

#[test]
fn legacy_invalid_payment_refunds_and_preserves_history() {
    for supply in [false, true] {
        let mut game = positions(reveal(setup("Egypt", true, ResourcePile::food(2))), &["B7"]);
        if supply {
            game.player_mut(1).destroyed_structures.cities = 6;
        } else {
            game.map
                .tiles
                .insert(Position::from_offset("B7"), Terrain::Water);
        }
        let game = pay(restore(&game), ResourcePile::food(2));
        assert_eq!(game.player(1).resources.food, 2);
        assert_eq!(game.player(1).cities.len(), 1);
        assert!(game.current_event_handler().is_none());
        assert!(
            common::to_json(&game).contains("Decided to build a city B7"),
            "previous history is not rewritten"
        );
    }
}
