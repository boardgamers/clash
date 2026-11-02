use super::*;
use crate::action::{Action, execute_action};
use crate::advance::{Advance, AdvanceAction};
use crate::cache::Cache;
use crate::content::custom_actions::{CustomAction, CustomActionType};
use crate::content::persistent_events::{EventResponse, PersistentEventRequest};
use crate::game::GameContext;
use crate::game_data::GameData;
use crate::playing_actions::PlayingAction;
use crate::unit::Unit;
use crate::web_view;
use serde_json::json;

fn pos(value: &str) -> Position {
    Position::from_offset(value)
}

fn positions(values: &[&str]) -> Vec<Position> {
    values.iter().map(|v| pos(v)).collect()
}

fn fixture() -> Game {
    // Sanitized public board at history cursor 173 of the reported Celts/Maya game.
    // Only terrain, cities, and units are restored; this is NOT a full historical
    // save. Advances/resources/log/deck are synthetic, with player 0 and one action
    // left so the reported Tribal Allies placement can be exercised directly.
    // No names, hidden hands, source log, or history are retained in the fixture.
    let data: GameData = serde_json::from_str(include_str!(
        "../tests/fixtures/tribal_allies_public_board.json"
    ))
    .unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

fn choices(game: &Game) -> Vec<Position> {
    let mut result = possible_barbarians_spawns(game, game.player(0));
    result.sort();
    result
}

fn placement_choices(game: &Game) -> Vec<Position> {
    let PersistentEventRequest::SelectPositions(request) =
        &game.current_event_handler().unwrap().request
    else {
        panic!("expected Barbarian placement decision");
    };
    assert_eq!(request.needed, 1..=1);
    let mut result = request.choices.clone();
    result.sort();
    result
}

fn tribal_allies(game: Game) -> Game {
    execute_action(
        game,
        Action::Playing(PlayingAction::Custom(CustomAction::new(
            CustomActionType::TribalAllies,
            None,
        ))),
        0,
    )
}

fn choose(game: Game, response: EventResponse) -> Game {
    execute_action(game, Action::Response(response), 0)
}

fn place_and_reinforce(game: Game, target: Position) -> Game {
    let game = choose(game, EventResponse::SelectPositions(vec![target]));
    let barbarian = get_barbarians_player(&game);
    assert!(barbarian.cities.iter().any(|c| c.position == target));
    assert_eq!(barbarian.get_units(target).len(), 1);
    assert_eq!(barbarian.get_units(target)[0].unit_type, UnitType::Infantry);
    assert!(placement_choices(&game).contains(&target));
    let game = choose(game, EventResponse::SelectPositions(vec![target]));
    let game = choose(game, EventResponse::SelectUnitType(UnitType::Cavalry));
    assert!(game.events.is_empty());
    assert_eq!(
        get_barbarians_player(&game)
            .get_units(target)
            .iter()
            .map(|u| u.unit_type)
            .collect::<Vec<_>>(),
        vec![UnitType::Infantry, UnitType::Cavalry]
    );
    game
}

#[test]
fn reported_board_allows_npc_neighbors_but_not_other_human_city_neighbors() {
    let game = fixture();
    assert_eq!(game.map.get(pos("B4")), Some(&Terrain::Fertile));
    for (target, settlement) in [("B4", "A5"), ("C3", "D3")] {
        assert_eq!(pos(target).distance(pos(settlement)), 1);
        assert!(!anything_present(&game, pos(target)));
        assert!(
            get_barbarians_player(&game)
                .cities
                .iter()
                .any(|c| c.position == pos(settlement))
        );
    }
    assert_eq!(pos("C5").distance(pos("C6")), 1);
    assert!(!anything_present(&game, pos("C5")));
    assert!(
        game.player(1)
            .cities
            .iter()
            .any(|c| c.position == pos("C6"))
    );
    assert!(
        game.map.tiles.keys().all(|&p| {
            !is_base_barbarian_spawn_pos(&game, p, game.player(0))
                || !cities_in_land_range(&game, |p| p.index == 0, p, 1).is_empty()
                || steps_towards_land_range2_cites(&game, game.player(0), p).is_empty()
        }),
        "this board must exercise the adjacent fallback, not distance-two priority"
    );
    assert_eq!(choices(&game), positions(&["B4", "C2", "C3", "E2"]));
}

#[test]
fn tribal_allies_offers_and_places_both_previously_excluded_tiles_after_reload() {
    for target in [pos("B4"), pos("C3")] {
        let game = fixture();
        let resources = game.player(0).resources.clone();
        let game = tribal_allies(game);
        assert_eq!(
            placement_choices(&game),
            positions(&["B4", "C2", "C3", "E2"])
        );
        // The real web decision query accepts the new choices, but not the empty
        // plains at C5 bordering the other human's C6 city.
        assert!(web_view::query(&game, 0, json!({"kind":"decision", "values":[target]})).is_ok());
        assert!(web_view::query(&game, 0, json!({"kind":"decision", "values":["C5"]})).is_err());
        let game = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
        let game = place_and_reinforce(game, target);
        assert_eq!(game.actions_left, 0);
        assert_eq!(game.player(0).resources, resources);
    }
}

#[test]
fn ordinary_incident_uses_the_same_spawn_choices_and_placement_rules() {
    let game = execute_action(
        fixture(),
        Action::Playing(PlayingAction::Advance(AdvanceAction::new(
            Advance::Storage,
            ResourcePile::ideas(2),
        ))),
        0,
    );
    assert!(matches!(
        game.current_event().event_type,
        PersistentEventType::Incident(_)
    ));
    assert_eq!(
        placement_choices(&game),
        placement_choices(&tribal_allies(fixture()))
    );
    assert_eq!(
        placement_choices(&game),
        positions(&["B4", "C2", "C3", "E2"])
    );
    assert!(web_view::query(&game, 0, json!({"kind":"decision", "values":["C5"]})).is_err());
    let game = place_and_reinforce(game, pos("B4"));
    assert_eq!(game.actions_left, 0);
}

#[test]
fn fortresses_neither_block_fallback_nor_enable_spawning_next_to_other_humans() {
    for own_fortress in [false, true] {
        for other_fortress in [false, true] {
            let mut game = fixture();
            game.player_mut(0).get_city_mut(pos("C4")).pieces.fortress = own_fortress.then_some(0);
            game.player_mut(1).get_city_mut(pos("C6")).pieces.fortress =
                other_fortress.then_some(1);
            assert_eq!(choices(&game), positions(&["B4", "C2", "C3", "E2"]));
            assert_eq!(
                placement_choices(&tribal_allies(game)),
                positions(&["B4", "C2", "C3", "E2"])
            );
        }
    }
}

#[test]
fn distance_two_has_priority_even_next_to_barbarians_and_falls_back_when_blocked() {
    let mut game = fixture();
    // B1 is two land steps from D2 via C2, but is not adjacent to any own city.
    game.map.tiles.insert(pos("B1"), Terrain::Fertile);
    game.map.tiles.insert(pos("A1"), Terrain::Fertile);
    let barbarian = get_barbarians_player(&game).index;
    game.player_mut(barbarian)
        .cities
        .push(City::new(barbarian, pos("A1")));
    assert_eq!(pos("B1").distance(pos("D2")), 2);
    assert_eq!(pos("B1").distance(pos("A1")), 1);
    assert_eq!(choices(&game), positions(&["B1"]));
    // Same geometry, but a human city must disqualify B1 and restore fallback.
    game.player_mut(barbarian)
        .cities
        .retain(|c| c.position != pos("A1"));
    game.player_mut(1).cities.push(City::new(1, pos("A1")));
    assert_eq!(choices(&game), positions(&["B4", "C2", "C3", "E2"]));
}

#[test]
fn distance_two_still_requires_a_land_path() {
    let mut game = fixture();
    game.map.tiles.insert(pos("B1"), Terrain::Fertile);
    assert_eq!(choices(&game), positions(&["B1"]));
    game.map.tiles.insert(pos("C2"), Terrain::Water);
    assert!(steps_towards_land_range2_cites(&game, game.player(0), pos("B1")).is_empty());
    assert_eq!(choices(&game), positions(&["B4", "C3", "E2"]));
}

#[test]
fn spawn_destination_still_excludes_cities_and_units_of_every_faction() {
    let target = pos("B4");
    for player in 0..fixture().players.len() {
        let mut game = fixture();
        assert!(choices(&game).contains(&target));
        let owner = game.player_mut(player);
        owner.units.push(Unit::new(
            player,
            target,
            UnitType::Settler,
            owner.next_unit_id,
        ));
        assert!(!choices(&game).contains(&target), "unit owner {player}");

        let mut game = fixture();
        game.player_mut(player)
            .cities
            .push(City::new(player, target));
        assert!(!choices(&game).contains(&target), "city owner {player}");
    }
}

#[test]
fn spawn_destination_still_requires_revealed_non_barren_non_exhausted_land() {
    let target = pos("B4");
    for (terrain, allowed) in [
        (Terrain::Fertile, true),
        (Terrain::Forest, true),
        (Terrain::Mountain, true),
        (Terrain::Barren, false),
        (Terrain::Water, false),
        (Terrain::Unexplored, false),
        (Terrain::Exhausted(Box::new(Terrain::Fertile)), false),
        (Terrain::Exhausted(Box::new(Terrain::Forest)), false),
        (Terrain::Exhausted(Box::new(Terrain::Mountain)), false),
    ] {
        let mut game = fixture();
        game.map.tiles.insert(target, terrain);
        assert_eq!(choices(&game).contains(&target), allowed);
    }
    let mut game = fixture();
    game.map.tiles.remove(&target);
    assert!(!choices(&game).contains(&target));
}
