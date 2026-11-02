//! Calendar's live boundary must not record look-only interactions.
use serde_json::{Value, json};
use server::{
    action::{Action, try_execute_action},
    advance::{Advance, AdvanceAction},
    board_history,
    cache::Cache,
    collect::{Collect, PositionCollection},
    content::{
        custom_actions::{CustomAction, CustomActionType},
        persistent_events::{EventResponse, PersistentEventRequest},
    },
    game::{Game, GameContext},
    game_api::{log_length, strip_secret},
    game_data::GameData,
    game_setup::{GameSetupBuilder, setup_game},
    playing_actions::{PlayingAction, PlayingActionType},
    resource_pile::ResourcePile,
    timing::time_increments,
};

fn saved(game: &Game) -> Value {
    serde_json::to_value(game.cloned_data()).unwrap()
}

fn reload(value: Value) -> Game {
    let data: GameData = serde_json::from_value(value).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

fn play(game: Game, action: Action) -> Game {
    board_history::execute(game, action, 0).unwrap()
}

fn calendar() -> Action {
    Action::Playing(PlayingAction::Custom(CustomAction::new(
        CustomActionType::Calendar,
        None,
    )))
}

fn payment(pile: ResourcePile) -> Action {
    Action::Response(EventResponse::Payment(vec![pile]))
}

fn keep() -> Action {
    payment(ResourcePile::empty())
}

fn bury() -> Action {
    payment(ResourcePile::culture_tokens(1))
}

fn research(advance: Advance) -> Action {
    Action::Playing(PlayingAction::Advance(AdvanceAction::new(
        advance,
        ResourcePile::food(2),
    )))
}

// All setup changes are legal recorded actions, so the resulting save replays.
fn before_astronomy() -> Game {
    let mut game = setup_game(
        &GameSetupBuilder::new(2)
            .seed("calendar-preview-regression".into())
            .assigned_civilizations(vec!["Maya".into(), "Rome".into()])
            .build(),
    );
    if game.active_player() != 0 {
        game = board_history::execute(game, Action::Playing(PlayingAction::EndTurn), 1).unwrap();
    }
    game = play(game, research(Advance::Math));
    let home = game.player(0).cities[0].position;
    for _ in 0..2 {
        game = play(
            game,
            Action::Playing(PlayingAction::Collect(Collect::new(
                home,
                vec![PositionCollection::new(home, ResourcePile::food(1))],
                PlayingActionType::Collect,
            ))),
        );
    }
    game = play(game, Action::Playing(PlayingAction::EndTurn));
    game = board_history::execute(game, Action::Playing(PlayingAction::EndTurn), 1).unwrap();
    game
}

fn ready() -> Game {
    play(before_astronomy(), research(Advance::Astronomy))
}

fn prompt(game: &Game) -> String {
    let PersistentEventRequest::Payment(requests) = &game.current_event_handler().unwrap().request
    else {
        panic!("Calendar payment request");
    };
    requests[0].name.clone()
}

fn current_turn_log(game: &Game) -> &server::log::ActionLogTurn {
    game.log
        .last()
        .unwrap()
        .rounds
        .last()
        .unwrap()
        .turns
        .last()
        .unwrap()
}

fn history_shape(game: &Game) -> (usize, usize, Value) {
    (
        log_length(game),
        current_turn_log(game).actions.len(),
        saved(game)["board_history"].clone(),
    )
}

#[test]
fn hundreds_of_keep_and_cancel_cycles_are_bounded_live_updates_without_clock_credits() {
    let original = ready();
    let shape = history_shape(&original);
    let resources = original.player(0).resources.clone();
    let counters = time_increments(&original);
    let deck = original.incidents_left.clone();
    let rng = original.rng.seed;
    let mut game = play(play(original, calendar()), Action::Undo);
    let sealed = saved(&game);
    for i in 0..200 {
        game = play(game, calendar());
        assert!(game.uncommitted_calendar && game.early_choice_update);
        assert!(game.can_undo());
        assert!(!game.can_redo());
        assert_eq!(history_shape(&game), shape);
        assert_eq!(game.player(0).resources, resources);
        assert_eq!(game.incidents_left, deck);
        assert_eq!(game.rng.seed, rng);
        assert_eq!(time_increments(&game), counters);
        assert!(saved(&game).to_string().len() < sealed.to_string().len() + 4000);
        game = play(game, if i % 2 == 0 { keep() } else { Action::Undo });
        assert!(!game.uncommitted_calendar && game.early_choice_update);
        assert_eq!(saved(&game), sealed);
    }
}

#[test]
fn looking_seals_astronomy_but_cancels_only_the_uncommitted_prompt() {
    let before = before_astronomy();
    let game = play(before.clone(), research(Advance::Astronomy));
    assert!(game.can_undo());
    let undone = play(game.clone(), Action::Undo);
    assert!(!undone.player(0).advances.contains(Advance::Astronomy));
    assert_eq!(undone.player(0).resources, before.player(0).resources);
    for close in [Action::Undo, keep()] {
        let peek = play(game.clone(), calendar());
        assert_eq!(peek.undo_limit, peek.log_index);
        assert!(
            current_turn_log(&peek)
                .actions
                .iter()
                .all(|a| a.undo.is_empty())
        );
        let closed = play(peek, close);
        assert!(closed.player(0).advances.contains(Advance::Astronomy));
        assert!(!closed.can_undo());
        assert_eq!(history_shape(&closed), history_shape(&game));
        assert!(board_history::execute(closed, Action::Undo, 0).is_err());
    }
}

#[test]
fn paid_burial_has_two_actions_and_one_step_undo_redo_until_the_next_look() {
    let start = ready();
    let count = log_length(&start);
    let deck = start.incidents_left.clone();
    let culture = start.player(0).resources.culture_tokens;
    let open = play(start.clone(), calendar());
    let paid = play(open, bury());
    assert!(!paid.uncommitted_calendar && !paid.early_choice_update);
    assert!(paid.events.is_empty());
    assert_eq!(log_length(&paid), count + 2);
    assert_eq!(paid.incidents_left.last(), deck.first());
    assert_eq!(paid.player(0).resources.culture_tokens, culture - 1);
    assert!(paid.can_undo());
    let mut undone = play(reload(saved(&paid)), Action::Undo);
    assert_eq!(log_length(&undone), count + 1);
    assert_eq!(undone.incidents_left, deck);
    assert_eq!(undone.player(0).resources.culture_tokens, culture);
    assert!(!undone.uncommitted_calendar);
    assert!(
        !undone.can_undo(),
        "recorded look is an information boundary"
    );
    assert!(undone.can_redo());
    assert!(prompt(&undone).starts_with(&undone.cache.get_incident(deck[0]).name));
    // Legacy/undo-restored prompts keep the ordinary one-response history flow.
    let kept = play(undone.clone(), keep());
    assert_eq!(log_length(&kept), count + 2);
    assert!(kept.events.is_empty());
    for _ in 0..3 {
        undone = play(play(undone, Action::Redo), Action::Undo);
        assert_eq!(log_length(&undone), count + 1);
        assert_eq!(undone.incidents_left, deck);
    }
    let redone = play(undone, Action::Redo);
    assert_eq!(redone.incidents_left, paid.incidents_left);
    assert_eq!(redone.player(0).resources, paid.player(0).resources);
    assert_eq!(time_increments(&redone), time_increments(&paid));
    let next = play(redone, calendar());
    let closed = play(next, Action::Undo);
    assert_eq!(log_length(&closed), count + 2);
    assert_eq!(closed.player(0).resources.culture_tokens, culture - 1);
    assert!(!closed.can_undo());
    assert_eq!(closed.incidents_left, paid.incidents_left);
}

#[test]
fn empty_deck_peek_is_speculative_and_paid_commit_reproduces_it() {
    let mut start = ready();
    start.incidents_left.clear();
    let rng = start.rng.seed;
    // The lower executor is the recorded/replay representation, with no live hook.
    let recorded = try_execute_action(start.clone(), calendar(), 0).unwrap();
    assert!(!recorded.uncommitted_calendar);
    assert!(!recorded.incidents_left.is_empty());
    assert_ne!(recorded.rng.seed, rng);
    let expected_top = recorded.incidents_left[0];
    let mut game = start.clone();
    for close in [keep(), Action::Undo, keep()] {
        let open = play(game, calendar());
        assert_eq!(prompt(&open), prompt(&recorded));
        assert!(open.incidents_left.is_empty());
        assert_eq!(open.rng.seed, rng);
        game = play(reload(saved(&open)), close);
        assert!(game.incidents_left.is_empty());
        assert_eq!(game.rng.seed, rng);
    }
    let paid = play(play(game, calendar()), bury());
    let expected = try_execute_action(recorded, bury(), 0).unwrap();
    assert_eq!(paid.incidents_left, expected.incidents_left);
    assert_eq!(paid.rng.seed, expected.rng.seed);
    assert_eq!(paid.incidents_left.last(), Some(&expected_top));
    let undone = play(paid, Action::Undo);
    assert_eq!(undone.incidents_left[0], expected_top);
    assert!(!undone.uncommitted_calendar);
    assert_eq!(undone.player(0).resources, start.player(0).resources);
}

#[test]
fn reload_privacy_and_rejections_do_not_expose_or_resolve_a_preview() {
    let start = ready();
    assert!(board_history::execute(start.clone(), calendar(), 1).is_err());
    assert!(board_history::execute(before_astronomy(), calendar(), 0).is_err());
    let open = play(start, calendar());
    let request = prompt(&open);
    let raw = saved(&open);
    assert_eq!(raw["uncommitted_calendar"], json!(true));
    let owner = strip_secret(open.clone(), Some(0));
    assert!(owner.uncommitted_calendar);
    assert_eq!(prompt(&owner), request);
    for seat in [Some(1), None] {
        let public = strip_secret(open.clone(), seat);
        assert!(public.uncommitted_calendar);
        assert_eq!(prompt(&public), "Calendar · Inspecting the next event");
        assert!(!saved(&public)["events"].to_string().contains(&request));
    }
    let open = reload(raw.clone());
    assert!(open.uncommitted_calendar && open.can_undo());
    for action in [
        Action::Response(EventResponse::Payment(vec![])),
        Action::Response(EventResponse::Payment(vec![ResourcePile::empty(); 2])),
        Action::Response(EventResponse::Bool(false)),
        payment(ResourcePile::food(1)),
        payment(ResourcePile::culture_tokens(2)),
        payment(ResourcePile::culture_tokens(1) + ResourcePile::food(1)),
        calendar(),
        Action::Playing(PlayingAction::EndTurn),
        Action::Redo,
    ] {
        assert!(board_history::execute(open.clone(), action, 0).is_err());
        assert_eq!(saved(&open), raw);
    }
    for action in [keep(), bury(), Action::Undo] {
        for seat in [1, usize::MAX] {
            assert!(board_history::execute(open.clone(), action.clone(), seat).is_err());
        }
    }
    let mut broke = open.clone();
    broke.player_mut(0).resources.culture_tokens = 0;
    assert!(board_history::execute(broke.clone(), bury(), 0).is_err());
    assert!(play(broke, keep()).events.is_empty());
    // Lower callers must not try to apply a nonexistent cursor patch.
    let closed = try_execute_action(open, Action::Undo, 0).unwrap();
    assert!(!closed.uncommitted_calendar);
    assert!(closed.early_choice_update);
    assert!(saved(&closed).get("uncommitted_calendar").is_none());
}

#[test]
fn independent_private_queue_updates_remain_available_during_a_preview() {
    let mut game = ready();
    // Seat 1 has finished its final turn and can preselect a status advance.
    game.round = 3;
    game.starting_player_index = 1;
    let offer = server::early_choices::offers(&game, 1)
        .into_iter()
        .find(|offer| offer.title == "Free Advance")
        .unwrap();
    let queue = |response| Action::QueueChoice {
        key: offer.key.clone(),
        response,
    };
    game = board_history::execute(game, queue(Some(offer.options[0].response.clone())), 1).unwrap();
    game.action_plan = Some(server::action_queue::Plan {
        player: 1,
        key: "failed-plan".into(),
        actions: vec![],
        waiting: false,
        error: Some("The earlier choice has changed; review your plan".into()),
    });
    let open = play(game, calendar());
    let original = saved(&open);
    for execute in [board_history::execute, try_execute_action] {
        let revised = execute(
            open.clone(),
            queue(Some(offer.options[1].response.clone())),
            1,
        )
        .unwrap();
        assert!(revised.uncommitted_calendar && revised.early_choice_update);
        assert_eq!(revised.early_choices[0].response, offer.options[1].response);
        let cleared = execute(
            revised,
            Action::QueueMoves {
                key: "failed-plan".into(),
                actions: vec![],
                waiting: false,
            },
            1,
        )
        .unwrap();
        assert!(cleared.action_plan.is_none());
        assert!(cleared.uncommitted_calendar && cleared.early_choice_update);
        assert_eq!(saved(&cleared)["events"], original["events"]);
        assert_eq!(history_shape(&cleared), history_shape(&open));
        assert_eq!(time_increments(&cleared), time_increments(&open));
        for close in [keep(), Action::Undo] {
            let closed = play(cleared.clone(), close);
            assert!(closed.events.is_empty());
            assert!(!closed.uncommitted_calendar && closed.early_choice_update);
            assert_eq!(closed.early_choices, cleared.early_choices);
            assert_eq!(history_shape(&closed), history_shape(&open));
        }
    }
}

#[test]
fn full_replay_omits_transient_prompts_preserves_the_seal_and_reproduces_paid_actions() {
    let game = ready();
    for state in [
        play(game.clone(), calendar()),
        play(play(game.clone(), calendar()), keep()),
    ] {
        let replayed =
            server::replay::replay(serde_json::from_value(saved(&state)).unwrap(), None).unwrap();
        assert_eq!(log_length(&replayed), log_length(&state));
        assert_eq!(replayed.incidents_left, state.incidents_left);
        assert_eq!(replayed.rng.seed, state.rng.seed);
        assert_eq!(replayed.player(0).resources, state.player(0).resources);
        assert!(!replayed.uncommitted_calendar);
        assert!(replayed.events.is_empty());
        assert!(
            !replayed.can_undo(),
            "current reconstruction must preserve the look's seal"
        );
    }
    let paid = play(play(game, calendar()), bury());
    let replayed =
        server::replay::replay(serde_json::from_value(saved(&paid)).unwrap(), None).unwrap();
    assert_eq!(log_length(&replayed), log_length(&paid));
    assert_eq!(replayed.incidents_left, paid.incidents_left);
    assert_eq!(replayed.rng.seed, paid.rng.seed);
    assert_eq!(replayed.player(0).resources, paid.player(0).resources);
    assert_eq!(time_increments(&replayed), time_increments(&paid));
    assert!(replayed.can_undo());
    let undone = play(replayed, Action::Undo);
    assert!(!undone.uncommitted_calendar);
    assert!(undone.can_redo());
    assert_eq!(log_length(&undone), log_length(&paid) - 1);
}
