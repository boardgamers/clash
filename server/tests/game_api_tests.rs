use crate::common::*;
use itertools::Itertools;
use playing_actions::PlayingActionType;
use server::card::HandCard;
use server::collect::PositionCollection;
use server::content::persistent_events::{EventResponse, SelectedStructure};
use server::game::{CivSetupOption, GameOptions, PatchOption, UndoOption};
use server::game_setup::{GameSetupBuilder, setup_game};
use server::leader::Leader;
use server::structure::Structure;
use server::unit::Units;
use server::wonder::Wonder;
use server::{
    action::Action,
    advance,
    city::{City, MoodState::*},
    city_pieces::Building::*,
    collect, construct, cultural_influence, game_api, happiness,
    map::Terrain::*,
    playing_actions,
    playing_actions::PlayingAction::*,
    position::Position,
    recruit,
    resource_pile::ResourcePile,
};
use std::{collections::HashMap, vec};

mod common;

const JSON: JsonTest = JsonTest::new("base");

#[test]
fn new_game() {
    let mut game = setup_game(
        &GameSetupBuilder::new(2)
            .options(GameOptions {
                civilization: CivSetupOption::ChooseCivilization,
                undo: UndoOption::SamePlayer,
                patch: PatchOption::Standard,
            })
            .build(),
    );
    let player_index = game.current_player_index;
    game = game_api::execute(
        game,
        Action::ChooseCivilization("Vikings".to_string()),
        player_index,
    );
    // should have a water tile now
    JSON.compare_game("new_game", &game);
}

#[test]
fn basic_actions() {
    let mut game = setup_game(
        &GameSetupBuilder::new(1)
            .assigned_civilizations(vec!["Rome".to_string(), "Greece".to_string()])
            .skip_random_map()
            .build(),
    );

    game.wonders_left.retain(|w| *w == Wonder::Pyramids);
    let founded_city_position = Position::new(0, 1);
    game.map.tiles = HashMap::from([(founded_city_position, Forest)]);
    let game = game_api::execute(
        game,
        advance_action(advance::Advance::Math, ResourcePile::food(2)),
        0,
    );
    let player = &game.players[0];

    assert_eq!(ResourcePile::culture_tokens(1), player.resources);
    assert_eq!(2, game.actions_left);

    let mut game = game_api::execute(
        game,
        advance_action(advance::Advance::Engineering, ResourcePile::empty()),
        0,
    );
    let player = &game.players[0];

    assert_eq!(
        vec![
            advance::Advance::Farming,
            advance::Advance::Mining,
            advance::Advance::Engineering,
            advance::Advance::Math,
        ],
        player.advances.iter().collect_vec()
    );
    assert_eq!(ResourcePile::culture_tokens(1), player.resources);
    assert_eq!(1, game.actions_left);

    game.players[0].resources += ResourcePile::new(2, 4, 4, 0, 2, 2, 3);
    let city_position = Position::new(0, 0);
    game.players[0].cities.push(City::new(0, city_position));
    game.players[0].advances.insert(advance::Advance::Rituals);
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 3)));
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 2)));

    let game = game_api::execute(
        game,
        Action::Playing(Construct(construct::Construct::new(
            city_position,
            Observatory,
            ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        ))),
        0,
    );
    let player = &game.players[0];

    assert_eq!(Some(0), player.get_city(city_position).pieces.observatory);
    assert_eq!(2, player.get_city(city_position).size());
    assert_eq!(ResourcePile::new(1, 3, 3, 0, 2, 2, 4), player.resources);
    assert_eq!(0, game.actions_left);

    let game = game_api::execute(game, Action::Playing(EndTurn), 0);

    assert_eq!(3, game.actions_left);
    assert_eq!(0, game.active_player());

    let mut game = game_api::execute(
        game,
        Action::Playing(IncreaseHappiness(happiness::IncreaseHappiness::new(
            vec![(city_position, 1)],
            ResourcePile::mood_tokens(2),
            PlayingActionType::IncreaseHappiness,
        ))),
        0,
    );
    let player = &game.players[0];

    assert_eq!(ResourcePile::new(1, 3, 3, 0, 2, 0, 4), player.resources);
    assert_eq!(player.get_city(city_position).mood_state, Happy);
    assert_eq!(2, game.actions_left);

    game.players[0].resources = ResourcePile::new(3, 3, 6, 0, 1, 0, 5);

    game = game_api::execute(game, Action::Playing(WonderCard(Wonder::Pyramids)), 0);
    game = game_api::execute(
        game,
        payment_response(ResourcePile::new(2, 3, 6, 0, 1, 0, 5)),
        0,
    );
    let player = &game.players[0];

    assert_eq!(12.1, player.victory_points(&game));
    assert_eq!(ResourcePile::food(1), player.resources);
    assert_eq!(vec![Wonder::Pyramids], player.wonders_built);
    assert_eq!(1, player.get_city(city_position).pieces.wonders.len());
    assert_eq!(4, player.get_city(city_position).mood_modified_size(player));
    assert_eq!(1, game.actions_left);

    let tile_position = Position::new(1, 0);
    game.map.tiles.insert(tile_position, Mountain);
    game.map.tiles.insert(city_position, Fertile);
    let collect_action = Action::Playing(Collect(collect::Collect::new(
        city_position,
        vec![PositionCollection::new(tile_position, ResourcePile::ore(1))],
        PlayingActionType::Collect,
    )));
    let game = game_api::execute(game, collect_action, 0);
    let player = &game.players[0];
    assert_eq!(
        ResourcePile::ore(1) + ResourcePile::food(1),
        player.resources
    );
    assert!(
        player
            .try_get_city(city_position)
            .expect("player should have a city this position")
            .is_activated()
    );
    assert_eq!(0, game.actions_left);
    let mut game = game_api::execute(game, Action::Playing(EndTurn), 0);
    let player = &mut game.players[0];
    player.resources += ResourcePile::food(1);
    let recruit_action = Action::Playing(Recruit(recruit::Recruit::new(
        &Units::new(1, 0, 0, 0, 0, None),
        city_position,
        ResourcePile::food(2),
    )));
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

