use crate::common::*;
use server::collect::PositionCollection;
use server::content::custom_phase_actions::{EventResponse, SelectedStructure, Structure};
use server::log::current_player_turn_log;
use server::unit::Units;
use server::{
    action::Action,
    city::{City, MoodState::*},
    city_pieces::Building::*,
    construct,
    game::Game,
    game_api,
    map::Terrain::*,
    playing_actions,
    playing_actions::PlayingAction::*,
    position::Position,
    resource_pile::ResourcePile,
};
use std::{collections::HashMap, vec};

mod common;

const JSON: JsonTest = JsonTest::new("base");

#[test]
fn new_game() {
    let seed = String::new();
    let game = Game::new(2, seed, true);
    JSON.compare_game("new_game", &game);
}

#[test]
fn basic_actions() {
    let seed = String::new();
    let mut game = Game::new(1, seed, false);

    game.wonders_left.retain(|w| w == "Pyramids");
    let founded_city_position = Position::new(0, 1);
    game.map.tiles = HashMap::from([(founded_city_position, Forest)]);
    let advance_action = Action::Playing(Advance {
        advance: String::from("Math"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute(game, advance_action, 0);
    let player = &game.players[0];

    assert_eq!(ResourcePile::culture_tokens(1), player.resources);
    assert_eq!(2, game.actions_left);

    let advance_action = Action::Playing(Advance {
        advance: String::from("Engineering"),
        payment: ResourcePile::empty(),
    });
    let mut game = game_api::execute(game, advance_action, 0);
    let player = &game.players[0];

    assert_eq!(
        vec![
            String::from("Farming"),
            String::from("Mining"),
            String::from("Math"),
            String::from("Engineering")
        ],
        player
            .advances
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<String>>()
    );
    assert_eq!(ResourcePile::culture_tokens(1), player.resources);
    assert_eq!(1, game.actions_left);

    game.players[0].gain_resources(ResourcePile::new(2, 4, 4, 0, 2, 2, 3));
    let city_position = Position::new(0, 0);
    game.players[0].cities.push(City::new(0, city_position));
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 3)));
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 2)));

    let construct_action = Action::Playing(Construct(construct::Construct::new(
        city_position,
        Observatory,
        ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
    )));
    let game = game_api::execute(game, construct_action, 0);
    let player = &game.players[0];

    assert_eq!(Some(0), player.get_city(city_position).pieces.observatory);
    assert_eq!(2, player.get_city(city_position).size());
    assert_eq!(ResourcePile::new(1, 3, 3, 0, 2, 2, 4), player.resources);
    assert_eq!(0, game.actions_left);

    let game = game_api::execute(game, Action::Playing(EndTurn), 0);

    assert_eq!(3, game.actions_left);
    assert_eq!(0, game.active_player());

    let increase_happiness_action =
        Action::Playing(IncreaseHappiness(playing_actions::IncreaseHappiness {
            happiness_increases: vec![(city_position, 1)],
            payment: ResourcePile::mood_tokens(2),
        }));
    let mut game = game_api::execute(game, increase_happiness_action, 0);
    let player = &game.players[0];

    assert_eq!(ResourcePile::new(1, 3, 3, 0, 2, 0, 4), player.resources);
    assert!(matches!(player.get_city(city_position).mood_state, Happy));
    assert_eq!(2, game.actions_left);

    game = game_api::execute(game, Action::Playing(WonderCard("Pyramids".to_string())), 0);
    game = game_api::execute(
        game,
        Action::Response(EventResponse::Payment(vec![ResourcePile::new(
            0, 3, 3, 0, 2, 0, 4,
        )])),
        0,
    );
    let player = &game.players[0];

    assert_eq!(10.0, player.victory_points(&game));
    assert_eq!(ResourcePile::food(1), player.resources);
    assert_eq!(vec![String::from("Pyramids")], player.wonders_build);
    assert_eq!(1, player.get_city(city_position).pieces.wonders.len());
    assert_eq!(4, player.get_city(city_position).mood_modified_size(player));
    assert_eq!(1, game.actions_left);

    let tile_position = Position::new(1, 0);
    game.map.tiles.insert(tile_position, Mountain);
    game.map.tiles.insert(city_position, Fertile);
    let collect_action = Action::Playing(Collect(playing_actions::Collect {
        city_position,
        collections: vec![PositionCollection::new(tile_position, ResourcePile::ore(1))],
    }));
    let game = game_api::execute(game, collect_action, 0);
    let player = &game.players[0];
    assert_eq!(
        ResourcePile::ore(1) + ResourcePile::food(1),
        player.resources
    );
    assert!(player
        .try_get_city(city_position)
        .expect("player should have a city at this position")
        .is_activated());
    assert_eq!(0, game.actions_left);
    let mut game = game_api::execute(game, Action::Playing(EndTurn), 0);
    let player = &mut game.players[0];
    player.gain_resources(ResourcePile::food(1));
    let recruit_action = Action::Playing(Recruit(playing_actions::Recruit {
        units: Units::new(1, 0, 0, 0, 0, 0),
        city_position,
        payment: ResourcePile::food(2),
        leader_name: None,
        replaced_units: Vec::new(),
    }));
    let mut game = game_api::execute(game, recruit_action, 0);
    let player = &mut game.players[0];
    assert_eq!(1, player.units.len());
    assert_eq!(1, player.next_unit_id);
    assert_eq!(ResourcePile::ore(1), player.resources);
    assert!(player.get_city(city_position).is_activated());

    let movement_action = move_action(vec![0], founded_city_position);
    let game = game_api::execute(game, movement_action, 0);
    // move stopped automatically - no more movable units left
    let player = &game.players[0];
    assert_eq!(founded_city_position, player.units[0].position);

    let found_city_action = Action::Playing(FoundCity { settler: 0 });
    let game = game_api::execute(game, found_city_action, 0);
    let player = &game.players[0];
    assert_eq!(0, player.units.len());
    assert_eq!(1, player.next_unit_id);
    assert_eq!(4, player.cities.len());
    assert_eq!(1, player.get_city(founded_city_position).size());
}

