use crate::common::JsonTest;
use itertools::Itertools;
use server::action::{Action, ActionType};
use server::ai_actions::{city_collections, get_available_actions};
use server::collect::PositionCollection;
use server::playing_actions::{
    Collect, IncreaseHappiness, PlayingAction, PlayingActionType, Recruit,
};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;
use server::utils::remove_element_by;
use std::vec;

mod common;

const JSON: JsonTest = JsonTest::new("ai");

#[test]
fn collect_city() {
    let game = &JSON.load_game("collect");
    let p = game.player(0);
    let mut collect = city_collections(game, p, p.get_city(Position::from_offset("C2")));
    for c in &mut collect {
        c.collections.sort_by_key(|x| x.position);
    }
    let got = collect.into_iter().map(|c| c.collections).collect_vec();
    assert_eq!(got.len(), 4);
    assert_eq!(
        got[0],
        vec![
            PositionCollection::new(Position::from_offset("C2"), ResourcePile::wood(1)),
            PositionCollection::new(Position::from_offset("C3"), ResourcePile::gold(1)),
        ]
    )
}

#[test]
fn all_actions() {
    let game = &JSON.load_game("start");
    let mut all = get_available_actions(game);
    let (_, advances) = remove_element_by(&mut all, |(t, _)| {
        matches!(t, ActionType::Playing(PlayingActionType::Advance))
    })
    .unwrap();
    assert_eq!(advances.len(), 13);

    assert_eq!(
        all,
        vec![
            (
                ActionType::Playing(PlayingActionType::Recruit),
                vec![Action::Playing(PlayingAction::Recruit(Recruit::new(
                    &Units::new(1, 0, 0, 0, 0, 0),
                    Position::from_offset("D8"),
                    ResourcePile::food(2)
                )))]
            ),
            (
                ActionType::Playing(PlayingActionType::Collect),
                vec![
                    Action::Playing(PlayingAction::Collect(Collect::new(
                        Position::from_offset("D8"),
                        vec![PositionCollection::new(
                            Position::from_offset("E8"),
                            ResourcePile::wood(1)
                        )]
                    ))),
                    Action::Playing(PlayingAction::Collect(Collect::new(
                        Position::from_offset("D8"),
                        vec![PositionCollection::new(
                            Position::from_offset("C8"),
                            ResourcePile::ore(1)
                        )]
                    )))
                ]
            ),
            (
                ActionType::Playing(PlayingActionType::IncreaseHappiness),
                vec![Action::Playing(PlayingAction::IncreaseHappiness(
                    IncreaseHappiness::new(
                        vec![(Position::from_offset("D8"), 1)],
                        ResourcePile::mood_tokens(1)
                    )
                ))]
            ),
            (
                ActionType::Playing(PlayingActionType::ActionCard(0)),
                vec![Action::Playing(PlayingAction::ActionCard(42))]
            ),
        ]
    )
}
