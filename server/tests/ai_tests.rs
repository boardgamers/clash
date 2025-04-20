use crate::common::JsonTest;
use itertools::Itertools;
use server::action::{Action, ActionType};
use server::ai_actions::get_available_actions;
use server::collect::{PositionCollection, city_collections_uncached};
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
    let mut collect = city_collections_uncached(game, p, p.get_city(Position::from_offset("C2")));
    for c in &mut collect {
        c.collections.sort_by_key(|x| x.position);
    }
    assert_eq!(collect.len(), 3);
    let got = collect.into_iter().map(|c| c.total).collect_vec();
    assert_eq!(
        got,
        vec![
            ResourcePile::wood(1) + ResourcePile::food(2),
            ResourcePile::food(1) + ResourcePile::wood(1) + ResourcePile::gold(1),
            ResourcePile::food(1) + ResourcePile::wood(1) + ResourcePile::mood_tokens(1),
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
    assert_eq!(advances.len(), 12);

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
                        )],
                        ResourcePile::wood(1),
                        PlayingActionType::Collect,
                    ))),
                    Action::Playing(PlayingAction::Collect(Collect::new(
                        Position::from_offset("D8"),
                        vec![PositionCollection::new(
                            Position::from_offset("C8"),
                            ResourcePile::ore(1)
                        )],
                        ResourcePile::ore(1),
                        PlayingActionType::Collect,
                    )))
                ]
            ),
            (
                ActionType::Playing(PlayingActionType::IncreaseHappiness),
                vec![Action::Playing(PlayingAction::IncreaseHappiness(
                    IncreaseHappiness::new(
                        vec![(Position::from_offset("D8"), 1)],
                        ResourcePile::mood_tokens(1),
                        PlayingActionType::IncreaseHappiness
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
