use crate::action::Action;
use crate::advance::Advance;
use crate::content::persistent_events::EventResponse;
use crate::events::{EventOrigin, EventPlayer};
use crate::game::{
    Game, GameContext, GameLength, GameOptions, GameState, VariableEnd, variable_end_threshold,
};
use crate::game_setup::{GameSetupBuilder, setup_game};
use crate::status_phase::{StatusPhaseState, enter_status_phase, get_status_phase};
use crate::unit::UnitType;
use crate::wonder::{Wonder, construct_wonder, gain_wonder};

fn setup(length: GameLength) -> Game {
    let mut game = setup_game(
        &GameSetupBuilder::new(2)
            .seed("official-length-tests".into())
            .assigned_civilizations(vec!["Rome".into(), "Greece".into()])
            .options(GameOptions {
                length,
                ..GameOptions::default()
            })
            .build(),
    );
    for p in &mut game.players {
        p.objective_cards.clear();
    }
    game.starting_player_index = 0;
    game.current_player_index = 0;
    game
}

fn answer(game: Game, advance: Advance) -> Game {
    let player = game.active_player();
    crate::game_api::execute(
        game,
        Action::Response(EventResponse::SelectAdvance(advance)),
        player,
    )
}

#[test]
fn printed_track_and_legacy_options() {
    for age in 0..=9 {
        for round in 0..=4 {
            let expected = match (age, round) {
                (6, 2) => Some(6),
                (6, 3) => Some(5),
                (7, 1) => Some(4),
                (7, 2) => Some(3),
                (7, 3) => Some(2),
                _ => None,
            };
            assert_eq!(variable_end_threshold(age, round), expected);
        }
    }
    for (name, ages) in [
        ("Standard", 6),
        ("Epic", 8),
        ("Shorter", 4),
        ("RollToEnd", 7),
        ("WonderfulEnding", 7),
    ] {
        let options: GameOptions = serde_json::from_value(
            serde_json::json!({"length":name,"variant":"Builder","civilization":"DraftThree"}),
        )
        .expect("valid test options");
        assert_eq!(options.length.ages(), ages);
        assert_eq!(
            serde_json::to_value(options).expect("valid test options")["variant"],
            "Builder"
        );
    }
    for length in [
        serde_json::json!(["Shorter", "RollToEnd"]),
        serde_json::json!("Short"),
        serde_json::json!({"Shorter":true,"WonderfulEnding":true}),
    ] {
        assert!(
            serde_json::from_value::<GameOptions>(serde_json::json!({"length":length})).is_err()
        );
    }
    assert_eq!(
        serde_json::from_str::<GameOptions>("{}")
            .expect("valid test options")
            .length
            .ages(),
        6
    );
}

#[test]
fn shorter_setup_and_two_complete_passes() {
    let mut game = setup(GameLength::Shorter);
    for p in game.players.iter().filter(|p| p.is_human()) {
        assert_eq!(
            p.units
                .iter()
                .filter(|u| u.unit_type == UnitType::Settler)
                .count(),
            2
        );
    }
    game.starting_player_index = 1;
    enter_status_phase(&mut game);
    for (player, advance, phase) in [
        (1, Advance::Storage, StatusPhaseState::FreeAdvance),
        (0, Advance::Storage, StatusPhaseState::FreeAdvance),
        (1, Advance::Irrigation, StatusPhaseState::SecondFreeAdvance),
        (0, Advance::Irrigation, StatusPhaseState::SecondFreeAdvance),
    ] {
        assert_eq!(game.active_player(), player);
        assert_eq!(get_status_phase(&game), Some(&phase));
        if phase == StatusPhaseState::SecondFreeAdvance {
            game.player_mut(player).incident_tokens = 1;
        }
        let mood = game.player(player).resources.mood_tokens;
        let before = game.player(player).incident_tokens;
        game = answer(game, advance);
        assert!(game.player(player).has_advance(advance));
        if advance == Advance::Irrigation {
            assert_eq!(game.player(player).resources.mood_tokens, mood + 1);
        }
        assert_eq!(
            game.player(player).incident_tokens,
            before - u8::from(phase == StatusPhaseState::FreeAdvance)
        );
    }
    assert_ne!(
        get_status_phase(&game),
        Some(&StatusPhaseState::SecondFreeAdvance)
    );
}

