use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{PersistentEventState, PersistentEventType};
use crate::game::{Game, GameContext, GameState};
use crate::game_setup::{GameSetupBuilder, setup_game};
use crate::resource_pile::ResourcePile;

fn reload(game: &Game) -> Game {
    let data = serde_json::from_str(&serde_json::to_string(&game.cloned_data()).unwrap()).unwrap();
    Game::from_data(data, game.cache.clone(), GameContext::Play)
}

fn banking() -> Game {
    let mut game = setup_game(
        &GameSetupBuilder::new(4)
            .seed("withdraw-banking".into())
            .assigned_civilizations(vec![
                "Greece".into(),
                "China".into(),
                "Persia".into(),
                "Rome".into(),
            ])
            .build(),
    );
    for player in &mut game.players {
        player.objective_cards.clear();
    }
    game.starting_player_index = 0;
    game.current_player_index = 2;
    game.age = 4;
    game.round = 2;
    game.player_mut(2)
        .advances
        .insert(crate::advance::Advance::Currency);
    game.player_mut(2).resources = ResourcePile::gold(1);
    game = reload(&game);
    game.start_turn();
    assert_eq!(game.events.len(), 1);
    assert_eq!(game.active_player(), 2);
    // A real successor turn-start effect proves that start_turn is not skipped
    // or duplicated (including after serialization and a repeated notification).
    game.permanent_effects
        .push(PermanentEffect::AssassinationLoseAction(3));
    game
}

fn snapshot(game: &Game) -> String {
    serde_json::to_string(&game.cloned_data()).unwrap()
}

#[test]
fn withdrawal_banking_starts_successor_once_and_preserves_seats() {
    let mut game = banking();
    let seats = game.players.len();
    game.player_mut(2)
        .event_info
        .insert("personal".into(), "stale".into());
    let resources = game.player(2).resources.clone();
    game.drop_player(2);
    assert_eq!(game.player(2).resources, resources);
    assert!(game.player(2).event_info.is_empty());
    assert!(game.events.is_empty());
    assert_eq!(game.current_player_index, 3);
    assert_eq!(game.actions_left, 2);
    assert_eq!(game.round, 2);
    assert_eq!(game.players.len(), seats);
    let saved = snapshot(&game);
    game.drop_player(2);
    assert_eq!(snapshot(&game), saved);
    game = reload(&game);
    assert_eq!(snapshot(&game), saved);
    game.next_turn();
    assert_eq!(game.current_player_index, 0);
    assert_eq!(game.round, 3);
}

#[test]
fn withdrawal_banking_legacy_advanced_pointer_recovers_once() {
    let mut old = banking();
    old.dropped_players.push(2);
    old.current_player_index = 3;
    let mut game = reload(&old);
    assert_eq!(game.current_player_index, 3);
    assert!(game.events.is_empty());
    assert_eq!(game.actions_left, 2);
    assert_eq!(game.player(2).resources, ResourcePile::gold(1));
    let saved = snapshot(&game);
    assert_eq!(snapshot(&reload(&game)), saved);
    game.drop_player(2);
    assert_eq!(snapshot(&game), saved);
    assert_eq!(snapshot(&reload(&old)), saved);
}

#[test]
fn withdrawal_opponent_leaves_current_banking_intact() {
    let mut game = banking();
    let events = game.events.clone();
    game.drop_player(1);
    assert_eq!(game.events, events);
    assert_eq!(game.active_player(), 2);
    assert_eq!(game.current_player_index, 2);
}

#[test]
fn withdrawal_banking_round_boundary_and_first_seat() {
    for first in [2, 3] {
        let mut game = banking();
        game.starting_player_index = first;
        game.drop_player(2);
        assert_eq!(game.current_player_index, 3);
        assert_eq!(game.starting_player_index, 3);
        assert_eq!(game.round, if first == 2 { 2 } else { 3 });
        assert_eq!(game.actions_left, 2);
        assert!(game.events.is_empty());
    }
}

#[test]
fn withdrawal_does_not_discard_nested_or_other_handlers() {
    let mut game = banking();
    game.events.push(PersistentEventState::new(
        2,
        PersistentEventType::ChooseActionCard,
        None,
    ));
    let events = game.events.clone();
    game.drop_player(2);
    assert_eq!(game.events, events);
    assert_eq!(reload(&game).events, events);

    let mut game = banking();
    game.events[0].player.handler.as_mut().unwrap().origin =
        crate::events::EventOrigin::Ability("Not Banking".into());
    let events = game.events.clone();
    game.drop_player(2);
    assert_eq!(game.events, events);
    assert_eq!(reload(&game).events, events);
}

#[test]
fn withdrawal_last_survivor_still_finishes_pending_banking() {
    let mut game = banking();
    game.drop_player(0);
    game.drop_player(1);
    game.drop_player(2);
    assert!(matches!(game.state, GameState::Finished));
    assert!(game.events.is_empty());
    assert_eq!(game.current_player_index, 3);
}

#[test]
fn withdrawal_legacy_does_not_restart_an_already_started_successor() {
    let mut old = banking();
    old.dropped_players.push(2);
    old.current_player_index = 3;
    crate::log::add_turn_log(&mut old, crate::log::TurnType::Player(3));
    let restored = reload(&old);
    assert_eq!(restored.events, old.events);
    assert_eq!(snapshot(&restored), snapshot(&old));
}
