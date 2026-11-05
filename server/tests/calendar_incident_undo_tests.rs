//! Calendar makes the existing top incident knowable, not other players' choices
//! or the randomness/hidden draws caused by resolving that incident.
use serde_json::{Value, json};
use server::{
    action::Action,
    advance::{Advance, AdvanceAction},
    board_history,
    cache::Cache,
    city::City,
    content::{
        custom_actions::{CustomAction, CustomActionType},
        persistent_events::{EventResponse, PersistentEventRequest},
    },
    game::{Game, GameContext},
    game_data::GameData,
    game_setup::{GameSetupBuilder, setup_game},
    playing_actions::PlayingAction,
    resource_pile::ResourcePile,
    timing::time_increments,
    wonder::Wonder,
};

fn saved(game: &Game) -> Value {
    serde_json::to_value(game.cloned_data()).unwrap()
}

fn load(value: Value) -> Game {
    let data: GameData = serde_json::from_value(value).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

fn play(game: Game, action: Action) -> Game {
    let actor = game.active_player();
    board_history::execute(game, action, actor).unwrap()
}

fn research(advance: Advance) -> Action {
    Action::Playing(PlayingAction::Advance(AdvanceAction::new(
        advance,
        ResourcePile::food(2),
    )))
}

fn ready(incident: u8, calendar: bool) -> Game {
    let mut game = setup_game(
        &GameSetupBuilder::new(2)
            .seed("calendar-incident-undo".into())
            .assigned_civilizations(vec!["Maya".into(), "Rome".into()])
            .build(),
    );
    if game.active_player() != 0 {
        game = play(game, Action::Playing(PlayingAction::EndTurn));
    }
    let mut data = saved(&game);
    for player in 0..2 {
        data["players"][player]["objective_cards"] = json!([]);
        data["players"][player]["action_cards"] = json!([]);
        data["players"][player]["resources"] =
            json!({"food":7,"ideas":7,"culture_tokens":3,"mood_tokens":3});
    }
    data["players"][0]["advances"] = if calendar {
        json!(["Farming", "Mining", "Math", "Astronomy"])
    } else {
        json!(["Farming", "Mining", "Math"])
    };
    data["players"][0]["incident_tokens"] = json!(1);
    data["incidents_left"] = json!(
        std::iter::once(incident)
            .chain([29, 30].into_iter().filter(|&id| id != incident))
            .collect::<Vec<_>>()
    );
    data["time_increments"] = json!([8, 5]);
    load(data)
}

fn add_city(game: &mut Game, player: usize) {
    let position = *game
        .map
        .tiles
        .iter()
        .filter(|(pos, terrain)| terrain.is_land() && game.try_get_any_city(**pos).is_none())
        .map(|(pos, _)| pos)
        .min()
        .unwrap();
    game.player_mut(player)
        .cities
        .push(City::new(player, position));
}

fn select_first_position(game: Game) -> Game {
    let PersistentEventRequest::SelectPositions(request) =
        &game.current_event_handler().unwrap().request
    else {
        panic!("Expected position choice");
    };
    let position = request.choices[0];
    play(
        game,
        Action::Response(EventResponse::SelectPositions(vec![position])),
    )
}

fn assert_same_position(actual: &Game, expected: &Game) {
    let actual = saved(actual);
    let expected = saved(expected);
    for key in [
        "players",
        "incidents_left",
        "incidents_discarded",
        "rng",
        "actions_left",
        "events",
        "permanent_effects",
        "map",
        "time_increments",
    ] {
        assert_eq!(actual.get(key), expected.get(key), "{key}");
    }
}

#[test]
fn calendar_incident_undo_redo_reload_preserves_decks_resources_and_clocks() {
    // Vulcan has no effect with fewer than four cities. No explicit peek needed.
    let before = ready(29, true);
    let floor = before.log_index;
    let mut game = before.clone();
    for _ in 0..10 {
        game = play(game, research(Advance::Storage));
        assert!(game.can_undo());
        assert_eq!(game.undo_limit, floor);
        assert!(game.events.is_empty());
        assert_eq!(game.incidents_discarded.last(), Some(&29));
        assert_eq!(time_increments(&game), [8, 5]);
        assert!(saved(&game).get("calendar_incident_undo").is_none());
        let resolved = game.clone();
        game = play(load(saved(&game)), Action::Undo);
        assert_same_position(&game, &before);
        assert!(!game.can_undo());
        assert!(game.can_redo());
        game = play(load(saved(&game)), Action::Redo);
        assert_same_position(&game, &resolved);
        game = play(game, Action::Undo);
    }
    assert!(board_history::execute(game, Action::Undo, 0).is_err());
}

#[test]
fn owner_only_incident_choices_can_be_undone_back_through_the_trigger() {
    // Pestilence: choose one of the owner's two cities; the other player has one.
    let mut before = ready(1, true);
    add_city(&mut before, 0);
    let choosing = play(before.clone(), research(Advance::Storage));
    assert_eq!(choosing.active_player(), 0);
    assert!(choosing.can_undo());
    let resolved = select_first_position(choosing.clone());
    assert!(resolved.events.is_empty());
    assert!(resolved.can_undo());
    let undone_choice = play(load(saved(&resolved)), Action::Undo);
    assert_same_position(&undone_choice, &choosing);
    let undone_trigger = play(undone_choice, Action::Undo);
    assert_same_position(&undone_trigger, &before);
    assert_eq!(time_increments(&undone_trigger), [8, 5]);
}

#[test]
fn another_players_choice_seals_the_trigger_even_after_control_returns() {
    for owner_also_chooses in [false, true] {
        let mut before = ready(1, true);
        if owner_also_chooses {
            add_city(&mut before, 0);
        }
        add_city(&mut before, 1);
        let mut game = play(before, research(Advance::Storage));
        if owner_also_chooses {
            assert!(game.can_undo());
            game = select_first_position(game);
        }
        assert_eq!(game.active_player(), 1);
        assert!(!game.can_undo());
        assert!(board_history::execute(game.clone(), Action::Undo, 0).is_err());
        assert!(board_history::execute(game.clone(), Action::Undo, 1).is_err());
        game = select_first_position(game);
        assert_eq!(game.active_player(), 0);
        assert!(!game.can_undo());
        assert!(board_history::execute(load(saved(&game)), Action::Undo, 0).is_err());
    }
}

#[test]
fn no_calendar_or_acquiring_calendar_in_the_triggering_action_still_locks() {
    for advance in [Advance::Storage, Advance::Astronomy] {
        let game = play(ready(29, false), research(advance));
        assert!(!game.can_undo());
        assert_eq!(time_increments(&game), [9, 5]);
        assert!(board_history::execute(game, Action::Undo, 0).is_err());
    }
}

#[test]
fn deferred_astronomy_incident_does_not_use_new_calendar_through_a_wonder() {
    for already_owned in [false, true] {
        let mut game = ready(29, already_owned);
        game.player_mut(0).cities[0]
            .pieces
            .wonders
            .push(Wonder::GreatMausoleum);
        game.incidents_discarded = vec![30];
        let advance = if already_owned {
            Advance::Storage
        } else {
            Advance::Astronomy
        };
        let choosing = play(load(saved(&game)), research(advance));
        assert!(matches!(
            choosing.current_event_handler().unwrap().request,
            PersistentEventRequest::BoolRequest(_)
        ));
        if !already_owned {
            assert!(
                choosing.calendar_knowledge.is_none(),
                "deferred acquisition must not disclose the pending draw"
            );
        }
        // The draw's direct origin is Great Mausoleum, not Astronomy.
        let resolved = play(choosing, Action::Response(EventResponse::Bool(false)));
        assert_eq!(resolved.can_undo(), already_owned);
        if !already_owned {
            assert_eq!(
                resolved.calendar_knowledge.as_ref().unwrap().incident,
                resolved.incidents_left.first().copied()
            );
        }
        assert_eq!(
            time_increments(&resolved),
            if already_owned { [8, 5] } else { [9, 5] }
        );
    }
}

#[test]
fn implicit_calendar_knowledge_cannot_refund_earlier_astronomy() {
    let mut before = ready(29, false);
    before.player_mut(0).incident_tokens = 2;
    let astronomy = play(before, research(Advance::Astronomy));
    assert!(
        !astronomy.can_undo(),
        "acquisition disclosure seals Astronomy immediately"
    );
    assert!(astronomy.player(0).has_advance(Advance::Astronomy));
    let game = play(astronomy.clone(), research(Advance::Storage));
    let undone = play(load(saved(&game)), Action::Undo);
    assert_same_position(&undone, &astronomy);
    assert_eq!(undone.undo_limit, astronomy.log_index);
    assert!(!undone.can_undo());
    assert!(board_history::execute(undone, Action::Undo, 0).is_err());
}

#[test]
fn implicit_calendar_knowledge_cannot_refund_a_previous_paid_burial() {
    let mut before = ready(30, true);
    before.incidents_left = vec![30, 29];
    let open = play(
        before,
        Action::Playing(PlayingAction::Custom(CustomAction::new(
            CustomActionType::Calendar,
            None,
        ))),
    );
    let buried = play(
        open,
        Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
            1,
        )])),
    );
    assert!(buried.can_undo());
    let game = play(buried.clone(), research(Advance::Storage));
    assert_eq!(
        server::game_api::strip_secret(game.clone(), Some(0))
            .calendar_knowledge
            .unwrap()
            .incident,
        None
    );
    let undone = play(game, Action::Undo);
    assert_same_position(&undone, &buried);
    assert!(!undone.can_undo());
    assert_eq!(
        server::game_api::strip_secret(undone.clone(), Some(0))
            .calendar_knowledge
            .unwrap()
            .incident,
        Some(29)
    );
    let floor = undone.undo_limit;
    let opened = play(
        load(saved(&undone)),
        Action::Playing(PlayingAction::Custom(CustomAction::new(
            CustomActionType::Calendar,
            None,
        ))),
    );
    assert_eq!(
        opened.undo_limit, floor,
        "the incident already disclosed this restored card"
    );
}

