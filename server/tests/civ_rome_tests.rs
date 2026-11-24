use crate::common::{
    JsonTest, TestAction, advance_action, custom_action, move_action, payment_response,
};
use server::action::Action;
use server::advance::Advance;
use server::city_pieces::Building;
use server::construct::Construct;
use server::content::custom_actions::{CustomActionType, PlayingActionModifier, SpecialAction};
use server::happiness::IncreaseHappiness;
use server::movement::{MoveUnits, MovementAction};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "rome");

#[test]
fn aqueduct_discount() {
    JSON.test(
        "aqueduct_discount",
        vec![TestAction::undoable(
            0,
            advance_action(Advance::Sanitation, ResourcePile::empty()),
        )],
    );
}

#[test]
fn aqueduct_free_action() {
    JSON.test(
        "aqueduct_free_action",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::Aqueduct)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::food(2))),
        ],
    );
}

#[test]
fn roman_roads() {
    JSON.test(
        "roman_roads",
        vec![TestAction::undoable(
            0,
            Action::Movement(MovementAction::Move(MoveUnits::new(
                vec![0, 1, 2, 3, 4, 5],
                Position::from_offset("A1"),
                None,
                ResourcePile::food(1) + ResourcePile::ore(1),
            ))),
        )],
    );
}

#[test]
fn captivi() {
    JSON.test(
        "captivi",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Construct(Construct::new(
                Position::from_offset("A1"),
                Building::Market,
                ResourcePile::new(1, 1, 0, 0, 0, 1, 0),
            ))),
        )],
    );
}

#[test]
fn provinces() {
    JSON.test(
        "provinces",
        vec![
            TestAction::undoable(0, advance_action(Advance::Voting, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("B2"))).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))),
        ],
    );
}

#[test]
fn princeps() {
    JSON.test(
        "princeps",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::Princeps)).skip_json(),
            TestAction::not_undoable(0, payment_response(ResourcePile::culture_tokens(1))),
        ],
    );
}

#[test]
fn emperor() {
    JSON.test(
        "emperor",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("D8")),
        )],
    );
}

#[test]
fn statesman() {
    JSON.test(
        "statesman",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::IncreaseHappiness(IncreaseHappiness::new(
                vec![(Position::from_offset("A1"), 1)],
                ResourcePile::mood_tokens(2),
                PlayingActionType::Special(SpecialAction::Modifier(
                    PlayingActionModifier::StatesmanIncreaseHappiness,
                )),
            ))),
        )],
    );
}

#[test]
fn proconsul() {
    JSON.test(
        "proconsul",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("D8")))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::gold(1))),
        ],
    );
}
#[test]
fn civilizer() {
    JSON.test(
        "civilizer",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("D8")),
        )],
    );
}

fn provinces_capture(culture: u8, corrected: bool) -> server::game::Game {
    let mut game = JSON.load_game("provinces");
    game.provinces_from = Some(if corrected { 0 } else { usize::MAX });
    game = server::game_api::execute(
        game,
        advance_action(Advance::Voting, ResourcePile::food(2)),
        0,
    );
    game.players[0].resources.culture_tokens = culture;
    server::game_api::execute(game, move_action(vec![0], Position::from_offset("B2")), 0)
}

#[test]
fn provinces_zero_culture_is_neutral_without_a_prompt() {
    let game = provinces_capture(0, true);
    assert_eq!(
        game.player(0)
            .get_city(Position::from_offset("B2"))
            .mood_state,
        server::city::MoodState::Neutral
    );
    assert!(game.events.is_empty());
}

#[test]
fn provinces_legacy_capture_retains_angry() {
    let game = provinces_capture(0, false);
    assert_eq!(
        game.player(0)
            .get_city(Position::from_offset("B2"))
            .mood_state,
        server::city::MoodState::Angry
    );
}

