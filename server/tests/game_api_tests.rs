use server::{
    city::{AvailableBuildings, Building, City, MoodState::*},
    content::custom_actions::CustomAction::*,
    game::{Action, GameState::*},
    game_api,
    hexagon::Position,
    playing_actions::PlayingAction::*,
    resource_pile::ResourcePile,
};

#[test]
fn one_player() {
    let game = game_api::init(1, String::new());
    let advance_action = Action::PlayingAction(Advance {
        advance: String::from("Math"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute_action(game, advance_action, 0);
    let player = &game.players[0];

    assert_eq!(&ResourcePile::culture_tokens(1), player.resources());
    assert_eq!(2, game.actions_left);

    let advance_action = Action::PlayingAction(Advance {
        advance: String::from("Engineering"),
        payment: ResourcePile::empty(),
    });
    let mut game = game_api::execute_action(game, advance_action, 0);
    let player = &game.players[0];

    assert_eq!(
        vec![String::from("Math"), String::from("Engineering")],
        player.advances
    );
    assert_eq!(&ResourcePile::culture_tokens(1), player.resources());
    assert_eq!(1, game.actions_left);

    game.players[0].gain_resources(ResourcePile::new(2, 4, 4, 0, 2, 2, 3));
    let city_position = Position::new(0, 0);
    game.players[0]
        .cities
        .push(City::new(0, city_position.clone()));
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 1)));
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 2)));

    let construct_action = Action::PlayingAction(Construct {
        city_position: city_position.clone(),
        city_piece: Building::Observatory,
        payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        temple_bonus: None,
    });
    let game = game_api::execute_action(game, construct_action, 0);
    let player = &game.players[0];

    assert_eq!(
        AvailableBuildings::new(5, 5, 5, 4, 5, 5, 5),
        player.available_buildings
    );

    assert_eq!(
        Some(0),
        player
            .get_city(&city_position)
            .expect("player should have a city at this position")
            .city_pieces
            .observatory
    );
    assert_eq!(
        2,
        player
            .get_city(&city_position)
            .expect("player should have a city at this position")
            .size()
    );
    assert_eq!(&ResourcePile::new(1, 3, 3, 0, 2, 2, 4), player.resources());
    assert_eq!(0, game.actions_left);

    let game = game_api::execute_action(game, Action::PlayingAction(EndTurn), 0);

    assert_eq!(3, game.actions_left);
    assert_eq!(0, game.current_player_index);

    let increase_happiness_action = Action::PlayingAction(IncreaseHappiness {
        happiness_increases: vec![(city_position.clone(), 1)],
    });
    let game = game_api::execute_action(game, increase_happiness_action, 0);
    let player = &game.players[0];

    assert_eq!(&ResourcePile::new(1, 3, 3, 0, 2, 0, 4), player.resources());
    assert!(matches!(
        player
            .get_city(&city_position)
            .expect("player should have a city at this position")
            .mood_state,
        Happy
    ));
    assert_eq!(2, game.actions_left);

    let construct_wonder_action = Action::PlayingAction(Custom(ConstructWonder {
        city_position: city_position.clone(),
        wonder: String::from("test"),
        payment: ResourcePile::new(1, 3, 3, 0, 2, 0, 4),
    }));
    let game = game_api::execute_action(game, construct_wonder_action, 0);
    let player = &game.players[0];

    assert_eq!(9.0, player.victory_points());
    assert_eq!(&ResourcePile::empty(), player.resources());
    assert_eq!(1, player.wonders_build);
    assert_eq!(vec![String::from("test")], player.wonders);
    assert_eq!(
        1,
        player
            .get_city(&city_position)
            .expect("player should have a city at this position")
            .city_pieces
            .wonders
            .len()
    );
    assert_eq!(1, game.actions_left);
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
    assert_eq!(city0position.distance(&city1position), 2);
    game.players[0]
        .cities
        .push(City::new(0, city0position.clone()));
    game.players[1]
        .cities
        .push(City::new(1, city1position.clone()));
    game.players[1].construct(&Building::Academy, &city1position);
    let influence_action = Action::PlayingAction(InfluenceCultureAttempt {
        starting_city_position: city0position.clone(),
        target_player_index: 1,
        target_city_position: city1position.clone(),
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 0);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    let influence_action = Action::PlayingAction(InfluenceCultureAttempt {
        starting_city_position: city0position.clone(),
        target_player_index: 1,
        target_city_position: city1position.clone(),
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 0);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(
        game.state,
        CulturalInfluenceResolution {
            roll_boost_cost: 2,
            target_player_index: 1,
            target_city_position: city1position.clone(),
            city_piece: Building::Academy
        }
    );
    let influence_resolution_decline_action = Action::CulturalInfluenceResolutionAction(false);
    let game = game_api::execute_action(game, influence_resolution_decline_action, 0);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    assert!(!game.successful_cultural_influence);
    let influence_action = Action::PlayingAction(InfluenceCultureAttempt {
        starting_city_position: city0position,
        target_player_index: 1,
        target_city_position: city1position.clone(),
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 0);
    assert!(game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    assert!(game.successful_cultural_influence);
    let game = game_api::execute_action(game, Action::PlayingAction(EndTurn), 0);
    assert_eq!(game.current_player_index, 1);
    let influence_action = Action::PlayingAction(InfluenceCultureAttempt {
        starting_city_position: city1position.clone(),
        target_player_index: 1,
        target_city_position: city1position.clone(),
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 1);
    assert!(game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    assert!(!game.successful_cultural_influence);
    let influence_action = Action::PlayingAction(InfluenceCultureAttempt {
        starting_city_position: city1position.clone(),
        target_player_index: 1,
        target_city_position: city1position,
        city_piece: Building::Academy,
    });
    let game = game_api::execute_action(game, influence_action, 1);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(game.state, Playing);
    assert!(game.successful_cultural_influence);
}
