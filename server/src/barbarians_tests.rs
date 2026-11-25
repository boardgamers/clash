use super::*;
use crate::action::Action;
use crate::cache::Cache;
use crate::content::persistent_events::{EventResponse, PersistentEventRequest};
use crate::game::GameContext;
use crate::game_data::GameData;
use crate::unit::Unit;

fn setup() -> Game {
    let data: GameData = serde_json::from_str(include_str!(
        "../tests/test_games/incidents/barbarians_move.json"
    ))
    .unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.events.clear();
    game.log_index = 1;
    crate::log::current_turn_log_mut(&mut game)
        .actions
        .push(crate::log::ActionLogAction::new(
            Action::Response(EventResponse::SelectAdvance(
                crate::advance::Advance::Storage,
            )),
            0,
            None,
            0,
        ));
    let barbarian = get_barbarians_player(&game).index;
    game.players[barbarian].units.clear();
    game.players[barbarian].cities.clear();
    game
}
fn city(game: &mut Game, position: &str, units: &[UnitType]) -> Position {
    let pos = Position::from_offset(position);
    let player = get_barbarians_player(game).index;
    game.players[player].cities.push(City::new(player, pos));
    for unit in units {
        let id = game.players[player].next_unit_id;
        game.players[player].next_unit_id += 1;
        game.players[player]
            .units
            .push(Unit::new(player, pos, *unit, id));
    }
    pos
}
fn choose(game: Game, response: EventResponse) -> Game {
    let player = game.active_player();
    crate::game_api::execute(game, Action::Response(response), player)
}

#[test]
fn infantry_unlocks_all_available_types_and_full_cities_cannot_reinforce() {
    let mut game = setup();
    let pos = city(&mut game, "B3", &[UnitType::Infantry]);
    assert_eq!(
        get_barbarian_reinforcement_choices(&game, pos),
        barbarian_fighters()
    );
    on_reinforce_barbarians(&mut game, 0, vec![pos]);
    assert!(
        matches!(&game.current_event_handler().unwrap().request, PersistentEventRequest::SelectUnitType(r) if r.choices == barbarian_fighters())
    );
    // Reload a real pending decision, then resolve it exactly once.
    game = game.clone();
    game = choose(game, EventResponse::SelectUnitType(UnitType::Elephant));
    assert!(game.events.is_empty());
    assert_eq!(
        get_barbarians_player(&game)
            .get_units(pos)
            .iter()
            .map(|u| u.unit_type)
            .collect::<Vec<_>>(),
        vec![UnitType::Infantry, UnitType::Elephant]
    );
    let full = city(&mut game, "B1", &[UnitType::Infantry; 4]);
    assert!(get_barbarian_reinforcement_choices(&game, full).is_empty());
    assert!(!possible_barbarians_reinforcements(&game).contains(&full));
}

#[test]
fn first_unit_must_be_infantry_and_the_human_supply_does_not_limit_barbarians() {
    let mut game = setup();
    let pos = city(&mut game, "B3", &[]);
    let human = &mut game.players[0];
    human.units = (0..16)
        .map(|id| Unit::new(0, Position::from_offset("A1"), UnitType::Infantry, id))
        .collect();
    assert_eq!(human.available_units().infantry, 0);
    reinforce_after_move(
        &mut game,
        &EventPlayer::new(0, EventOrigin::Ability("test".into())),
    );
    assert!(game.events.is_empty());
    assert_eq!(
        get_barbarians_player(&game).get_units(pos)[0].unit_type,
        UnitType::Infantry
    );
}

#[test]
fn exhausted_barbarian_supply_excludes_empty_cities_but_keeps_cavalry_and_elephants() {
    let mut game = setup();
    let empty = city(&mut game, "B1", &[]);
    let pos = city(&mut game, "B3", &[UnitType::Infantry]);
    let player = get_barbarians_player(&game).index;
    for id in 100..119 {
        game.players[player].units.push(Unit::new(
            player,
            Position::from_offset("D2"),
            UnitType::Infantry,
            id,
        ));
    }
    assert!(get_barbarian_reinforcement_choices(&game, empty).is_empty());
    assert!(!possible_barbarians_reinforcements(&game).contains(&empty));
    assert_eq!(
        get_barbarian_reinforcement_choices(&game, pos),
        vec![UnitType::Cavalry, UnitType::Elephant]
    );
    on_reinforce_barbarians(&mut game, 0, vec![empty, pos]);
    game = choose(game, EventResponse::SelectUnitType(UnitType::Cavalry));
    assert!(game.events.is_empty());
    assert_eq!(get_barbarians_player(&game).get_units(pos).len(), 2);
}