#[test]
fn provinces_optional_payment_accept_decline_and_legacy_resume() {
    for corrected in [false, true] {
        for accept in [false, true] {
            let game = provinces_capture(1, corrected);
            assert_eq!(
                game.player(0)
                    .get_city(Position::from_offset("B2"))
                    .mood_state,
                if corrected {
                    server::city::MoodState::Neutral
                } else {
                    server::city::MoodState::Angry
                }
            );
            let mut game = server::game::Game::from_data(
                game.cloned_data(),
                server::cache::Cache::new(&game.options),
                server::game::GameContext::Play,
            );
            // Existing pending choices remain valid when an old save is upgraded.
            game.provinces_from = None;
            let game = server::game_api::execute(
                game,
                payment_response(if accept {
                    ResourcePile::culture_tokens(1)
                } else {
                    ResourcePile::empty()
                }),
                0,
            );
            assert_eq!(
                game.player(0)
                    .get_city(Position::from_offset("B2"))
                    .mood_state,
                if accept {
                    server::city::MoodState::Happy
                } else {
                    server::city::MoodState::Neutral
                }
            );
            assert_eq!(game.player(0).resources.culture_tokens, u8::from(!accept));
        }
    }
}

#[test]
fn provinces_obsolete_prompt_refunds_and_does_not_touch_another_owner() {
    for accept in [false, true] {
        let mut game = provinces_capture(1, false);
        let position = Position::from_offset("B2");
        let index = game.players[0]
            .cities
            .iter()
            .position(|c| c.position == position)
            .unwrap();
        let city = game.players[0].cities.remove(index);
        game.players[1].cities.push(city);
        let game = server::game_api::execute(
            game,
            payment_response(if accept {
                ResourcePile::culture_tokens(1)
            } else {
                ResourcePile::empty()
            }),
            0,
        );
        assert_eq!(game.player(0).resources.culture_tokens, 1);
        assert_eq!(
            game.player(1).get_city(position).mood_state,
            server::city::MoodState::Angry
        );
    }
    let mut game = provinces_capture(1, false);
    game.players[0]
        .cities
        .iter_mut()
        .find(|c| c.position == Position::from_offset("B2"))
        .unwrap()
        .mood_state = server::city::MoodState::Happy;
    let game = server::game_api::execute(game, payment_response(ResourcePile::empty()), 0);
    assert_eq!(
        game.player(0)
            .get_city(Position::from_offset("B2"))
            .mood_state,
        server::city::MoodState::Happy
    );
}

#[test]
fn provinces_boundary_upgrade_undo_redo_and_branch() {
    let mut game = JSON.load_game("provinces");
    game = server::game_api::execute(
        game,
        advance_action(Advance::Voting, ResourcePile::food(2)),
        0,
    );
    game.players[0].resources.culture_tokens = 0;
    game.provinces_from = None;
    let boundary = server::game_api::log_length(&game) + 1;
    let action = move_action(vec![0], Position::from_offset("B2"));
    game = server::game_api::execute(game, action.clone(), 0);
    assert_eq!(game.provinces_from, Some(boundary));
    game = server::game_api::execute(game, Action::Undo, 0);
    assert_eq!(game.provinces_from, Some(boundary));
    game = server::game_api::execute(game, Action::Redo, 0);
    assert_eq!(
        game.player(0)
            .get_city(Position::from_offset("B2"))
            .mood_state,
        server::city::MoodState::Neutral
    );
    game = server::game_api::execute(game, Action::Undo, 0);
    game.provinces_from = Some(usize::MAX);
    game = server::game_api::execute(game, action, 0);
    assert_eq!(game.provinces_from, Some(boundary));
    assert_eq!(
        game.player(0)
            .get_city(Position::from_offset("B2"))
            .mood_state,
        server::city::MoodState::Neutral
    );
}

#[test]
fn provinces_at_city_limit_automatically_razes_without_prompt() {
    let mut game = JSON.load_game("provinces");
    game.provinces_from = Some(0);
    game = server::game_api::execute(
        game,
        advance_action(Advance::Voting, ResourcePile::food(2)),
        0,
    );
    // Use spare cities solely to exhaust the city pieces; capture remains real.
    while game.players[0].cities.len() < 7 {
        let city = server::city::City::new(
            0,
            Position::from_offset(&format!("G{}", game.players[0].cities.len())),
        );
        game.players[0].cities.push(city);
    }
    let game =
        server::game_api::execute(game, move_action(vec![0], Position::from_offset("B2")), 0);
    assert!(
        game.player(0)
            .try_get_city(Position::from_offset("B2"))
            .is_none()
    );
    assert!(game.events.is_empty());
}
