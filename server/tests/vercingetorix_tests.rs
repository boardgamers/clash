use serde_json::{Value, json};
use server::action::{Action, execute_action, try_execute_action};
use server::cache::Cache;
use server::city::{City, MoodState};
use server::content::persistent_events::{EventResponse, PersistentEventRequest};
use server::game::{Game, GameContext, GameState};
use server::game_data::GameData;
use server::leader::Leader;
use server::map::Terrain;
use server::movement::{MoveUnits, MovementAction, MovementRestriction, possible_move_routes};
use server::player::CostTrigger;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::recruit::{Recruit, recruit_cost_without_replaced};
use server::resource_pile::ResourcePile;
use server::unit::{Unit, UnitType, Units};
use server::web_view;

fn city() -> Position {
    Position::from_offset("C3")
}

fn fixture() -> Game {
    let mut data: Value =
        serde_json::from_str(include_str!("test_games/base/recruit.json")).unwrap();
    let p = &mut data["players"][0];
    p["civilization"] = json!("Celts");
    p["advances"] = json!(["Farming", "Mining", "Tactics"]);
    p["available_leaders"] = json!(["Viriatus", "Boudica"]);
    p["cities"] = json!([{
        "position": city(), "mood_state": "Neutral",
        "city_pieces": {"academy": 0, "fortress": 0, "temple": 0}
    }]);
    p["units"] = json!([
        {"id": 0, "position": city(), "unit_type": {"Leader": "Vercingetorix"}},
        {"id": 1, "position": city(), "unit_type": "Infantry"},
        {"id": 2, "position": city(), "unit_type": "Infantry"},
        {"id": 3, "position": city(), "unit_type": "Infantry"}
    ]);
    p["next_unit_id"] = json!(4);
    p["resources"] = json!({"food":7,"wood":7,"ore":7,"ideas":7,"gold":7,
        "mood_tokens":7,"culture_tokens":7});
    p["resource_limit"] = p["resources"].clone();
    data["actions_left"] = json!(3);
    data["players"][1]["units"] = json!([]);
    data["players"][1]["cities"] = json!([{
        "position": "H9", "mood_state": "Neutral", "city_pieces": {}
    }]);
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.map.tiles.clear();
    game.map.unexplored_blocks.clear();
    game.map.tiles.insert(city(), Terrain::Fertile);
    for adjacent in city().neighbors() {
        game.map.tiles.insert(adjacent, Terrain::Fertile);
        for outer in adjacent.neighbors() {
            game.map.tiles.insert(outer, Terrain::Fertile);
        }
    }
    game.map
        .tiles
        .insert(Position::from_offset("H9"), Terrain::Fertile);
    game
}

fn infantry(n: u8) -> Units {
    Units::new(0, n, 0, 0, 0, None)
}

fn quote(game: &Game, units: &Units) -> Result<Value, String> {
    web_view::query(
        game,
        0,
        json!({"kind":"recruit", "city":city(), "units":units, "replaced":[]}),
    )
}

fn recruit(game: Game, units: &Units) -> Game {
    let action = serde_json::from_value(quote(&game, units).unwrap()["action"].clone()).unwrap();
    execute_action(game, action, 0)
}

fn choose(game: Game, positions: &[Position]) -> Game {
    let action = web_view::query(&game, 0, json!({"kind":"decision", "values":positions})).unwrap()
        ["action"]
        .clone();
    execute_action(game, serde_json::from_value(action).unwrap(), 0)
}

fn decision(game: &Game) -> Value {
    web_view::view(game, Some(0))["decision"].clone()
}

fn count_army(game: &Game, pos: Position) -> usize {
    game.player(0)
        .get_units(pos)
        .iter()
        .filter(|u| u.is_army_unit())
        .count()
}

fn add_unit(game: &mut Game, player: usize, pos: Position, unit_type: UnitType) {
    let p = game.player_mut(player);
    p.units
        .push(Unit::new(player, pos, unit_type, p.next_unit_id));
    p.next_unit_id += 1;
}