#[test]
fn each_city_receives_one_unit_and_supply_is_rechecked_between_choices() {
    let mut game = setup();
    let first = city(&mut game, "B1", &[UnitType::Infantry]);
    let second = city(&mut game, "B3", &[UnitType::Infantry]);
    let player = get_barbarians_player(&game).index;
    for id in 100..103 {
        game.players[player].units.push(Unit::new(
            player,
            Position::from_offset("D2"),
            UnitType::Elephant,
            id,
        ));
    }
    on_reinforce_barbarians(&mut game, 0, vec![first, second]);
    game = choose(game, EventResponse::SelectPositions(vec![second]));
    game = choose(game, EventResponse::SelectUnitType(UnitType::Elephant));
    assert!(
        matches!(&game.current_event_handler().unwrap().request, PersistentEventRequest::SelectUnitType(r) if r.choices == vec![UnitType::Infantry, UnitType::Cavalry])
    );
    game = game.clone();
    game = choose(game, EventResponse::SelectUnitType(UnitType::Cavalry));
    assert!(game.events.is_empty());
    assert_eq!(get_barbarians_player(&game).get_units(first).len(), 2);
    assert_eq!(get_barbarians_player(&game).get_units(second).len(), 2);
}

#[test]
fn three_empty_cities_reinforce_automatically_when_supply_can_serve_them_all() {
    let mut game = setup();
    let cities = ["B1", "B3", "D2"].map(|pos| city(&mut game, pos, &[]));
    on_reinforce_barbarians(&mut game, 0, cities.to_vec());
    assert!(
        game.events.is_empty(),
        "forced placements must not ask for city order"
    );
    for pos in cities {
        let units = get_barbarians_player(&game).get_units(pos);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].unit_type, UnitType::Infantry);
    }
}

#[test]
fn insufficient_infantry_supply_keeps_the_city_choice() {
    let mut game = setup();
    let first = city(&mut game, "B1", &[]);
    let second = city(&mut game, "B3", &[]);
    let barbarian = get_barbarians_player(&game).index;
    let limit = game.players[barbarian].unit_limit().infantry;
    for id in 0..u32::from(limit - 1) {
        game.players[barbarian].units.push(Unit::new(
            barbarian,
            Position::from_offset("D2"),
            UnitType::Infantry,
            id,
        ));
    }
    game.players[barbarian].next_unit_id = u32::from(limit);
    on_reinforce_barbarians(&mut game, 0, vec![first, second]);
    assert!(
        matches!(&game.current_event_handler().unwrap().request, PersistentEventRequest::SelectPositions(r) if r.choices == vec![first, second])
    );
    game = choose(game, EventResponse::SelectPositions(vec![second]));
    assert!(game.events.is_empty());
    assert!(get_barbarians_player(&game).get_units(first).is_empty());
    assert_eq!(get_barbarians_player(&game).get_units(second).len(), 1);
}

fn add_barbarian_units(game: &mut Game, pos: Position, units: &[UnitType]) -> Vec<u32> {
    let player = get_barbarians_player(game).index;
    units
        .iter()
        .map(|unit| {
            let id = game.players[player].next_unit_id;
            game.players[player].next_unit_id += 1;
            game.players[player]
                .units
                .push(Unit::new(player, pos, *unit, id));
            id
        })
        .collect()
}

