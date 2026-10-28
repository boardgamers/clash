use super::*;
use crate::action::Action;
use crate::cache::Cache;
use crate::events::EventOrigin;
use crate::game::GameContext;
use crate::game_data::GameData;
use crate::log::{ActionLogItem, ActionLogRound, current_turn_log_mut};
use crate::playing_actions::PlayingAction;
use crate::wonder::Wonder;

fn game() -> Game {
    let data: GameData = serde_json::from_str(include_str!(
        "../../../tests/test_games/tactics_cards/heavy_resistance.json"
    ))
    .unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.log.last_mut().unwrap().rounds = vec![ActionLogRound {
        round: 3,
        turns: vec![ActionLogTurn {
            turn_type: TurnType::Player(0),
            actions: vec![],
        }],
    }];
    game
}

fn build(game: &mut Game, player: usize) {
    let mut action = ActionLogAction::new(Action::Playing(PlayingAction::EndTurn), player, None, 0);
    action.items.push(ActionLogItem::new(
        player,
        ActionLogEntry::HandCard {
            card: HandCard::Wonder(Wonder::Pyramids),
            from: HandCardLocation::Hand(player),
            to: HandCardLocation::PlayToKeep,
        },
        EventOrigin::Ability("Build Wonder".into()),
        vec![],
    ));
    current_turn_log_mut(game).actions.push(action);
    game.log_index += 1;
}

#[test]
fn only_builder_in_last_round_qualifies_even_with_multiple_wonders() {
    let mut game = game();
    let objective = magnificent_culture();
    let check = objective.status_phase_check.unwrap();
    assert!(!check(&game, game.player(0)));
    build(&mut game, 0);
    assert!(check(&game, game.player(0)));
    assert!(!check(&game, game.player(1)));
    build(&mut game, 0);
    assert!(check(&game, game.player(0)));
    build(&mut game, 1);
    assert!(!check(&game, game.player(0)));
    assert!(!check(&game, game.player(1)));
}

#[test]
fn first_construction_uses_history_not_current_ownership() {
    let mut game = game();
    build(&mut game, 0);
    let builders = |g: &Game| {
        wonder_builders(
            g.log
                .iter()
                .flat_map(|a| &a.rounds)
                .flat_map(|r| &r.turns)
                .flat_map(|t| &t.actions),
        )
    };
    assert_eq!(builders(&game), [0]);
    // No Wonder remains on the board; its construction still counts.
    assert!(game.players.iter().all(|p| p.wonders_built.is_empty()));
    build(&mut game, 1);
    assert_eq!(builders(&game), [0, 1]);
}

#[test]
fn construction_event_offers_only_the_first_wonder() {
    use crate::content::persistent_events::{EventResponse, PersistentEventType};
    use crate::resource_pile::ResourcePile;
    for earlier_builder in [None, Some(0), Some(1)] {
        let data: GameData = serde_json::from_str(include_str!(
            "../../../tests/test_games/wonders/pyramids.json"
        ))
        .unwrap();
        let cache = Cache::new(&data.options);
        let mut game = Game::from_data(data, cache, GameContext::Play);
        if let Some(player) = earlier_builder {
            build(&mut game, player);
        }
        let game = crate::game_api::execute(
            game,
            Action::Playing(PlayingAction::WonderCard(Wonder::Pyramids)),
            0,
        );
        let game = crate::game_api::execute(
            game,
            Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                2, 3, 6, 0, 1, 0, 5,
            )])),
            0,
        );
        let offered = game.events.iter().any(|event| matches!(&event.event_type,
            PersistentEventType::SelectObjectives(info) if info.shown_objective.as_deref() == Some("Magnificent Culture")));
        assert_eq!(offered, earlier_builder.is_none());
    }
}