#[test]
fn full_city_recruits_city_size_and_view_requires_legal_final_placements() {
    let mut game = fixture();
    let [mountain, forest, ..] = city().neighbors()[..] else {
        unreachable!()
    };
    game.map.tiles.insert(mountain, Terrain::Mountain);
    game.map.tiles.insert(forest, Terrain::Forest);
    // The leader's earlier movement must neither prevent nor be cleared by recruitment.
    game.player_mut(0).get_unit_mut(0).movement_restrictions = vec![MovementRestriction::Forest];
    let view = web_view::view(&game, Some(0));
    let offer = view["cityActions"][0]["recruits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["type"] == "Infantry")
        .unwrap();
    assert_eq!(
        offer["reason"],
        Value::Null,
        "the actual city viewer must offer recruitment"
    );
    assert!(
        recruit_cost_without_replaced(
            &game,
            game.player(0),
            &infantry(4),
            city(),
            CostTrigger::NoModifiers
        )
        .is_ok(),
        "native and web previews must agree"
    );
    let resources = game.player(0).resources.clone();
    let mut game = recruit(game, &infantry(4));
    for target in [mountain, forest, mountain, forest] {
        let d = decision(&game);
        assert!(
            d["description"]
                .as_str()
                .unwrap()
                .starts_with("Rapid Recruitment")
        );
        assert_eq!(d["min"], 1);
        assert!(
            d["options"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["position"] == json!(target))
        );
        assert!(web_view::query(&game, 0, json!({"kind":"decision", "values":[]})).is_err());
        game = choose(game, &[target]);
    }
    assert!(game.events.is_empty());
    assert_eq!(game.state, GameState::Playing);
    assert_eq!(
        game.actions_left, 2,
        "placement consumes no movement action"
    );
    assert_eq!(game.player(0).get_city(city()).activations, 1);
    let mut expected_resources = resources;
    expected_resources -= ResourcePile::food(4) + ResourcePile::ore(4);
    assert_eq!(game.player(0).resources, expected_resources);
    assert_eq!(count_army(&game, city()), 4);
    assert_eq!(count_army(&game, mountain), 2);
    assert_eq!(count_army(&game, forest), 2);
    for id in 0..4 {
        assert_eq!(
            game.player(0).get_unit(id).position,
            city(),
            "only new units deploy"
        );
    }
    assert_eq!(
        game.player(0).get_unit(0).movement_restrictions,
        vec![MovementRestriction::Forest]
    );
    for id in 4..8 {
        assert!(game.player(0).get_unit(id).movement_restrictions.is_empty());
    }
}

#[test]
fn terrain_deployment_leaves_units_free_to_move_but_normal_movement_keeps_terrain_rules() {
    for terrain in [Terrain::Mountain, Terrain::Forest] {
        let mut game = fixture();
        let target = city().neighbors()[0];
        let onward = target
            .neighbors()
            .into_iter()
            .find(|p| p.distance(city()) == 2)
            .unwrap();
        game.map.tiles.insert(target, terrain);
        game.map.tiles.insert(onward, Terrain::Mountain);
        let game = choose(recruit(game, &infantry(1)), &[target]);
        assert_eq!(game.state, GameState::Playing);
        assert!(game.player(0).get_unit(4).movement_restrictions.is_empty());
        let routes = possible_move_routes(game.player(0), &game, &[4], target, None).unwrap();
        assert!(routes.iter().any(|r| r.destination == onward));
        let movement = web_view::query(&game, 0, json!({"kind":"movement", "units":[4]})).unwrap();
        assert!(
            movement["destinations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["position"] == json!(onward))
        );
        let game = execute_action(
            game,
            Action::Movement(MovementAction::Move(MoveUnits::new(
                vec![4],
                onward,
                None,
                ResourcePile::empty(),
            ))),
            0,
        );
        assert_eq!(game.player(0).get_unit(4).position, onward);
        assert!(
            game.player(0)
                .get_unit(4)
                .movement_restrictions
                .contains(&MovementRestriction::Mountain)
        );
        let GameState::Movement(movement) = game.state else {
            panic!("ordinary movement must start")
        };
        assert_eq!(movement.moved_units, vec![4]);
        assert_eq!(game.actions_left, 1);
    }
}

#[test]
fn keeping_recruits_is_optional_only_while_final_city_stack_can_fit() {
    let mut game = fixture();
    game.player_mut(0).units.retain(|u| u.id != 3);
    let mut game = recruit(game, &infantry(2));
    assert_eq!(decision(&game)["min"], 0);
    game = choose(game, &[]);
    assert_eq!(decision(&game)["min"], 1);
    assert!(web_view::query(&game, 0, json!({"kind":"decision", "values":[]})).is_err());
    game = choose(game, &[city().neighbors()[0]]);
    assert!(game.events.is_empty());
    assert_eq!(count_army(&game, city()), 4);
    assert_eq!(game.player(0).get_unit(4).position, city());
    assert_eq!(game.player(0).get_unit(5).position, city().neighbors()[0]);
}

#[test]
fn destinations_exclude_enemy_units_cities_water_unexplored_and_full_armies() {
    let mut game = fixture();
    let neighbors = city().neighbors();
    for _ in 0..4 {
        add_unit(&mut game, 0, neighbors[0], UnitType::Infantry);
    }
    add_unit(&mut game, 1, neighbors[1], UnitType::Settler);
    game.player_mut(1).cities.push(City::new(1, neighbors[2]));
    game.map.tiles.insert(neighbors[3], Terrain::Water);
    game.map.tiles.insert(neighbors[4], Terrain::Unexplored);
    // Friendly cities and Settlers are legal; only army units consume stack capacity.
    game.player_mut(0).cities.push(City::new(0, neighbors[5]));
    for _ in 0..3 {
        add_unit(&mut game, 0, neighbors[5], UnitType::Infantry);
    }
    add_unit(&mut game, 0, neighbors[5], UnitType::Settler);
    assert!(quote(&game, &infantry(1)).is_ok());
    assert_eq!(
        quote(&game, &infantry(2)).unwrap_err(),
        "Too many units in stack"
    );
    // A spare city slot keeps placement optional, so its one legal target is shown.
    game.player_mut(0).units.retain(|u| u.id != 3);
    let mut game = recruit(game, &infantry(2));
    let d = decision(&game);
    assert_eq!(d["min"], 0);
    assert_eq!(d["options"].as_array().unwrap().len(), 1);
    assert_eq!(d["options"][0]["position"], json!(neighbors[5]));
    for invalid in neighbors[..5]
        .iter()
        .copied()
        .chain([city(), Position::from_offset("H9")])
    {
        assert!(web_view::query(&game, 0, json!({"kind":"decision", "values":[invalid]})).is_err());
    }
    game = choose(game, &[neighbors[5]]);
    // The destination is now full; the second unit stays in the city, without a dead-end prompt.
    assert!(game.events.is_empty());
    assert_eq!(count_army(&game, neighbors[5]), 4);
    assert_eq!(count_army(&game, city()), 4);
}

#[test]
fn no_recruitment_overflow_when_all_adjacent_destinations_are_blocked() {
    let mut game = fixture();
    for pos in city().neighbors() {
        game.map.tiles.remove(&pos);
    }
    assert_eq!(
        quote(&game, &infantry(1)).unwrap_err(),
        "Too many units in stack"
    );
    let action = Action::Playing(PlayingAction::Recruit(Recruit::new(
        &infantry(1),
        city(),
        ResourcePile::food(1) + ResourcePile::ore(1),
    )));
    assert_eq!(
        try_execute_action(game, action, 0).err().unwrap(),
        "Too many units in stack"
    );
}

#[test]
fn settlers_do_not_consume_stack_capacity_or_force_deployment() {
    let mut game = fixture();
    let target = city().neighbors()[0];
    for _ in 0..4 {
        add_unit(&mut game, 0, target, UnitType::Infantry);
    }
    let game = recruit(game, &Units::new(1, 1, 0, 0, 0, None));
    let d = decision(&game);
    assert_eq!(d["min"], 0);
    assert!(
        d["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["position"] == json!(target))
    );
    let game = choose(game, &[target]);
    assert_eq!(decision(&game)["min"], 1);
    assert!(
        !decision(&game)["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["position"] == json!(target))
    );
    let game = choose(game, &[city().neighbors()[1]]);
    assert!(game.events.is_empty());
    assert_eq!(count_army(&game, target), 4);
    assert!(
        game.player(0)
            .get_units(target)
            .iter()
            .any(|u| u.is_settler())
    );
}

#[test]
fn city_mood_capacity_cost_building_and_activation_rules_remain_unchanged() {
    for (mood, limit) in [
        (MoodState::Neutral, 4),
        (MoodState::Happy, 5),
        (MoodState::Angry, 1),
    ] {
        let mut game = fixture();
        game.player_mut(0).get_city_mut(city()).mood_state = mood;
        assert!(quote(&game, &infantry(limit)).is_ok());
        assert_eq!(
            quote(&game, &infantry(limit + 1)).unwrap_err(),
            "Too many units"
        );
    }
    let mut game = fixture();
    assert_eq!(
        quote(&game, &Units::new(0, 0, 0, 1, 0, None)).unwrap_err(),
        "Missing building: market"
    );
    game.player_mut(0).resources = ResourcePile::empty();
    assert!(quote(&game, &infantry(1)).is_err());
    let mut game = fixture();
    game.player_mut(0).get_city_mut(city()).angry_activation = true;
    assert_eq!(
        quote(&game, &infantry(1)).unwrap_err(),
        "City cannot be activated"
    );
}

#[test]
fn ordinary_recruitment_and_leader_own_recruitment_do_not_gain_adjacent_capacity() {
    for leader_elsewhere in [false, true] {
        let mut game = fixture();
        if leader_elsewhere {
            game.player_mut(0).get_unit_mut(0).position = city().neighbors()[0];
            add_unit(&mut game, 0, city(), UnitType::Infantry);
        } else {
            game.player_mut(0).get_unit_mut(0).unit_type = UnitType::Infantry;
        }
        // Reinitialize listeners as these are setup changes rather than gameplay.
        let game = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
        assert_eq!(
            quote(&game, &infantry(1)).unwrap_err(),
            "Too many units in stack"
        );
        let mut game = game;
        game.player_mut(0).units.retain(|u| u.id != 3);
        let game = recruit(game, &infantry(1));
        assert!(game.events.is_empty());
        assert_eq!(count_army(&game, city()), 4);
    }
    let mut game = fixture();
    game.player_mut(0).units.retain(|u| u.id != 0 && u.id != 3);
    game.player_mut(0)
        .available_leaders
        .push(Leader::Vercingetorix);
    let game = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
    let mut units = infantry(2);
    units.leader = Some(Leader::Vercingetorix);
    assert_eq!(quote(&game, &units).unwrap_err(), "Too many units in stack");
    units.infantry = 1;
    let game = recruit(game, &units);
    assert!(
        game.events.is_empty(),
        "the leader's own recruitment must not trigger his ability"
    );
    assert_eq!(count_army(&game, city()), 4);
    // A later action in the same turn does qualify, including with the city now full.
    let game = recruit(game, &infantry(1));
    assert_eq!(decision(&game)["min"], 1);
    let game = choose(game, &[city().neighbors()[0]]);
    assert!(game.events.is_empty());
}

#[test]
fn pending_rapid_recruitment_round_trips_without_losing_required_choices() {
    let game = recruit(fixture(), &infantry(2));
    let game = choose(game, &[city().neighbors()[0]]);
    let game = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
    let handler = game.current_event_handler().unwrap();
    let PersistentEventRequest::SelectPositions(request) = &handler.request else {
        panic!("pending placement expected")
    };
    assert_eq!(request.needed, 1..=1);
    let game = execute_action(
        game,
        Action::Response(EventResponse::SelectPositions(vec![city().neighbors()[1]])),
        0,
    );
    assert!(game.events.is_empty());
    assert_eq!(count_army(&game, city()), 4);
}
