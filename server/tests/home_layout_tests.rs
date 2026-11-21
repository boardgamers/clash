use serde_json::Value;
use server::{
    action::{Action, try_execute_action},
    cache::Cache,
    game::{CivSetupOption, Game, GameContext, GameOptions},
    game_data::GameData,
    game_setup::{GameSetupBuilder, setup_game},
    map::HomeLayout,
    playing_actions::PlayingAction,
};

fn saved(game: &Game) -> Value {
    serde_json::to_value(game.cloned_data()).unwrap()
}
fn reload(value: Value) -> Game {
    let data: GameData = serde_json::from_value(value).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}
fn replay(value: Value, to: Option<usize>) -> Game {
    server::replay::replay(serde_json::from_value(value).unwrap(), to).unwrap()
}
fn homes(game: &Game) -> Vec<String> {
    game.players
        .iter()
        .filter(|p| p.is_human())
        .map(|p| p.cities[0].position.to_string())
        .collect()
}
fn regions(game: &Game) -> Value {
    server::web_view::view(game, None)["regions"].clone()
}
fn setup(count: usize, layout: HomeLayout, civilization: CivSetupOption) -> Game {
    setup_game(
        &GameSetupBuilder::new(count)
            .seed("clockwise-layout".into())
            .home_layout(layout)
            .options(GameOptions {
                civilization,
                ..GameOptions::default()
            })
            .build(),
    )
}

#[test]
fn new_and_legacy_layout_survive_save_replay_and_playable_branch() {
    for count in 2..=4 {
        for layout in [HomeLayout::Legacy, HomeLayout::ClockwiseV1] {
            let mut game = setup(count, layout, CivSetupOption::Random);
            let expected_homes = homes(&game);
            let expected_regions = regions(&game);
            let initial_length = server::game_api::log_length(&game);
            let player = game.active_player();
            game =
                try_execute_action(game, Action::Playing(PlayingAction::EndTurn), player).unwrap();
            let data = saved(&game);
            if layout == HomeLayout::Legacy {
                assert!(
                    data.get("home_layout").is_none(),
                    "legacy format remains absent"
                );
            } else {
                assert_eq!(data["home_layout"], "ClockwiseV1");
            }
            for restored in [
                reload(data.clone()),
                replay(data.clone(), None),
                replay(data, Some(initial_length)),
            ] {
                assert_eq!(restored.home_layout, layout);
                assert_eq!(homes(&restored), expected_homes);
                assert_eq!(regions(&restored), expected_regions);
                let player = restored.active_player();
                let branch =
                    try_execute_action(restored, Action::Playing(PlayingAction::EndTurn), player)
                        .unwrap();
                let again = replay(saved(&branch), None);
                assert_eq!(again.home_layout, layout);
                assert_eq!(homes(&again), expected_homes);
            }
        }
    }
}

#[test]
fn old_production_history_without_layout_replays_unchanged() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/legacy_ballcourts_history.json")).unwrap();
    assert!(fixture.get("home_layout").is_none());
    let game = replay(fixture, None);
    assert_eq!(game.home_layout, HomeLayout::Legacy);
    assert_eq!(homes(&game), ["D2", "D7"]);
    let again = reload(saved(&game));
    assert_eq!(homes(&again), homes(&game));
    assert_eq!(regions(&again), regions(&game));
}

#[test]
fn default_new_game_is_clockwise_and_custom_map_reload_is_not_inferred() {
    let game = setup_game(&GameSetupBuilder::new(4).seed("custom".into()).build());
    assert_eq!(game.home_layout, HomeLayout::ClockwiseV1);
    let custom = setup_game(&GameSetupBuilder::new(4).skip_random_map().build());
    assert!(custom.map.tiles.is_empty());
    let mut data = saved(&custom);
    data.as_object_mut().unwrap().remove("home_layout");
    let old_custom = reload(data);
    assert_eq!(old_custom.home_layout, HomeLayout::Legacy);
    assert!(old_custom.map.tiles.is_empty());
    assert_eq!(regions(&old_custom), serde_json::json!([]));
}

#[test]
fn draft_and_choose_keep_civilization_deal_rng_turns_and_home_metadata() {
    for count in 2..=4 {
        for option in [
            CivSetupOption::Random,
            CivSetupOption::ChooseCivilization,
            CivSetupOption::DraftThree,
        ] {
            let mut old = setup(count, HomeLayout::Legacy, option.clone());
            let mut new = setup(count, HomeLayout::ClockwiseV1, option.clone());
            assert_eq!(old.starting_player_index, new.starting_player_index);
            assert_eq!(old.current_player_index, new.current_player_index);
            assert_eq!(old.rng.seed, new.rng.seed);
            if option == CivSetupOption::DraftThree {
                assert!(new.map.tiles.is_empty());
                let offers = old.civilization_draft.as_ref().unwrap().offers.clone();
                assert_eq!(offers, new.civilization_draft.as_ref().unwrap().offers);
                // Reload while still drafting: final setup must use the saved layout.
                new = reload(saved(&new));
                for (player, offer) in offers.iter().enumerate() {
                    let action = Action::ChooseCivilization(offer[0].clone());
                    old = try_execute_action(old, action.clone(), player).unwrap();
                    new = try_execute_action(new, action, player).unwrap();
                }
            } else if option == CivSetupOption::ChooseCivilization {
                let choices = ["Rome", "Greece", "Egypt", "China"];
                for _ in 0..count {
                    let player = old.active_player();
                    let action = Action::ChooseCivilization(choices[player].into());
                    old = try_execute_action(old, action.clone(), player).unwrap();
                    new = try_execute_action(new, action, player).unwrap();
                }
            }
            assert_eq!(old.rng.seed, new.rng.seed);
            assert_eq!(old.current_player_index, new.current_player_index);
            for player in 0..count {
                assert_eq!(
                    old.player(player).civilization.name,
                    new.player(player).civilization.name
                );
                let home = new.player(player).cities[0].position;
                let region = regions(&new)[player].clone();
                assert_eq!(region["homePlayer"], player);
                assert!(
                    region["tiles"]
                        .as_array()
                        .unwrap()
                        .contains(&serde_json::json!(home))
                );
            }
            assert_eq!(homes(&replay(saved(&new), None)), homes(&new));
        }
    }
}