#[test]
fn drawing_other_hidden_cards_does_not_lose_its_existing_lock() {
    // Writing draws an Action and Objective card before triggering the known incident.
    let game = play(ready(29, true), research(Advance::Writing));
    assert_eq!(game.incidents_discarded.last(), Some(&29));
    assert_eq!(game.player(0).action_cards.len(), 1);
    assert_eq!(game.player(0).objective_cards.len(), 1);
    assert!(!game.can_undo());
    assert_eq!(time_increments(&game), [9, 5]);
    assert!(board_history::execute(game, Action::Undo, 0).is_err());
}

#[test]
fn an_empty_deck_reshuffle_still_locks_but_the_last_existing_card_does_not() {
    let mut game = ready(29, true);
    game.incidents_left = vec![29];
    let game = play(game, research(Advance::Storage));
    assert!(game.incidents_left.is_empty());
    assert!(game.can_undo());

    let mut game = ready(29, true);
    game.incidents_left.clear();
    let rng = game.rng.seed;
    let game = play(game, research(Advance::Storage));
    assert_ne!(game.rng.seed, rng);
    assert!(!game.can_undo());
    assert_eq!(time_increments(&game), [9, 5]);
}

#[test]
fn calendar_does_not_make_combat_rolls_undoable() {
    let mut data: Value =
        serde_json::from_str(include_str!("test_games/combat/retreat.json")).unwrap();
    data["players"][0]["civilization"] = json!("Maya");
    let advances = data["players"][0]["advances"].as_array_mut().unwrap();
    advances.extend([json!("Math"), json!("Astronomy")]);
    let mut game = load(data);
    game.dice_roll_outcomes = vec![4; 16];
    let before = game.dice_roll_log.len();
    let action = serde_json::from_value(json!({
        "Movement":{"Move":{"units":[0,1,2,3],"destination":"C1"}}
    }))
    .unwrap();
    let game = play(game, action);
    assert!(game.dice_roll_log.len() > before);
    assert!(!game.can_undo());
}