#[test]
fn final_status_never_awards_advances() {
    for (length, age) in [
        (GameLength::Shorter, 4),
        (GameLength::Standard, 6),
        (GameLength::Epic, 8),
        (GameLength::RollToEnd, 7),
        (GameLength::WonderfulEnding, 7),
    ] {
        let mut game = setup(length);
        game.age = age;
        let advances = game.player(0).advances;
        enter_status_phase(&mut game);
        assert!(matches!(game.state, GameState::Finished));
        assert_eq!(game.player(0).advances, advances);
        assert!(game.events.is_empty());
    }
}

#[test]
fn every_roll_boundary_precedes_turn_start_and_cannot_be_undone() {
    for (age, round, threshold) in [(6, 2, 6), (6, 3, 5), (7, 1, 4), (7, 2, 3), (7, 3, 2)] {
        for roll in 1..=6 {
            let mut game = setup(GameLength::RollToEnd);
            game.age = age;
            game.round = round - 1;
            game.current_player_index = 1;
            game.dice_roll_outcomes = vec![(roll - 1) * 2];
            // A turn-start penalty must remain untouched when the roll ends play.
            game.permanent_effects
                .push(crate::content::effects::PermanentEffect::RevolutionLoseAction(0));
            game = crate::game_api::execute(
                game,
                Action::Playing(crate::playing_actions::PlayingAction::EndTurn),
                1,
            );
            assert_eq!(game.dice_roll_log.last(), Some(&((roll - 1) * 2)));
            assert_eq!(matches!(game.state, GameState::Finished), roll >= threshold);
            assert_eq!(game.variable_end.is_some(), roll >= threshold);
            assert!(!game.can_undo());
            if roll >= threshold {
                assert!(!game.permanent_effects.is_empty());
            }
            let restored =
                Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
            assert_eq!(restored.variable_end, game.variable_end);
            assert_eq!(restored.dice_roll_log, game.dice_roll_log);
            assert!(!restored.can_undo());
        }
    }
}

#[test]
fn rolls_only_at_eligible_round_start_not_each_player() {
    let mut game = setup(GameLength::RollToEnd);
    game.age = 6;
    game.dice_roll_outcomes = vec![0];
    game.next_turn();
    assert_eq!(game.dice_roll_outcomes.len(), 1);
    game.next_turn();
    assert!(game.dice_roll_outcomes.is_empty());
    game.dice_roll_outcomes.push(10);
    game.next_turn();
    assert_eq!(game.dice_roll_outcomes.len(), 1);
    assert!(!matches!(game.state, GameState::Finished));
}

#[test]
fn wonderful_ending_construction_only_and_finishes_whole_round() {
    for (age, round, eligible) in [
        (6, 1, false),
        (6, 2, true),
        (6, 3, true),
        (7, 1, true),
        (7, 2, true),
        (7, 3, true),
    ] {
        for builder in 0..2 {
            let mut game = setup(GameLength::WonderfulEnding);
            game.age = age;
            game.round = round;
            game.current_player_index = builder;
            crate::log::add_start_turn_action_if_needed(&mut game, builder);
            let player = EventPlayer::new(builder, EventOrigin::Wonder(Wonder::Colosseum));
            let city = game.player(builder).cities[0].position;
            construct_wonder(&mut game, &player, Wonder::Colosseum, city);
            assert_eq!(game.variable_end.is_some(), eligible);
            assert!(!matches!(game.state, GameState::Finished));
            if builder == 0 {
                game.next_turn();
                assert_eq!(game.current_player_index, 1);
                assert!(!matches!(game.state, GameState::Finished));
            }
            game.next_turn();
            assert_eq!(matches!(game.state, GameState::Finished), eligible);
        }
    }
    let mut game = setup(GameLength::WonderfulEnding);
    game.age = 7;
    game.round = 2;
    crate::log::add_start_turn_action_if_needed(&mut game, 0);
    let city = game.player(0).cities[0].position;
    gain_wonder(
        &mut game,
        &EventPlayer::new(0, EventOrigin::Wonder(Wonder::Colosseum)),
        Wonder::Colosseum,
        city,
    );
    assert_eq!(game.variable_end, None);
    game.variable_end = Some(VariableEnd::WonderfulEnding);
    let restored = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
    assert_eq!(restored.variable_end, game.variable_end);
}