fn assert_undo(
    game: &Game,
    can_undo: bool,
    can_redo: bool,
    action_log_len: usize,
    action_log_index: usize,
    undo_limit: usize,
) {
    assert_eq!(can_undo, game.can_undo(), "can_undo");
    assert_eq!(can_redo, game.can_redo(), "can_redo");
    assert_eq!(
        action_log_len,
        current_player_turn_log(game).items.len(),
        "action_log_len"
    );
    assert_eq!(action_log_index, game.action_log_index, "action_log_index");
    assert_eq!(undo_limit, game.undo_limit, "undo_limit");
}

fn increase_happiness(game: Game) -> Game {
    let increase_happiness_action =
        Action::Playing(IncreaseHappiness(playing_actions::IncreaseHappiness {
            happiness_increases: vec![(Position::new(0, 0), 1)],
            payment: ResourcePile::mood_tokens(1),
        }));
    game_api::execute(game, increase_happiness_action, 0)
}

#[test]
fn undo() {
    let mut game = Game::new(1, String::new(), false);
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 0)));
    game.players[0].gain_resources(ResourcePile::mood_tokens(2));
    game.players[0].cities[0].decrease_mood_state();

    assert_undo(&game, false, false, 0, 0, 0);
    assert_eq!(Angry, game.players[0].cities[0].mood_state);

    let game = increase_happiness(game);
    assert_undo(&game, true, false, 1, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);

    let game = increase_happiness(game);
    assert_undo(&game, true, false, 2, 2, 0);
    assert_eq!(Happy, game.players[0].cities[0].mood_state);

    let game = game_api::execute(game, Action::Undo, 0);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);

    let game = game_api::execute(game, Action::Undo, 0);
    assert_undo(&game, false, true, 2, 0, 0);
    assert_eq!(Angry, game.players[0].cities[0].mood_state);

    let game = game_api::execute(game, Action::Redo, 0);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);

    let game = game_api::execute(game, Action::Redo, 0);
    assert_undo(&game, true, false, 2, 2, 0);
    assert_eq!(Happy, game.players[0].cities[0].mood_state);

    let game = game_api::execute(game, Action::Undo, 0);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);

    let game = game_api::execute(game, Action::Undo, 0);
    assert_undo(&game, false, true, 2, 0, 0);
    assert_eq!(Angry, game.players[0].cities[0].mood_state);

    let advance_action = Action::Playing(Advance {
        advance: String::from("Math"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute(game, advance_action, 0);
    assert_undo(&game, true, false, 1, 1, 0);
    let game = game_api::execute(game, Action::Undo, 0);
    assert_undo(&game, false, true, 1, 0, 0);
    assert_eq!(2, game.players[0].advances.len());
    let advance_action = Action::Playing(Advance {
        advance: String::from("Engineering"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute(game, advance_action, 0);
    assert_undo(&game, false, false, 1, 1, 1);
}

#[test]
fn test_cultural_influence_instant() {
    JSON.test(
        "cultural_influence_instant",
        vec![TestAction::not_undoable(
            1,
            Action::Playing(InfluenceCultureAttempt(SelectedStructure::new(
                Position::from_offset("C2"),
                Structure::Building(Fortress),
            ))),
        )],
    );
}

#[test]
fn test_cultural_influence() {
    JSON.test(
        "cultural_influence",
        vec![
            TestAction::not_undoable(1, influence_action()),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    4,
                )])),
            ),
        ],
    );
}

