use server::{
    city::{BuildingData, City, MoodState::*},
    content::custom_actions::CustomAction::*,
    game::Game,
    game_api,
    hexagon::Position,
    playing_actions::PlayingAction::*,
    resource_pile::ResourcePile,
};

#[actix_rt::test]
async fn one_player() {
    let game = game_api::init(
        1,
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    )
    .await;
    let advance_action = serde_json::to_string(&Advance {
        advance: String::from("Math"),
        payment: ResourcePile::food(2),
    })
    .expect("advance should be a valid action");
    let game = game_api::execute_action(game, advance_action, 0);
    let game = Game::from_json(&game);
    let player = &game.players[0];

    assert_eq!(&ResourcePile::culture_tokens(1), player.resources());
    assert_eq!(2, game.actions_left);

    let advance_action = serde_json::to_string(&Advance {
        advance: String::from("Engineering"),
        payment: ResourcePile::empty(),
    })
    .expect("advance should be a valid action");
    let game = game_api::execute_action(game.json(), advance_action, 0);
    let mut game = Game::from_json(&game);
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

    let construct_action = serde_json::to_string(&Construct {
        city_position: city_position.clone(),
        city_piece: BuildingData::Observatory,
        payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        temple_bonus: None,
    })
    .expect("construct should be a valid action");
    let game = game_api::execute_action(game.json(), construct_action, 0);
    let game = Game::from_json(&game);
    let player = &game.players[0];

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

    let game = game_api::execute_action(
        game.json(),
        serde_json::to_string(&EndTurn).expect("ending turn should be allowed"),
        0,
    );
    let game = Game::from_json(&game);

    assert_eq!(3, game.actions_left);
    assert_eq!(0, game.current_player_index);

    let increase_happiness_action = serde_json::to_string(&IncreaseHappiness {
        happiness_increases: vec![(city_position.clone(), 1)],
    })
    .expect("increasing happiness should be a valid action");
    let game = game_api::execute_action(game.json(), increase_happiness_action, 0);
    let game = Game::from_json(&game);
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

    let construct_wonder_action = serde_json::to_string(&Custom(ConstructWonder {
        city_position: city_position.clone(),
        wonder: String::from("test"),
        payment: ResourcePile::new(1, 3, 3, 0, 2, 0, 4),
    }))
    .expect("player should have a city at this position");
    let game = game_api::execute_action(game.json(), construct_wonder_action, 0);
    let game = Game::from_json(&game);
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