#[test]
fn first_pass_still_triggers_an_incident() {
    let mut game = setup(GameLength::Shorter);
    game.player_mut(0).incident_tokens = 1;
    enter_status_phase(&mut game);
    let incidents = game.incidents_left.len();
    game = answer(game, Advance::Storage);
    assert_eq!(game.player(0).incident_tokens, 3);
    assert!(game.incidents_left.len() < incidents);
}

#[test]
fn second_pass_preserves_initializers_and_save_load() {
    let mut game = setup(GameLength::Shorter);
    enter_status_phase(&mut game);
    game = answer(game, Advance::Storage);
    game = answer(game, Advance::Storage);
    assert_eq!(
        get_status_phase(&game),
        Some(&StatusPhaseState::SecondFreeAdvance)
    );
    game = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
    game.player_mut(0).incident_tokens = 1;
    let cards = game.player(0).action_cards.len();
    let objectives = game.player(0).objective_cards.len();
    let culture = game.player(0).resources.culture_tokens;
    game = answer(game, Advance::Writing);
    assert_eq!(game.player(0).action_cards.len(), cards + 1);
    assert_eq!(game.player(0).objective_cards.len(), objectives + 1);
    assert_eq!(game.player(0).resources.culture_tokens, culture + 1);
    assert_eq!(game.player(0).incident_tokens, 1);
    assert_eq!(game.active_player(), 1);
    assert_eq!(
        get_status_phase(&game),
        Some(&StatusPhaseState::SecondFreeAdvance)
    );
}

#[test]
fn remaining_seats_finish_wonderful_round_after_a_drop() {
    for dropped in 0..3 {
        let mut game = setup_game(
            &GameSetupBuilder::new(3)
                .seed("drop-length".into())
                .options(GameOptions {
                    length: GameLength::WonderfulEnding,
                    ..GameOptions::default()
                })
                .build(),
        );
        for player in &mut game.players {
            player.objective_cards.clear();
        }
        game.starting_player_index = 0;
        game.current_player_index = 0;
        game.age = 6;
        game.round = 2;
        game.variable_end = Some(VariableEnd::WonderfulEnding);
        game.drop_player(dropped);
        assert!(!matches!(game.state, GameState::Finished));
        // The current round still gives each surviving seat its turn.
        game.next_turn();
        assert!(!matches!(game.state, GameState::Finished));
        game.next_turn();
        assert!(matches!(game.state, GameState::Finished));
    }
}

#[test]
fn first_player_choosing_self_cannot_undo_round_start_roll() {
    for roll in [0, 10] {
        let mut game = setup(GameLength::RollToEnd);
        game.age = 6;
        game.round = 4;
        game.dice_roll_outcomes = vec![roll];
        crate::status_phase::status_phase_response(
            &mut game,
            StatusPhaseState::DetermineFirstPlayer(0),
        );
        game = crate::game_api::execute(game, Action::Response(EventResponse::SelectPlayer(0)), 0);
        assert_eq!(game.age, 7);
        assert_eq!(game.round, 1);
        assert_eq!(game.dice_roll_log.last(), Some(&roll));
        assert!(!game.can_undo());
        assert_eq!(matches!(game.state, GameState::Finished), roll == 10);
    }
}

#[test]
fn second_pass_resumes_after_normal_nested_wonder_choice() {
    let mut game = setup(GameLength::Shorter);
    enter_status_phase(&mut game);
    game = answer(game, Advance::Storage);
    game = answer(game, Advance::Storage);
    game.player_mut(0).incident_tokens = 1;
    game.permanent_effects
        .push(crate::content::effects::PermanentEffect::PublicWonderCard(
            Wonder::Colosseum,
        ));
    game = answer(game, Advance::Engineering);
    assert_eq!(game.active_player(), 0);
    assert_eq!(
        get_status_phase(&game),
        Some(&StatusPhaseState::SecondFreeAdvance)
    );
    assert!(matches!(
        game.events.last().map(|event| &event.event_type),
        Some(crate::content::persistent_events::PersistentEventType::DrawWonderCard(_))
    ));
    game = Game::from_data(game.cloned_data(), game.cache.clone(), GameContext::Play);
    game = crate::game_api::execute(game, Action::Response(EventResponse::Bool(true)), 0);
    assert!(game.player(0).wonder_cards.contains(&Wonder::Colosseum));
    assert_eq!(game.player(0).incident_tokens, 1);
    assert_eq!(game.active_player(), 1);
    assert_eq!(
        get_status_phase(&game),
        Some(&StatusPhaseState::SecondFreeAdvance)
    );
}
