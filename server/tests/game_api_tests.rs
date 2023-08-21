use std::collections::HashMap;

use server::{
    action::Action,
    city::{City, MoodState::*},
    city_pieces::{AvailableCityPieces, Building},
    content::custom_actions::CustomAction::*,
    game::{Game, GameState::*},
    game_api,
    map::Terrain::*,
    playing_actions::PlayingAction::*,
    position::Position,
    resource_pile::ResourcePile,
    unit::UnitType::*,
};

#[test]
fn basic_actions() {
    let mut game = game_api::init(1, String::new());
    let founded_city_position = Position::new(0, 1);
    game.map.tiles = HashMap::from([(founded_city_position, Forest)]);
    let advance_action = Action::Playing(Advance {
        advance: String::from("Math"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute_action(game, advance_action, 0);
    let player = &game.players[0];

    assert_eq!(ResourcePile::culture_tokens(1), player.resources);
    assert_eq!(2, game.actions_left);

    let advance_action = Action::Playing(Advance {
        advance: String::from("Engineering"),
        payment: ResourcePile::empty(),
    });
    let mut game = game_api::execute_action(game, advance_action, 0);
    let player = &game.players[0];

    assert_eq!(
        vec![
            String::from("Farming"),
            String::from("Mining"),
            String::from("Math"),
            String::from("Engineering")
        ],
        player.advances
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

    let construct_action = Action::Playing(Construct {
        city_position,
        city_piece: Building::Observatory,
        payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        port_position: None,
        temple_bonus: None,
    });
    let game = game_api::execute_action(game, construct_action, 0);
    let player = &game.players[0];

    assert_eq!(
        AvailableCityPieces::new(5, 5, 5, 4, 5, 5, 5),
        player.available_buildings
    );

    assert_eq!(
        Some(0),
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .pieces
            .observatory
    );
    assert_eq!(
        2,
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .size()
    );
    assert_eq!(ResourcePile::new(1, 3, 3, 0, 2, 2, 4), player.resources);
    assert_eq!(0, game.actions_left);

    let game = game_api::execute_action(game, Action::Playing(EndTurn), 0);

    assert_eq!(3, game.actions_left);
    assert_eq!(0, game.current_player_index);

    let increase_happiness_action = Action::Playing(IncreaseHappiness {
        happiness_increases: vec![(city_position, 1)],
    });
    let game = game_api::execute_action(game, increase_happiness_action, 0);
    let player = &game.players[0];

    assert_eq!(ResourcePile::new(1, 3, 3, 0, 2, 0, 4), player.resources);
    assert!(matches!(
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .mood_state,
        Happy
    ));
    assert_eq!(2, game.actions_left);

    let construct_wonder_action = Action::Playing(Custom(ConstructWonder {
        city_position,
        wonder: String::from("X"),
        payment: ResourcePile::new(1, 3, 3, 0, 2, 0, 4),
    }));
    let mut game = game_api::execute_action(game, construct_wonder_action, 0);
    let player = &game.players[0];

    assert_eq!(10.0, player.victory_points());
    assert_eq!(ResourcePile::empty(), player.resources);
    assert_eq!(1, player.wonders_build);
    assert_eq!(vec![String::from("X")], player.wonders);
    assert_eq!(
        1,
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .pieces
            .wonders
            .len()
    );
    assert_eq!(
        4,
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .mood_modified_size()
    );
    assert_eq!(1, game.actions_left);

    let tile_position = Position::new(1, 0);
    game.map.tiles.insert(tile_position, Mountain);
    let collect_action = Action::Playing(Collect {
        city_position,
        collections: vec![(tile_position, ResourcePile::ore(1))],
    });
    let game = game_api::execute_action(game, collect_action, 0);
    let player = &game.players[0];
    assert_eq!(ResourcePile::ore(1), player.resources);
    assert!(player
        .get_city(city_position)
        .expect("player should have a city at this position")
        .is_activated());
    assert_eq!(0, game.actions_left);
    let mut game = game_api::execute_action(game, Action::Playing(EndTurn), 0);
    let player = &mut game.players[0];
    player.gain_resources(ResourcePile::food(2));
    let recruit_action = Action::Playing(Recruit {
        units: vec![Settler],
        city_position,
        payment: ResourcePile::food(2),
        leader_index: None,
        replaced_units: Vec::new(),
    });
    let mut game = game_api::execute_action(game, recruit_action, 0);
    let player = &mut game.players[0];
    assert_eq!(1, player.units.len());
    assert_eq!(1, player.next_unit_id);
    assert_eq!(ResourcePile::ore(1), player.resources);
    assert!(player
        .get_city(city_position)
        .expect("The player should have a city at this position")
        .is_activated());

    //todo use movement action here instead
    player.units[0].position = founded_city_position;

    let found_city_action = Action::Playing(FoundCity { settler: 0 });
    let game = game_api::execute_action(game, found_city_action, 0);
    let player = &game.players[0];
    assert_eq!(0, player.units.len());
    assert_eq!(1, player.next_unit_id);
    assert_eq!(4, player.cities.len());
    assert_eq!(
        1,
        player
            .get_city(founded_city_position)
            .expect("The player should have the founded city")
            .size()
    );
}

#[test]
fn cultural_influence() {
    let mut game = game_api::init(2, String::new());
    game.dice_roll_outcomes = vec![6, 4, 5, 3, 7];
    game.current_player_index = 0;
    game.players[0].gain_resources(ResourcePile::culture_tokens(4));
    game.players[1].gain_resources(ResourcePile::culture_tokens(1));
    let city0position = Position::new(0, 0);
    let city1position = Position::new(2, 0);
    assert_eq!(city0position.distance(city1position), 2);
    game.players[0].cities.push(City::new(0, city0position));
    game.players[1].cities.push(City::new(1, city1position));
    game.players[1].construct(&Building::Academy, city1position, None);
    let influence_action = Action::Playing(InfluenceCultureAttempt {
        starting_city_position: city0position,
        target_player_index: 1,
        target_city_position: city1position,
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 0);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    let influence_action = Action::Playing(InfluenceCultureAttempt {
        starting_city_position: city0position,
        target_player_index: 1,
        target_city_position: city1position,
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 0);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(
        game.state,
        CulturalInfluenceResolution {
            roll_boost_cost: 2,
            target_player_index: 1,
            target_city_position: city1position,
            city_piece: Building::Academy
        }
    );
    let influence_resolution_decline_action = Action::CulturalInfluenceResolution(false);
    let game = game_api::execute_action(game, influence_resolution_decline_action, 0);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    assert!(!game.successful_cultural_influence);
    let influence_action = Action::Playing(InfluenceCultureAttempt {
        starting_city_position: city0position,
        target_player_index: 1,
        target_city_position: city1position,
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 0);
    assert!(game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    assert!(game.successful_cultural_influence);
    let game = game_api::execute_action(game, Action::Playing(EndTurn), 0);
    assert_eq!(game.current_player_index, 1);
    let influence_action = Action::Playing(InfluenceCultureAttempt {
        starting_city_position: city1position,
        target_player_index: 1,
        target_city_position: city1position,
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 1);
    assert!(game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    assert!(!game.successful_cultural_influence);
    let influence_action = Action::Playing(InfluenceCultureAttempt {
        starting_city_position: city1position,
        target_player_index: 1,
        target_city_position: city1position,
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 1);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    assert!(game.successful_cultural_influence);
}

fn assert_undo(
    game: &Game,
    can_undo: bool,
    can_redo: bool,
    action_log_len: usize,
    action_log_index: usize,
    undo_limit: usize,
) {
    assert_eq!(can_undo, game.can_undo());
    assert_eq!(can_redo, game.can_redo());
    assert_eq!(action_log_len, game.action_log.len());
    assert_eq!(action_log_index, game.action_log_index);
    assert_eq!(undo_limit, game.undo_limit);
}

fn increase_happiness(game: Game) -> Game {
    let increase_happiness_action = Action::Playing(IncreaseHappiness {
        happiness_increases: vec![(Position::new(0, 0), 1)],
    });
    game_api::execute_action(game, increase_happiness_action, 0)
}

#[test]
fn undo() {
    let mut game = game_api::init(1, String::new());
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
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, false, true, 2, 0, 0);
    assert_eq!(Angry, game.players[0].cities[0].mood_state);
    let game = increase_happiness(game);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Redo, 0);
    assert_undo(&game, true, false, 2, 2, 0);
    assert_eq!(Happy, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, false, true, 2, 0, 0);
    assert_eq!(Angry, game.players[0].cities[0].mood_state);

    let advance_action = Action::Playing(Advance {
        advance: String::from("Math"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute_action(game, advance_action, 0);
    assert_undo(&game, true, false, 1, 1, 0);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, false, true, 1, 0, 0);
    assert_eq!(2, game.players[0].advances.len());
    let advance_action = Action::Playing(Advance {
        advance: String::from("Engineering"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute_action(game, advance_action, 0);
    assert_undo(&game, false, false, 1, 1, 1);
}