#[test]
fn barbarian_move_destinations_respects_stack_limit_and_unmoved_units_in_normal_and_hunnic_modes() {
    let mut game = setup();
    game.barbarian_movement_from = Some(0);
    // In Odd-Q coordinates, B2=(1,1) is distance 2 from C1=(2,-1) with land steps B1=(1,0) and C2=(2,0).
    let a3 = Position::from_offset("A3");
    let b1 = Position::from_offset("B1");
    let b2 = Position::from_offset("B2");
    let b3 = Position::from_offset("B3");
    let c1 = Position::from_offset("C1");
    let c2 = Position::from_offset("C2");
    game.players[1].cities.clear();
    game.players[0].cities = vec![City::new(0, c1)];

    let moved_on_b2 = add_barbarian_units(&mut game, b2, &[UnitType::Cavalry, UnitType::Cavalry]);
    let _unmoved_on_b2 =
        add_barbarian_units(&mut game, b2, &[UnitType::Infantry, UnitType::Infantry]);
    add_barbarian_units(
        &mut game,
        b1,
        &[UnitType::Infantry, UnitType::Infantry, UnitType::Infantry],
    );
    add_barbarian_units(&mut game, c2, &[UnitType::Infantry, UnitType::Infantry]);

    let state = BarbariansEventState {
        moved_units: moved_on_b2,
        ..BarbariansEventState::new()
    };
    // Normal branch: 2 unmoved on B2 + 3 on B1 = 5 > STACK_LIMIT (excluded); 2 + 2 on C2 = 4 (included).
    assert_eq!(barbarian_move_destinations(&game, 0, &state, b2), vec![c2]);

    // Legacy boundary preserves stack_size = 0 in the normal branch.
    game.barbarian_movement_from = Some(usize::MAX);
    let mut legacy_dests = barbarian_move_destinations(&game, 0, &state, b2);
    legacy_dests.sort();
    assert_eq!(legacy_dests, vec![b1, c2]);

    // Hunnic Tribes branch: 2 already-moved + 2 unmoved on B2.
    let mut hun_game = setup();
    hun_game.barbarian_movement_from = Some(0);
    let moved_on_b2 =
        add_barbarian_units(&mut hun_game, b2, &[UnitType::Cavalry, UnitType::Cavalry]);
    let _unmoved_on_b2 =
        add_barbarian_units(&mut hun_game, b2, &[UnitType::Infantry, UnitType::Infantry]);
    add_barbarian_units(&mut hun_game, b3, &[UnitType::Infantry, UnitType::Infantry]);
    add_barbarian_units(
        &mut hun_game,
        b1,
        &[UnitType::Infantry, UnitType::Infantry, UnitType::Infantry],
    );
    let hun_state = BarbariansEventState {
        hunnic_tribes: true,
        moved_units: moved_on_b2,
        ..BarbariansEventState::new()
    };
    let dests = barbarian_move_destinations(&hun_game, 0, &hun_state, b2);
    assert!(dests.contains(&b3), "2 unmoved + 2 on B3 <= 4");
    assert!(dests.contains(&a3), "2 unmoved + 0 on A3 <= 4");
    assert!(!dests.contains(&b1), "2 unmoved + 3 on B1 = 5 > 4");

    // Legacy Hunnic Tribes counted all 4 units on B2, excluding B3 (4 + 2 > 4).
    hun_game.barbarian_movement_from = Some(usize::MAX);
    let legacy_hun = barbarian_move_destinations(&hun_game, 0, &hun_state, b2);
    assert!(!legacy_hun.contains(&b3));
    assert!(legacy_hun.contains(&a3));
}

#[test]
fn map_guide_matches_corrected_barbarian_move_destinations() {
    let mut game = setup();
    game.barbarian_movement_from = Some(0);
    let b1 = Position::from_offset("B1");
    let b2 = Position::from_offset("B2");
    let c1 = Position::from_offset("C1");
    let c2 = Position::from_offset("C2");
    game.players[1].cities.clear();
    game.players[0].cities = vec![City::new(0, c1)];
    add_barbarian_units(&mut game, b2, &[UnitType::Infantry, UnitType::Infantry]);
    add_barbarian_units(
        &mut game,
        b1,
        &[UnitType::Infantry, UnitType::Infantry, UnitType::Infantry],
    );
    add_barbarian_units(&mut game, c2, &[UnitType::Infantry, UnitType::Infantry]);

    let guide = map_guide(&game, 0);
    let b2_moves: Vec<&str> = guide["moves"]
        .as_array()
        .expect("moves array")
        .iter()
        .filter(|m| m["from"] == "B2")
        .map(|m| m["to"].as_str().expect("destination string"))
        .collect();
    assert_eq!(b2_moves, vec!["C2"]);
}
