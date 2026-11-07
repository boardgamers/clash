use crate::common::{
    JsonTest, TestAction, advance_action, custom_action, move_action, payment_response,
};
use server::action::Action;
use server::advance::Advance;
use server::collect::{Collect, PositionCollection};
use server::content::custom_actions::CustomActionType;
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction::WonderCard;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::recruit::Recruit;
use server::resource_pile::ResourcePile;
use server::unit::Units;
use server::wonder::Wonder;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "china");

#[test]
fn rice() {
    JSON.test(
        "rice",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Collect(Collect::new(
                Position::from_offset("B3"),
                vec![PositionCollection::new(
                    Position::from_offset("B4"),
                    ResourcePile::food(1),
                )],
                PlayingActionType::Collect,
            ))),
        )],
    );
}

#[test]
fn expansion() {
    JSON.test(
        "expansion",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Recruit(Recruit::new(
                &Units::new(1, 0, 0, 0, 0, None),
                Position::from_offset("A1"),
                ResourcePile::gold(2),
            ))),
        )],
    );
}

#[test]
fn fireworks() {
    JSON.test(
        "fireworks",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::gold(2))),
        ],
    );
}

#[test]
fn imperial_army() {
    JSON.test(
        "imperial_army",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::ImperialArmy)).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0, 4]))),
        ],
    );
}

#[test]
fn fast_war() {
    JSON.test(
        "fast_war",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("D8")),
        )],
    );
}

#[test]
fn great_wall() {
    JSON.test(
        "great_wall",
        vec![
            TestAction::undoable(
                0,
                advance_action(Advance::Engineering, ResourcePile::food(2)),
            )
            .skip_json(),
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::GreatWall))).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::new(3, 2, 7, 0, 0, 0, 3)))
                .skip_json(),
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)).skip_json(),
            TestAction::undoable(1, Action::Response(EventResponse::Bool(true))),
        ],
    );
}

#[test]
fn paid_fireworks_reports_final_hits_without_rewriting_the_roll() {
    use server::game_api::execute;
    use server::log::ActionLogEntry;
    let game = execute(
        JSON.load_game("fireworks"),
        move_action(vec![0], Position::from_offset("C1")),
        0,
    );
    let reports = |game: &server::game::Game| {
        game.log
            .iter()
            .flat_map(|a| &a.rounds)
            .flat_map(|r| &r.turns)
            .flat_map(|t| &t.actions)
            .flat_map(|a| &a.items)
            .filter(|i| i.player == 1)
            .filter_map(|i| match i.entry {
                ActionLogEntry::CombatHitReport {
                    combat_value,
                    hit_cancels,
                    extra_hits,
                    casualties,
                } => Some((combat_value, hit_cancels, extra_hits, casualties)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(reports(&game), vec![(6, 0, 0, 1)]);
    let original_roll_action = game
        .log
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
        .clone();
    let game = execute(game, payment_response(ResourcePile::gold(2)), 0);
    assert_eq!(reports(&game), vec![(6, 0, 0, 1), (6, 1, 0, 0)]);
    for seat in [None, Some(0), Some(1)] {
        let public = server::game_api::strip_secret(game.clone(), seat);
        assert_eq!(reports(&public), reports(&game));
    }
    let actions = &game
        .log
        .last()
        .unwrap()
        .rounds
        .last()
        .unwrap()
        .turns
        .last()
        .unwrap()
        .actions;
    assert_eq!(actions[actions.len() - 2].items, original_roll_action);
}
