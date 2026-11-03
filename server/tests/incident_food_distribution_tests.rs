//! Mandatory food distributions avoid waste while any opponent can keep the food.
use std::panic::{AssertUnwindSafe, catch_unwind};

use server::{
    action::Action,
    advance::{Advance, AdvanceAction},
    board_history,
    cache::Cache,
    content::persistent_events::{EventResponse, PersistentEventRequest},
    game::{Game, GameContext},
    game_setup::{GameSetupBuilder, setup_game},
    map::Terrain,
    playing_actions::PlayingAction,
    resource_pile::ResourcePile,
};

const DISTRIBUTIONS: [(u8, u8); 3] = [(11, 1), (14, 2), (16, 3)];

fn reload(game: &Game) -> Game {
    let data = game.cloned_data();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

// Each opponent is (food, food limit). Storage and its limit must agree.
fn ready(incident: u8, opponents: &[(u8, u8)]) -> Game {
    let count = opponents.len() + 1;
    let mut game = setup_game(
        &GameSetupBuilder::new(count)
            .seed("incident-food-distribution".into())
            .assigned_civilizations(
                ["Maya", "Rome", "Greece", "Egypt"][..count]
                    .iter()
                    .map(|name| (*name).into())
                    .collect(),
            )
            .build(),
    );
    game.current_player_index = 0;
    for p in &mut game.players {
        p.action_cards.clear();
        p.objective_cards.clear();
        p.wonder_cards.clear();
        if p.is_human() {
            let (food, limit) = if p.index == 0 {
                (4, 7)
            } else {
                opponents[p.index - 1]
            };
            p.resources = ResourcePile::food(food);
            p.resource_limit.food = limit;
            p.advances = Advance::Farming | Advance::Mining;
            if limit == 7 {
                p.advances.insert(Advance::Storage);
            }
        } else {
            // NPC capacity must neither create targets nor suppress all-full fallback.
            p.resources = ResourcePile::empty();
            p.resource_limit.food = 7;
            p.units.clear();
            p.cities.clear();
        }
    }
    // Calendar permits checking Undo/Redo of the owner's recipient choices.
    game.player_mut(0).advances.insert(Advance::Math);
    game.player_mut(0).advances.insert(Advance::Astronomy);
    game.player_mut(0).incident_tokens = 1;
    game.incidents_left = vec![incident, 29, 30];
    let cities: Vec<_> = game
        .players
        .iter()
        .flat_map(|p| p.cities.iter().map(|c| c.position))
        .collect();
    for (position, terrain) in &mut game.map.tiles {
        if terrain.is_land() && !cities.contains(position) {
            // No unrelated BarbariansSpawn or ExhaustedLand choices.
            *terrain = Terrain::Barren;
        }
    }
    reload(&game)
}

fn play(game: Game, action: Action) -> Game {
    let actor = game.active_player();
    board_history::execute(game, action, actor).unwrap()
}

fn trigger(game: Game) -> Game {
    play(
        game,
        Action::Playing(PlayingAction::Advance(AdvanceAction::new(
            Advance::Irrigation,
            ResourcePile::food(2),
        ))),
    )
}

fn choose(game: Game, recipient: usize) -> Game {
    play(
        game,
        Action::Response(EventResponse::SelectPlayer(recipient)),
    )
}

fn assert_choices(game: &Game, expected: &[usize]) {
    assert_eq!(game.active_player(), 0);
    let PersistentEventRequest::SelectPlayer(request) =
        &game.current_event_handler().unwrap().request
    else {
        panic!("expected a food recipient choice");
    };
    assert_eq!(request.choices, expected);
}

fn assert_finished(game: &Game, incident: u8, amount: u8, opponents: &[u8]) {
    assert!(
        game.events.is_empty(),
        "incident {incident} should be resolved"
    );
    assert_eq!(game.incidents_discarded.last(), Some(&incident));
    assert_eq!(game.player(0).resources.food, 2 + amount);
    assert_eq!(
        game.players
            .iter()
            .filter(|p| p.is_human() && p.index != 0)
            .map(|p| p.resources.food)
            .collect::<Vec<_>>(),
        opponents,
    );
    assert!(
        game.players
            .iter()
            .filter(|p| !p.is_human())
            .all(|p| p.resources.food == 0)
    );
}

#[test]
fn three_and_four_player_choices_use_each_recipients_cap_and_allow_repeat_awards() {
    for (incident, amount) in DISTRIBUTIONS {
        for with_full_opponent in [false, true] {
            let opponents = if with_full_opponent {
                vec![(2, 2), (2, 7), (0, 2)]
            } else {
                vec![(2, 7), (0, 2)]
            };
            let recipients = if with_full_opponent {
                vec![2, 3]
            } else {
                vec![1, 2]
            };
            let recipient = recipients[0];
            let mut game = trigger(ready(incident, &opponents));
            for _ in 0..amount {
                assert_choices(&game, &recipients);
                // An opponent at 2/7 remains eligible, unlike one at 2/2.
                game = choose(game, recipient);
            }
            let expected = if with_full_opponent {
                vec![2, 2 + amount, 0]
            } else {
                vec![2 + amount, 0]
            };
            assert_finished(&game, incident, amount, &expected);
        }
    }
}

#[test]
fn full_opponent_actor_and_npcs_are_rejected_while_opponents_have_room() {
    for (incident, _) in DISTRIBUTIONS {
        let game = trigger(ready(incident, &[(2, 2), (0, 7), (0, 7)]));
        assert_choices(&game, &[2, 3]);
        for invalid in [0, 1, 4, 5] {
            // Native recipient validation is an assertion, not a Result error.
            assert!(catch_unwind(AssertUnwindSafe(|| choose(game.clone(), invalid))).is_err());
        }
    }
}

#[test]
fn a_sole_opponent_with_room_receives_every_food_automatically() {
    for (incident, amount) in DISTRIBUTIONS {
        let game = trigger(ready(incident, &[(2, 2), (0, 7)]));
        assert_finished(&game, incident, amount, &[2, amount]);
    }
}

#[test]
fn choices_shrink_as_recipients_fill_and_survive_undo_redo_reload() {
    let choosing = trigger(ready(16, &[(1, 2), (1, 2), (0, 7)]));
    assert_choices(&choosing, &[1, 2, 3]);
    let after_first = choose(choosing.clone(), 1);
    assert_choices(&after_first, &[2, 3]);
    assert_eq!(after_first.player(1).resources.food, 2);
    assert!(after_first.can_undo());

    let undone = play(reload(&after_first), Action::Undo);
    assert_choices(&undone, &[1, 2, 3]);
    assert_eq!(undone.player(1).resources.food, 1);
    let redone = play(reload(&undone), Action::Redo);
    assert_choices(&redone, &[2, 3]);
    assert_eq!(redone.player(1).resources.food, 2);
    assert_eq!(redone.incidents_left, after_first.incidents_left);
    assert_eq!(redone.incidents_discarded, after_first.incidents_discarded);

    // Filling player 2 leaves only player 3, so the last food is automatic.
    let resolved = choose(redone, 2);
    assert_finished(&resolved, 16, 3, &[2, 2, 1]);
    let undone = play(reload(&resolved), Action::Undo);
    assert_choices(&undone, &[2, 3]);
    assert_eq!(undone.player(2).resources.food, 1);
    assert_eq!(undone.player(3).resources.food, 0);
    assert_finished(&play(undone, Action::Redo), 16, 3, &[2, 2, 1]);
}

#[test]
fn consuming_the_last_capacity_enables_unavoidable_waste_for_remaining_food() {
    for (incident, amount) in DISTRIBUTIONS {
        let mut game = trigger(ready(incident, &[(1, 2), (2, 2)]));
        assert_eq!(
            game.player(1).resources.food,
            2,
            "first food must be kept automatically"
        );
        for _ in 1..amount {
            assert_choices(&game, &[1, 2]);
            game = choose(game, 2);
        }
        assert_finished(&game, incident, amount, &[2, 2]);
    }
}

#[test]
fn all_full_fallback_ignores_the_actors_and_npcs_unused_capacity() {
    for (incident, amount) in DISTRIBUTIONS {
        let mut game = trigger(ready(incident, &[(2, 2), (7, 7), (2, 2)]));
        assert!(game.player(0).resources.food < game.player(0).resource_limit.food);
        for _ in 0..amount {
            assert_choices(&game, &[1, 2, 3]);
            game = choose(game, 1);
        }
        assert_finished(&game, incident, amount, &[2, 7, 2]);
    }
}

#[test]
fn two_player_singleton_fallback_keeps_what_fits_then_wastes_the_rest() {
    for (incident, amount) in DISTRIBUTIONS {
        for food in [1, 2] {
            let game = trigger(ready(incident, &[(food, 2)]));
            assert_finished(&game, incident, amount, &[2]);
        }
    }
}

#[test]
fn fixed_rewards_to_every_player_still_waste_excess_without_redistribution() {
    for (incident, amount) in [(9, 1), (13, 2), (15, 3)] {
        let game = trigger(ready(incident, &[(2, 2), (6, 7), (0, 7)]));
        assert_finished(&game, incident, amount, &[2, 7, amount]);
    }
}
