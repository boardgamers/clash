use server::action::Action;
use server::advance::Advance;
use server::barbarians::get_barbarians_player;
use server::cache::Cache;
use server::city::City;
use server::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use server::game::{Game, GameContext};
use server::game_api;
use server::game_data::GameData;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::{Unit, UnitType};

fn load_game(json: &str, boundary: Option<usize>) -> Game {
    let data: GameData = serde_json::from_str(json).expect("valid fixture");
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.barbarian_movement_from = boundary;
    game
}

fn reload(game: &Game) -> Game {
    let json = serde_json::to_string(&game.cloned_data()).expect("serialize game");
    let data: GameData = serde_json::from_str(&json).expect("deserialize game");
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

fn add_barbarian_unit(game: &mut Game, pos: Position, unit_type: UnitType) -> u32 {
    let barb = get_barbarians_player(game).index;
    let id = game.players[barb].next_unit_id;
    game.players[barb].next_unit_id += 1;
    game.players[barb]
        .units
        .push(Unit::new(barb, pos, unit_type, id));
    id
}

fn respond(game: Game, response: EventResponse) -> Game {
    let player = game.active_player();
    game_api::execute(game, Action::Response(response), player)
}

fn last_action_logs(game: &Game) -> Vec<String> {
    game.log
        .last()
        .unwrap()
        .rounds
        .last()
        .unwrap()
        .turns
        .last()
        .unwrap()
        .actions
        .last()
        .unwrap()
        .items
        .iter()
        .filter_map(|item| match &item.entry {
            server::log::ActionLogEntry::Text(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn setup_merging_barbarians(boundary: Option<usize>) -> (Game, u32, u32) {
    let mut game = load_game(
        include_str!("test_games/incidents/barbarians_move.json"),
        boundary,
    );
    let a3 = Position::from_offset("A3");
    let b2 = Position::from_offset("B2");
    let c2 = Position::from_offset("C2");
    // Remove P1's Sulla so StopBarbarianMovement does not interrupt the incident.
    game.players[1].units.clear();
    // Keep P0's two cities (A1, C2) as plain size-1 cities (no Fortress) so Storage triggers
    // incident #28 (Barbarians move) and B2 -> C2 captures C2 cleanly without combat.
    // In Odd-Q coordinates, B2 is distance 1 from C2 (and distance 2 from A1),
    // and A3 is distance 2 from C2 via B2 alone.
    let a1 = Position::from_offset("A1");
    game.players[0].cities = vec![City::new(0, a1), City::new(0, c2)];
    game.players[0]
        .units
        .retain(|u| u.position != c2 && u.position != b2);
    let barb = get_barbarians_player(&game).index;
    game.players[barb].cities.clear();
    game.players[barb].units.clear();
    let cav_id = add_barbarian_unit(&mut game, a3, UnitType::Cavalry);
    let inf_id = add_barbarian_unit(&mut game, b2, UnitType::Infantry);
    (game, cav_id, inf_id)
}

#[test]
fn merging_barbarian_armies_only_moves_unmoved_units_and_leaves_already_moved_behind() {
    let a3 = Position::from_offset("A3");
    let b2 = Position::from_offset("B2");
    let c2 = Position::from_offset("C2");

    // Corrected behavior (new game boundary = Some(0)):
    let (game, cav_id, inf_id) = setup_merging_barbarians(Some(0));
    let mut game = respond(game, EventResponse::SelectAdvance(Advance::Storage));
    // Both A3 and B2 are movable; select A3 first.
    // A3 has only destination B2 (auto-moves A3 -> B2), then B2 has only unmoved Infantry
    // and only destination C2 (auto-moves B2 -> C2).
    game = respond(game, EventResponse::SelectPositions(vec![a3]));
    let barb = get_barbarians_player(&game);
    assert_eq!(
        barb.get_units(b2).iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![cav_id],
        "already-moved Cavalry must remain at intermediate tile B2"
    );
    assert_eq!(
        barb.get_units(c2).iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![inf_id],
        "only unmoved Infantry should advance from B2 to C2"
    );
    let logs = last_action_logs(&game);
    assert!(
        logs.iter()
            .any(|l| l.contains("Barbarians move from B2 to C2: 1 infantry")),
        "expected B2 -> C2 move log to list only 1 infantry, got: {logs:?}"
    );
    assert!(
        !logs
            .iter()
            .any(|l| l.contains("B2 to C2") && l.contains("cavalry")),
        "B2 -> C2 move log must not mention already-moved cavalry: {logs:?}"
    );

    // Legacy boundary preserves historical double-move for deterministic replay of old games.
    let (legacy, cav_id, inf_id) = setup_merging_barbarians(Some(usize::MAX));
    let mut legacy = respond(legacy, EventResponse::SelectAdvance(Advance::Storage));
    legacy = respond(legacy, EventResponse::SelectPositions(vec![a3]));
    let barb = get_barbarians_player(&legacy);
    assert!(barb.get_units(b2).is_empty());
    let mut c2_ids: Vec<u32> = barb.get_units(c2).iter().map(|u| u.id).collect();
    c2_ids.sort();
    assert_eq!(c2_ids, vec![cav_id, inf_id]);
}

#[test]
fn merging_barbarian_armies_combat_attackers_exclude_already_moved_units() {
    let a3 = Position::from_offset("A3");
    let b2 = Position::from_offset("B2");
    let c2 = Position::from_offset("C2");

    let (mut game, cav_id, inf_id) = setup_merging_barbarians(Some(0));
    // Put a defending unit in C2 so B2 -> C2 starts combat.
    let defender_id = game.players[0].next_unit_id;
    game.players[0].next_unit_id += 1;
    game.players[0]
        .units
        .push(Unit::new(0, c2, UnitType::Infantry, defender_id));

    let mut game = respond(game, EventResponse::SelectAdvance(Advance::Storage));
    game = respond(game, EventResponse::SelectPositions(vec![a3]));

    // Combat at C2 must only have inf_id as attacker (cav_id stayed at B2).
    let barb = get_barbarians_player(&game);
    assert_eq!(
        barb.get_units(b2).iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![cav_id]
    );
    if let Some(combat) = game.events.iter().find_map(|e| match &e.event_type {
        PersistentEventType::CombatStart(c) => Some(c),
        _ => None,
    }) {
        assert_eq!(combat.attackers, vec![inf_id]);
    }
}

#[test]
fn stopped_barbarian_units_in_moved_units_do_not_move_with_unmoved_units_on_same_tile() {
    let b1 = Position::from_offset("B1");
    let b2 = Position::from_offset("B2");
    let c2 = Position::from_offset("C2");

    let mut game = load_game(
        include_str!("test_games/incidents/barbarians_move.json"),
        Some(0),
    );
    game.players[1].units.clear();
    // Place P0's two cities at B1 and C2 (both range 1 from B2), so B2 pauses for destination.
    game.players[0].cities = vec![City::new(0, b1), City::new(0, c2)];
    game.players[0].units.clear();
    let barb = get_barbarians_player(&game).index;
    game.players[barb].cities.clear();
    game.players[barb].units.clear();
    let stopped_cav = add_barbarian_unit(&mut game, b2, UnitType::Cavalry);
    let active_inf = add_barbarian_unit(&mut game, b2, UnitType::Infantry);

    let mut game = respond(game, EventResponse::SelectAdvance(Advance::Storage));
    // Mark stopped_cav as already stopped in state.moved_units while active_inf remains unmoved.
    for event in &mut game.events {
        if let PersistentEventType::Incident(i) = &mut event.event_type
            && let Some(state) = &mut i.barbarians
        {
            state.moved_units.push(stopped_cav);
        }
    }
    // B2 is paused waiting for destination choice (B1 or C2); pick C2.
    game = respond(game, EventResponse::SelectPositions(vec![c2]));

    let barb = get_barbarians_player(&game);
    assert_eq!(
        barb.get_units(b2).iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![stopped_cav],
        "stopped unit in moved_units must remain at B2"
    );
    assert_eq!(
        barb.get_units(c2).iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![active_inf],
        "unmoved unit must advance to C2"
    );
}

#[test]
fn barbarian_move_incident_enforces_stack_limit_in_destination_choices() {
    let a1 = Position::from_offset("A1");
    let b1 = Position::from_offset("B1");
    let b2 = Position::from_offset("B2");
    let c1 = Position::from_offset("C1");
    let c2 = Position::from_offset("C2");

    let mut game = load_game(
        include_str!("test_games/incidents/barbarians_move.json"),
        Some(0),
    );
    game.players[1].units.clear();
    game.players[1].cities.clear();
    // P0 has 2 cities (A1, C1); B2 is range 2 from C1 via land steps B1 and C2.
    game.players[0].cities = vec![City::new(0, a1), City::new(0, c1)];
    game.players[0].units.clear();
    let barb = get_barbarians_player(&game).index;
    game.players[barb].cities.clear();
    game.players[barb].units.clear();
    // 2 units on B2, 3 on B1 (2+3=5 > 4 -> invalid), 2 on C2 (2+2=4 -> valid).
    add_barbarian_unit(&mut game, b2, UnitType::Cavalry);
    add_barbarian_unit(&mut game, b2, UnitType::Cavalry);
    for _ in 0..3 {
        add_barbarian_unit(&mut game, b1, UnitType::Infantry);
    }
    for _ in 0..2 {
        add_barbarian_unit(&mut game, c2, UnitType::Infantry);
    }

    // Trigger incident #28; B2, B1, C2 are all movable (B1/C2 are range 1 from C1/A1).
    let mut game = respond(game, EventResponse::SelectAdvance(Advance::Storage));
    // Choose B2 to move first. Because B1 would overflow STACK_LIMIT (5 > 4),
    // C2 is the ONLY legal destination for B2, so B2 moves automatically to C2!
    game = respond(game, EventResponse::SelectPositions(vec![b2]));

    let barb = get_barbarians_player(&game);
    assert_eq!(barb.get_units(b2).len(), 0);
    assert_eq!(
        barb.get_units(c2).len(),
        4,
        "B2's 2 Cavalry must merge onto C2 (total 4), never overflowing B1"
    );
    assert_eq!(barb.get_units(b1).len(), 3);
}

#[test]
fn mercenaries_moving_first_army_onto_second_does_not_move_first_army_twice_and_survives_reload() {
    let a3 = Position::from_offset("A3");
    let b2 = Position::from_offset("B2");
    let b3 = Position::from_offset("B3");

    // Set up Mercenaries with 2 Cavalry at A3, 2 Infantry at B3, 2 Elephants at B2.
    let setup_merc = |boundary: Option<usize>| {
        let mut game = load_game(
            include_str!("test_games/action_cards/mercenaries.json"),
            boundary,
        );
        game.players[1].units.clear();
        game.players[0].units.retain(|u| u.position != b2);
        let barb = get_barbarians_player(&game).index;
        game.players[barb].cities.clear();
        game.players[barb].units.clear();
        let cav1 = add_barbarian_unit(&mut game, a3, UnitType::Cavalry);
        let cav2 = add_barbarian_unit(&mut game, a3, UnitType::Cavalry);
        let inf1 = add_barbarian_unit(&mut game, b3, UnitType::Infantry);
        let inf2 = add_barbarian_unit(&mut game, b3, UnitType::Infantry);
        let ele1 = add_barbarian_unit(&mut game, b2, UnitType::Elephant);
        let ele2 = add_barbarian_unit(&mut game, b2, UnitType::Elephant);
        (game, [cav1, cav2], [inf1, inf2], [ele1, ele2])
    };

    // Corrected behavior:
    let (game, cavs, infs, eles) = setup_merc(Some(0));
    let mut game = game_api::execute(game, Action::Playing(PlayingAction::ActionCard(13)), 0);
    // Select A3 and B3.
    game = respond(game, EventResponse::SelectPositions(vec![a3, b3]));
    // Pay 2 ore.
    game = respond(game, EventResponse::Payment(vec![ResourcePile::ore(2)]));
    // Pick A3 first, move onto B3 (merging 2 Cavalry onto 2 Infantry -> 4 on B3).
    game = respond(game, EventResponse::SelectPositions(vec![a3]));
    game = respond(game, EventResponse::SelectPositions(vec![b3]));
    // Save & reload mid-Mercenaries to verify moved_units persists across serialization!
    game = reload(&game);
    // Next prompt must be selecting destination for B3 (auto-selected as sole remaining army),
    // and B2 MUST be an allowed destination because only B3's 2 unmoved Infantry move (2 + 2 <= 4).
    let choices = match &game.current_event_handler().unwrap().request {
        PersistentEventRequest::SelectPositions(r) => r.choices.clone(),
        other => panic!("expected SelectPositions for B3 destination, got {other:?}"),
    };
    assert!(
        choices.contains(&b2),
        "B2 must be legal for B3's 2 unmoved Infantry (2+2<=4), choices: {choices:?}"
    );
    assert!(choices.contains(&a3));
    game = respond(game, EventResponse::SelectPositions(vec![b2]));
    assert!(game.events.is_empty());

    let barb = get_barbarians_player(&game);
    let mut b3_ids: Vec<u32> = barb.get_units(b3).iter().map(|u| u.id).collect();
    b3_ids.sort();
    assert_eq!(
        b3_ids,
        cavs.to_vec(),
        "already-moved Cavalry must remain on B3"
    );
    let mut b2_ids: Vec<u32> = barb.get_units(b2).iter().map(|u| u.id).collect();
    b2_ids.sort();
    let mut expected_b2 = vec![infs[0], infs[1], eles[0], eles[1]];
    expected_b2.sort();
    assert_eq!(
        b2_ids, expected_b2,
        "only B3's unmoved Infantry should join B2's Elephants"
    );

    // Legacy boundary preserves legacy Mercenaries behavior (moves all 4 units on B3 to A3,
    // excluding B2 because 4 + 2 > 4 -> A3 is auto-selected as the sole destination!).
    let (legacy, _cavs, _infs, _eles) = setup_merc(Some(usize::MAX));
    let mut legacy = game_api::execute(legacy, Action::Playing(PlayingAction::ActionCard(13)), 0);
    legacy = respond(legacy, EventResponse::SelectPositions(vec![a3, b3]));
    legacy = respond(legacy, EventResponse::Payment(vec![ResourcePile::ore(2)]));
    legacy = respond(legacy, EventResponse::SelectPositions(vec![a3]));
    legacy = respond(legacy, EventResponse::SelectPositions(vec![b3]));
    let barb = get_barbarians_player(&legacy);
    assert!(barb.get_units(b3).is_empty());
    assert_eq!(barb.get_units(a3).len(), 4);
}

#[test]
fn barbarian_movement_boundary_upgrade_undo_redo_and_branch() {
    let a3 = Position::from_offset("A3");
    let b2 = Position::from_offset("B2");
    let c2 = Position::from_offset("C2");

    // Upgraded save (barbarian_movement_from: None) enables corrected movement on next action.
    let (game, cav_id, inf_id) = setup_merging_barbarians(None);
    assert_eq!(game.barbarian_movement_from, None);
    let expected_boundary = game_api::log_length(&game) + 1;
    let game = respond(game, EventResponse::SelectAdvance(Advance::Storage));
    assert_eq!(game.barbarian_movement_from, Some(expected_boundary));
    let after_move = respond(game, EventResponse::SelectPositions(vec![a3]));
    assert_eq!(after_move.barbarian_movement_from, Some(expected_boundary));
    let barb = get_barbarians_player(&after_move);
    assert_eq!(
        barb.get_units(b2).iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![cav_id]
    );
    assert_eq!(
        barb.get_units(c2).iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![inf_id]
    );

    // Undo and redo preserve the boundary.
    let undone = game_api::execute(after_move, Action::Undo, 0);
    assert_eq!(undone.barbarian_movement_from, Some(expected_boundary));
    let redone = game_api::execute(undone, Action::Redo, 0);
    assert_eq!(redone.barbarian_movement_from, Some(expected_boundary));
    assert_eq!(
        get_barbarians_player(&redone)
            .get_units(b2)
            .iter()
            .map(|u| u.id)
            .collect::<Vec<_>>(),
        vec![cav_id]
    );

    // Branching after undo clamps a future legacy boundary to the new branch action.
    let (game, _, _) = setup_merging_barbarians(Some(usize::MAX));
    let game = respond(game, EventResponse::SelectAdvance(Advance::Storage));
    let game = respond(game, EventResponse::SelectPositions(vec![a3]));
    assert_eq!(game.barbarian_movement_from, Some(usize::MAX));
    let undone = game_api::execute(game, Action::Undo, 0);
    assert_eq!(undone.barbarian_movement_from, Some(usize::MAX));
    // Branch by choosing B2 first instead of redoing A3.
    let branch_boundary = game_api::log_length(&undone) + 1;
    let branched = respond(undone, EventResponse::SelectPositions(vec![b2]));
    assert_eq!(branched.barbarian_movement_from, Some(branch_boundary));
}
