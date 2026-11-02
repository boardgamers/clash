//! Exercise the authoritative executor used by both BGS move and tryMove.
use serde_json::{Value, json};
use server::{
    action::Action,
    action_queue, board_history,
    cache::Cache,
    game::{CivSetupOption, Game, GameContext, GameOptions},
    game_data::GameData,
    game_setup::{GameSetupBuilder, setup_game},
    timing::time_increments,
};

fn load(raw: &str) -> Game {
    let data: GameData = serde_json::from_str(raw).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

fn saved(game: &Game) -> Value {
    serde_json::to_value(game.cloned_data()).unwrap()
}

fn play(game: Game, actor: usize, action: Value) -> Game {
    board_history::execute(game, serde_json::from_value(action).unwrap(), actor).unwrap()
}

fn collect() -> Value {
    json!({"Playing":{"Collect":{"city_position":"C2","collections":[{"position":"B1","pile":{"ore":1},"times":1}],"action_type":"Collect"}}})
}

#[test]
fn deterministic_actions_undo_redo_and_reload_never_farm_credits() {
    let mut value: Value = serde_json::from_str(include_str!(
        "test_games/advances/collect_free_economy.json"
    ))
    .unwrap();
    // Nonzero totals from an earlier reveal must survive all navigation.
    value["time_increments"] = json!([11, 5]);
    let mut game = load(&value.to_string());
    for _ in 0..12 {
        game = play(game, 0, collect());
        assert_eq!(time_increments(&game), [11, 5]);
        game = load(&saved(&game).to_string());
        game = play(game, 0, json!("Undo"));
        assert_eq!(time_increments(&game), [11, 5]);
        game = play(game, 0, json!("Redo"));
        assert_eq!(time_increments(&game), [11, 5]);
        game = play(game, 0, json!("Undo"));
        assert_eq!(time_increments(&game), [11, 5]);
    }
    assert!(board_history::execute(game.clone(), Action::Undo, 0).is_err());
    assert_eq!(time_increments(&game), [11, 5]);
    game = play(game, 0, json!({"Playing":"EndTurn"}));
    assert_eq!(time_increments(&game), [12, 5]);
    assert!(
        !game.can_undo(),
        "a credited boundary must not be replay-farmable"
    );
}

#[test]
fn old_saves_and_migrations_begin_at_zero_without_changing_npc_or_dropped_seats() {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/legacy_0_4_16/setup.json")).unwrap();
    value["options"]["undo"] = json!("SamePlayer");
    let mut game = load(&value.to_string());
    assert_eq!(time_increments(&game), [0, 0]);
    assert!(game.players.len() > time_increments(&game).len());
    game.dropped_players.push(1);
    assert_eq!(time_increments(&game), [0, 0]);
    assert_eq!(time_increments(&game.clone()), [0, 0]);
}

#[test]
fn spy_reveals_without_rng_changes_but_payment_declaration_and_decline_do_not() {
    let game = load(include_str!("test_games/action_cards/spy.json"));
    let rng = saved(&game)["rng"].clone();
    let game = play(game, 0, json!({"Playing":{"ActionCard":7}}));
    assert_eq!(time_increments(&game), [0, 0]);
    let game = play(
        game,
        0,
        json!({"Response":{"Payment":[{"culture_tokens":1}]}}),
    );
    assert_eq!(saved(&game)["rng"], rng);
    assert_eq!(time_increments(&game), [1, 0]);
    assert!(!game.can_undo());
    let declined = play(game.clone(), 0, json!({"Response":{"SelectHandCards":[]}}));
    assert_eq!(time_increments(&declined), [1, 0]);
    let swapped = play(
        game,
        0,
        json!({"Response":{"SelectHandCards":[{"ActionCard":1},{"ActionCard":2}]}}),
    );
    assert_eq!(time_increments(&swapped), [2, 0]);
    assert_eq!(saved(&swapped)["rng"], rng);
}

#[test]
fn exploration_earns_once_but_deterministic_orientation_does_not() {
    let game = load(include_str!("test_games/movement/explore_resolution.json"));
    let game = play(
        game,
        1,
        json!({"Movement":{"Move":{"units":[0],"destination":"D6"}}}),
    );
    assert_eq!(time_increments(&game), [0, 1]);
    assert!(!game.can_undo());
    let game = play(game, 1, json!({"Response":{"ExploreResolution":3}}));
    assert_eq!(time_increments(&game), [0, 1]);
    let game = play(game, 1, json!("Undo"));
    assert_eq!(time_increments(&game), [0, 1]);
}

#[test]
fn a_card_draw_earns_once_even_when_the_pre_shuffled_rng_does_not_change() {
    let game = load(include_str!("test_games/base/same_player_undo.json"));
    let rng = saved(&game)["rng"].clone();
    let game = play(
        game,
        0,
        json!({"Playing":{"Construct":{
            "city_position":"C2", "city_piece":"Observatory", "payment":{"food":1,"wood":1,"ore":1}
        }}}),
    );
    assert_eq!(time_increments(&game), [1, 0]);
    assert_eq!(saved(&game)["rng"], rng);
    assert!(!game.can_undo());
}

#[test]
fn many_combat_rolls_in_one_action_coalesce_and_same_player_deterministic_response_does_not() {
    let mut game = load(include_str!("test_games/combat/retreat.json"));
    game.player_mut(0).units.retain(|unit| unit.id < 4);
    for unit in &mut game.player_mut(0).units {
        unit.unit_type = server::unit::UnitType::Infantry;
    }
    let defender = game.player(1).units[0].clone();
    game.player_mut(1).units = (0..4)
        .map(|id| {
            let mut unit = defender.clone();
            unit.id = id;
            unit.unit_type = server::unit::UnitType::Cavalry;
            unit
        })
        .collect();
    game.player_mut(1).next_unit_id = 4;
    game.dice_roll_outcomes = vec![4; 8];
    let before_rolls = game.dice_roll_log.len();
    let game = play(
        game,
        0,
        json!({"Movement":{"Move":{"units":[0,1,2,3],"destination":"C1"}}}),
    );
    assert_eq!(game.dice_roll_log.len() - before_rolls, 8);
    assert_eq!(time_increments(&game), [1, 0]);
    assert_eq!(game.active_player(), 0);
    let game = play(game, 0, json!({"Response":{"Bool":true}}));
    assert_eq!(time_increments(&game), [1, 0]);
}

#[test]
fn off_turn_response_and_queued_end_turn_credit_their_own_actors() {
    let mut value: Value =
        serde_json::from_str(include_str!("test_games/action_cards/tech_trade.json")).unwrap();
    value["round"] = json!(2);
    value["players"][0]["advances"]
        .as_array_mut()
        .unwrap()
        .push(json!("Irrigation"));
    let game = play(
        load(&value.to_string()),
        0,
        json!({"Playing":{"ActionCard":39}}),
    );
    assert_eq!(game.active_player(), 1);
    assert_eq!(time_increments(&game), [1, 0]);
    let (key, _) = action_queue::offer(&game, 0).unwrap();
    let mut game = game;
    for waiting in [true, false, true, false] {
        game = play(
            game,
            0,
            json!({"QueueMoves":{"key":key,"actions":[{"Playing":"EndTurn"}],"waiting":waiting}}),
        );
        assert_eq!(time_increments(&game), [1, 0]);
    }
    game = load(&saved(&game).to_string());
    game = play(game, 1, json!({"Response":{"SelectAdvance":"Fishing"}}));
    assert_eq!(game.active_player(), 1);
    assert_eq!(time_increments(&game), [2, 1]);
    assert!(game.action_plan.is_none());
}

#[test]
fn early_choice_revisions_are_free_then_automatic_handoffs_credit_each_actor() {
    let game = load(include_str!(
        "test_games/status_phase/raze_city_decline.json"
    ));
    let offer = server::early_choices::offers(&game, 1)
        .into_iter()
        .find(|offer| offer.title == "Raze city")
        .unwrap();
    let mut game = game;
    for response in [
        json!({"SelectPositions":[]}),
        Value::Null,
        json!({"SelectPositions":[]}),
    ] {
        game = play(
            game,
            1,
            json!({"QueueChoice":{"key":offer.key,"response":response}}),
        );
        assert_eq!(time_increments(&game), [0, 0]);
    }
    // Both responses are actual control handoffs, even though BGS saved one move.
    game = play(game, 0, json!({"Response":{"SelectPositions":[]}}));
    assert_eq!(time_increments(&game), [1, 1]);
    assert!(game.early_choices.is_empty());
}

#[test]
fn automatic_keep_government_is_not_a_boundary_when_control_stays_with_the_same_player() {
    let mut game = load(include_str!("test_games/status_phase/keep_government.json"));
    game.player_mut(0).settings.keep_government = Some(true);
    assert_eq!(time_increments(&game), [0, 0]);
    // Player 1 hands off to 0. Player 0's automatic payment then moves to a new
    // status phase, but still asks player 0: an undo lock alone is not a credit.
    game = play(game, 1, json!({"Response":{"SelectPositions":[]}}));
    assert_eq!(game.active_player(), 0);
    assert_eq!(time_increments(&game), [0, 1]);
    assert!(matches!(
        &game.current_event_handler().unwrap().request,
        server::content::persistent_events::PersistentEventRequest::SelectPlayer(_)
    ));
}

#[test]
fn automatic_response_chain_keeps_the_preceding_multi_draw_credit() {
    let game = load(include_str!("test_games/status_phase/free_advance.json"));
    let game = play(game, 0, json!({"Playing":"EndTurn"}));
    let game = play(game, 1, json!({"Response":{"SelectAdvance":"Storage"}}));
    let before = time_increments(&game);
    let cards_before: usize = game
        .players
        .iter()
        .map(|p| p.action_cards.len() + p.objective_cards.len())
        .sum();
    let mut automatic = game.clone();
    for player in &mut automatic.players {
        player.settings.skip_raze_city = Some(true);
        player.settings.keep_government = Some(true);
    }
    let response = json!({"Response":{"SelectAdvance":"Philosophy"}});
    let mut manual = play(game, 0, response.clone());
    assert_eq!(time_increments(&manual), [before[0] + 1, before[1]]);
    let cards_after: usize = manual
        .players
        .iter()
        .map(|p| p.action_cards.len() + p.objective_cards.len())
        .sum();
    assert!(
        cards_after >= cards_before + 4,
        "several draws still earn only one credit"
    );
    loop {
        use server::content::persistent_events::PersistentEventRequest;
        let response = match &manual.current_event_handler().unwrap().request {
            PersistentEventRequest::SelectPositions(_) => {
                json!({"Response":{"SelectPositions":[]}})
            }
            PersistentEventRequest::Payment(_) => json!({"Response":{"Payment":[{}]}}),
            PersistentEventRequest::SelectPlayer(_) => break,
            other => panic!("unexpected request {other:?}"),
        };
        let actor = manual.active_player();
        manual = play(manual, actor, response);
    }
    let automatic = play(automatic, 0, response);
    assert_eq!(time_increments(&automatic), time_increments(&manual));
    assert_eq!(automatic.active_player(), manual.active_player());
}

#[test]
fn initial_draft_commitments_credit_once_each_not_revisions_or_final_setup_draws() {
    let mut game = setup_game(
        &GameSetupBuilder::new(2)
            .seed("timing-draft".into())
            .options(GameOptions {
                civilization: CivSetupOption::DraftThree,
                ..GameOptions::default()
            })
            .build(),
    );
    let offers = game.civilization_draft.as_ref().unwrap().offers.clone();
    assert_eq!(time_increments(&game), [0, 0]);
    game = play(game, 0, json!({"ChooseCivilization":offers[0][0]}));
    assert_eq!(time_increments(&game), [1, 0]);
    game = load(&saved(&game).to_string());
    for choice in &offers[0] {
        game = play(game, 0, json!({"ChooseCivilization":choice}));
        assert_eq!(time_increments(&game), [1, 0]);
    }
    game = play(game, 1, json!({"ChooseCivilization":offers[1][0]}));
    assert!(game.civilization_draft.is_none());
    assert_eq!(time_increments(&game), [1, 1]);
}

#[test]
fn historical_replay_and_previews_preserve_not_recompute_clock_totals() {
    let game = server::game_api::init(2, "timing-replay".into(), GameOptions::default());
    let actor = game.active_player();
    let game = play(game, actor, json!({"Playing":"EndTurn"}));
    let counters = time_increments(&game);
    let raw = saved(&game).to_string();
    for target in [None, Some(server::game_api::log_length(&game))] {
        let replayed = server::replay::replay(serde_json::from_str(&raw).unwrap(), target).unwrap();
        assert_eq!(time_increments(&replayed), counters);
    }
    let next_actor = game.active_player();
    let preview = server::action::try_execute_action(
        game.clone(),
        serde_json::from_value(json!({"Playing":"EndTurn"})).unwrap(),
        next_actor,
    )
    .unwrap();
    assert_eq!(time_increments(&preview), counters);
    for context in [
        GameContext::Replay,
        GameContext::AI,
        GameContext::CombatPreview,
    ] {
        let mut simulation = game.clone();
        simulation.context = context;
        let simulation = play(simulation, next_actor, json!({"Playing":"EndTurn"}));
        assert_eq!(time_increments(&simulation), counters);
    }
}

#[test]
fn counters_saturate_at_js_safe_integer_and_reject_invalid_saved_values() {
    let game = server::game_api::init(2, "timing-safe".into(), GameOptions::default());
    let actor = game.active_player();
    let mut value = saved(&game);
    let max = (1_u64 << 53) - 1;
    value["time_increments"] = json!([max, max]);
    let game = play(
        load(&value.to_string()),
        actor,
        json!({"Playing":"EndTurn"}),
    );
    assert_eq!(time_increments(&game), [max, max]);
    for invalid in [json!([-1, 0]), json!([1.5, 0]), json!([max + 1, 0])] {
        value["time_increments"] = invalid;
        assert!(serde_json::from_value::<GameData>(value.clone()).is_err());
    }
}