#[test]
fn test_found_city() {
    JSON.test(
        "found_city",
        vec![TestAction::undoable(
            0,
            Action::Playing(FoundCity { settler: 4 }),
        )],
    );
}

#[test]
fn test_wonder() {
    JSON.test(
        "wonder",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard("Pyramids".to_string())))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                    2, 3, 3, 0, 0, 0, 4,
                )])),
            ),
        ],
    );
}

#[test]
fn test_increase_happiness() {
    JSON.test(
        "increase_happiness",
        vec![TestAction::undoable(
            0,
            Action::Playing(IncreaseHappiness(playing_actions::IncreaseHappiness {
                happiness_increases: vec![
                    (Position::from_offset("C2"), 1),
                    (Position::from_offset("B3"), 2),
                ],
                payment: ResourcePile::mood_tokens(5),
            })),
        )],
    );
}

#[test]
fn test_recruit() {
    JSON.test(
        "recruit",
        vec![TestAction::undoable(
            0,
            Action::Playing(Recruit(playing_actions::Recruit {
                units: Units::new(1, 1, 0, 0, 0, 0),
                city_position: Position::from_offset("A1"),
                payment: ResourcePile::food(1) + ResourcePile::ore(1) + ResourcePile::gold(2),
                leader_name: None,
                replaced_units: vec![4],
            })),
        )],
    );
}

#[test]
fn test_recruit_leader() {
    JSON.test(
        "recruit_leader",
        vec![TestAction::undoable(
            0,
            Action::Playing(Recruit(playing_actions::Recruit {
                units: Units::new(0, 0, 0, 0, 0, 1),
                city_position: Position::from_offset("A1"),
                payment: ResourcePile::mood_tokens(1) + ResourcePile::culture_tokens(1),
                leader_name: Some("Alexander".to_string()),
                replaced_units: vec![],
            })),
        )],
    );
}

#[test]
fn test_replace_leader() {
    JSON.test(
        "replace_leader",
        vec![TestAction::undoable(
            0,
            Action::Playing(Recruit(playing_actions::Recruit {
                units: Units::new(0, 0, 0, 0, 0, 1),
                city_position: Position::from_offset("A1"),
                payment: ResourcePile::mood_tokens(1) + ResourcePile::culture_tokens(1),
                leader_name: Some("Kleopatra".to_string()),
                replaced_units: vec![10],
            })),
        )],
    );
}

#[test]
fn test_collect() {
    JSON.test(
        "collect",
        vec![TestAction::undoable(
            0,
            Action::Playing(Collect(playing_actions::Collect {
                city_position: Position::from_offset("C2"),
                collections: vec![
                    PositionCollection::new(Position::from_offset("B1"), ResourcePile::ore(1)),
                    PositionCollection::new(Position::from_offset("B2"), ResourcePile::wood(1)),
                ],
            })),
        )],
    );
}

#[test]
fn test_construct() {
    JSON.test(
        "construct",
        vec![TestAction::undoable(
            0,
            Action::Playing(Construct(construct::Construct::new(
                Position::from_offset("C2"),
                Observatory,
                ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
            ))),
        )],
    );
}

#[test]
fn test_construct_port() {
    JSON.test(
        "construct_port",
        vec![TestAction::undoable(
            0,
            Action::Playing(Construct(
                construct::Construct::new(
                    Position::from_offset("A1"),
                    Port,
                    ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
                )
                .with_port_position(Some(Position::from_offset("A2"))),
            )),
        )],
    );
}
