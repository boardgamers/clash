#[cfg(not(target_arch = "wasm32"))]
use crate::common::JsonTest;
use server::collect::Collect;
use server::happiness::IncreaseHappiness;
use server::recruit::Recruit;

mod common;

#[cfg(not(target_arch = "wasm32"))]
const JSON: JsonTest = JsonTest::new("ai");

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn all_actions() {
    use server::action::{Action, ActionType};
    use server::ai_actions::AiActions;
    use server::collect::PositionCollection;
    use server::playing_actions::{PlayingAction, PlayingActionType};
    use server::position::Position;
    use server::resource_pile::ResourcePile;
    use server::unit::Units;
    use server::utils::remove_element_by;
    use std::vec;

    let game = &JSON.load_game("start");
    let mut actions = AiActions::new();
    let mut all = actions.get_available_actions(game);
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
                    &Units::new(1, 0, 0, 0, 0, None),
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
                            Position::from_offset("C8"),
                            ResourcePile::ore(1)
                        )],
                        PlayingActionType::Collect,
                    ))),
                    Action::Playing(PlayingAction::Collect(Collect::new(
                        Position::from_offset("D8"),
                        vec![PositionCollection::new(
                            Position::from_offset("E8"),
                            ResourcePile::wood(1)
                        )],
                        PlayingActionType::Collect,
                    ))),
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