#[test]
fn test_cultural_influence_instant() {
    JSON.test(
        "cultural_influence_instant",
        vec![TestAction::not_undoable(
            1,
            Action::Playing(InfluenceCultureAttempt(
                cultural_influence::InfluenceCultureAttempt::new(
                    SelectedStructure::new(
                        Position::from_offset("C2"),
                        Structure::Building(Fortress),
                    ),
                    PlayingActionType::InfluenceCultureAttempt,
                ),
            )),
        )],
    );
}

#[test]
fn test_cultural_influence() {
    JSON.test(
        "cultural_influence",
        vec![
            TestAction::undoable(1, influence_action()).skip_json(),
            TestAction::not_undoable(1, payment_response(ResourcePile::culture_tokens(1)))
                .skip_json(),
            TestAction::undoable(1, payment_response(ResourcePile::culture_tokens(4))),
        ],
    );
}

#[test]
fn test_found_city() {
    JSON.test(
        "found_city",
        vec![
            TestAction::undoable(0, Action::Playing(FoundCity { settler: 4 })).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(27),
                ])),
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
            Action::Playing(IncreaseHappiness(happiness::IncreaseHappiness::new(
                vec![
                    (Position::from_offset("C2"), 1),
                    (Position::from_offset("B3"), 2),
                ],
                ResourcePile::mood_tokens(5),
                PlayingActionType::IncreaseHappiness,
            ))),
        )],
    );
}

#[test]
fn test_recruit() {
    JSON.test(
        "recruit",
        vec![TestAction::undoable(
            0,
            Action::Playing(Recruit(
                recruit::Recruit::new(
                    &Units::new(1, 1, 0, 0, 0, None),
                    Position::from_offset("A1"),
                    ResourcePile::food(1) + ResourcePile::ore(1) + ResourcePile::gold(2),
                )
                .with_replaced_units(&[4]),
            )),
        )],
    );
}

#[test]
fn test_recruit_leader() {
    JSON.test(
        "recruit_leader",
        vec![TestAction::undoable(
            0,
            Action::Playing(Recruit(recruit::Recruit::new(
                &Units::new(0, 0, 0, 0, 0, Some(Leader::Augustus)),
                Position::from_offset("A1"),
                ResourcePile::mood_tokens(1) + ResourcePile::culture_tokens(1),
            ))),
        )],
    );
}

#[test]
fn test_replace_leader() {
    JSON.test(
        "replace_leader",
        vec![TestAction::undoable(
            0,
            Action::Playing(Recruit(
                recruit::Recruit::new(
                    &Units::new(0, 0, 0, 0, 0, Some(Leader::Augustus)),
                    Position::from_offset("A1"),
                    ResourcePile::mood_tokens(1) + ResourcePile::culture_tokens(1),
                )
                .with_replaced_units(&[10]),
            )),
        )],
    );
}

#[test]
fn test_collect() {
    JSON.test(
        "collect",
        vec![TestAction::undoable(
            0,
            Action::Playing(Collect(collect::Collect::new(
                Position::from_offset("C2"),
                vec![
                    PositionCollection::new(Position::from_offset("B1"), ResourcePile::ore(1)),
                    PositionCollection::new(Position::from_offset("B2"), ResourcePile::wood(1)),
                ],
                PlayingActionType::Collect,
            ))),
        )],
    );
}

#[test]
fn test_construct() {
    JSON.test(
        "construct",
        vec![TestAction::not_undoable(
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
fn test_same_player_undo() {
    JSON.test(
        "same_player_undo",
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
