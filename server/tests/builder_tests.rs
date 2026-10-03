use common::{JsonTest, influence_action, move_action};
use server::action::{Action, try_execute_action};
use server::cache::Cache;
use server::game::{CivSetupOption, Game, GameContext, GameLength, GameOptions, GameVariant};
use server::game_api;
use server::movement::possible_move_routes;
use server::player::CostTrigger;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::recruit::{Recruit, recruit_cost_without_replaced};
use server::resource_pile::ResourcePile;
use server::unit::{Unit, UnitType, Units};

mod common;

fn builder(mut game: Game) -> Game {
    game.options.variant = GameVariant::Builder;
    let cache = Cache::new(&game.options);
    Game::from_data(game.cloned_data(), cache, GameContext::Play)
}

fn can_move_to(game: &Game, units: &[u32], destination: Position) -> bool {
    let player = game.player(0);
    possible_move_routes(
        player,
        game,
        units,
        player.get_unit(units[0]).position,
        None,
    )
    .unwrap_or_default()
    .iter()
    .any(|route| route.destination == destination)
}

#[test]
fn builder_blocks_land_sea_and_disembarking_attacks_in_previews_and_execution() {
    for (folder, fixture, units, destination) in [
        (
            "combat",
            "remove_casualties_attacker",
            vec![0, 1, 2, 3],
            "C1",
        ),
        ("combat", "ship_combat", vec![7, 8], "D2"),
        (
            "movement",
            "ship_disembark_capture_empty_city",
            vec![1, 2],
            "B2",
        ),
    ] {
        let game = JsonTest::new(folder).load_game(fixture);
        let destination = Position::from_offset(destination);
        assert!(can_move_to(&game, &units, destination), "{fixture}");
        let game = builder(game);
        assert!(!can_move_to(&game, &units, destination), "{fixture}");
        let error = try_execute_action(game, move_action(units, destination), 0)
            .err()
            .expect("Builder must reject the submitted attack");
        assert!(error.contains("Builder"), "{fixture}: {error}");
    }
}

#[test]
fn builder_also_protects_undefended_settlers() {
    let mut game = JsonTest::new("combat").load_game("remove_casualties_attacker");
    let destination = Position::from_offset("C1");
    game.players[1].cities.clear();
    game.players[1].units = vec![Unit::new(1, destination, UnitType::Settler, 0)];
    assert!(can_move_to(&game, &[0, 1, 2, 3], destination));
    assert!(!can_move_to(&builder(game), &[0, 1, 2, 3], destination));
}

#[test]
fn builder_keeps_attacks_against_barbarians_and_pirates() {
    for (fixture, civilization, units, destination) in [
        (
            "remove_casualties_attacker",
            "Barbarians",
            vec![0, 1, 2, 3],
            "C1",
        ),
        ("ship_combat", "Pirates", vec![7, 8], "D2"),
    ] {
        let mut game = JsonTest::new("combat").load_game(fixture);
        game.players[1].civilization = game.cache.get_civilization(civilization);
        let game = builder(game);
        let destination = Position::from_offset(destination);
        assert!(can_move_to(&game, &units, destination), "{civilization}");
        assert!(try_execute_action(game, move_action(units, destination), 0).is_ok());
    }
}

#[test]
fn builder_preserves_barbarian_attacks_on_players() {
    let game = JsonTest::new("incidents").load_game("barbarians_attack");
    let action = Action::Response(
        server::content::persistent_events::EventResponse::SelectAdvance(
            server::advance::Advance::Storage,
        ),
    );
    let standard = try_execute_action(game.clone(), action.clone(), 0).unwrap();
    let mut peaceful = try_execute_action(builder(game), action, 0).unwrap();
    peaceful.options.variant = GameVariant::Standard;
    assert_eq!(common::to_json(&standard), common::to_json(&peaceful));
}

#[test]
fn builder_blocks_recruiting_into_player_fleets_but_keeps_pirate_battles_and_land_recruitment() {
    let game = builder(JsonTest::new("combat").load_game("recruit_combat"));
    let city = Position::from_offset("C2");
    let units = Units::new(0, 0, 4, 0, 0, None);
    let action = Action::Playing(PlayingAction::Recruit(Recruit::new(
        &units,
        city,
        ResourcePile::wood(5) + ResourcePile::gold(3),
    )));
    assert!(try_execute_action(game.clone(), action.clone(), 0).is_ok());
    let mut game = game;
    let pirates = game
        .players
        .iter()
        .position(|p| p.civilization.is_pirates())
        .unwrap();
    game.players[pirates].civilization = game.cache.get_civilization("Rome");
    assert!(
        recruit_cost_without_replaced(
            &game,
            game.player(0),
            &units,
            city,
            CostTrigger::WithModifiers
        )
        .err()
        .unwrap()
        .contains("Builder")
    );
    assert!(
        try_execute_action(game.clone(), action, 0)
            .err()
            .unwrap()
            .contains("Builder")
    );
    assert!(
        recruit_cost_without_replaced(
            &game,
            game.player(0),
            &Units::new(0, 1, 0, 0, 0, None),
            city,
            CostTrigger::WithModifiers,
        )
        .is_ok()
    );
}

#[test]
fn builder_preserves_cultural_influence() {
    let game = builder(JsonTest::new("base").load_game("cultural_influence"));
    assert!(try_execute_action(game, influence_action(), 1).is_ok());
}

#[test]
fn builder_removes_only_objectives_requiring_player_combat() {
    let standard = Cache::new(&GameOptions::default());
    let options = GameOptions {
        variant: GameVariant::Builder,
        ..GameOptions::default()
    };
    let builder = Cache::new(&options);
    let removed = [
        "Conqueror",
        "Defiance",
        "Naval Assault",
        "Scavenger",
        "Brutus",
    ];
    assert_eq!(
        builder.get_objective_cards().len(),
        standard.get_objective_cards().len()
    );
    for card in standard.get_objective_cards() {
        let filtered = builder.get_objective_card(card.id);
        let expected: Vec<_> = card
            .objectives
            .iter()
            .filter(|o| !removed.contains(&o.name.as_str()))
            .map(|o| &o.name)
            .collect();
        assert_eq!(
            filtered
                .objectives
                .iter()
                .map(|o| &o.name)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(!filtered.name().is_empty());
    }
    for name in removed {
        assert!(standard.get_objectives().iter().any(|o| o.name == name));
        assert!(!builder.get_objectives().iter().any(|o| o.name == name));
    }
    assert_eq!(builder.get_objective_card(10).name(), "Optimized Storage");
    for name in [
        "Barbarian Conquest",
        "Resistance",
        "Sea Cleansing",
        "General",
        "Warmonger",
        "Legendary Battle",
        "Threat",
        "Sea Blockade",
        "Consulate",
    ] {
        assert!(builder.get_objectives().iter().any(|o| o.name == name));
    }
}

#[test]
fn builder_combines_with_epic_and_civilization_draft_and_survives_save_load() {
    let default: GameOptions = serde_json::from_str("{}").unwrap();
    assert!(default.variant == GameVariant::Standard);
    assert_eq!(
        serde_json::to_value(default).unwrap(),
        serde_json::json!({})
    );
    let options = GameOptions {
        variant: GameVariant::Builder,
        length: GameLength::Epic,
        civilization: CivSetupOption::DraftThree,
        ..GameOptions::default()
    };
    let mut game = game_api::init(4, "builder-draft".into(), options);
    let offers = game.civilization_draft.as_ref().unwrap().offers.clone();
    for (seat, choices) in offers.into_iter().enumerate() {
        game =
            try_execute_action(game, Action::ChooseCivilization(choices[0].clone()), seat).unwrap();
    }
    let data = game.cloned_data();
    let cache = Cache::new(&data.options);
    let game = Game::from_data(data, cache, GameContext::Play);
    assert!(game.options.variant == GameVariant::Builder);
    assert_eq!(game.options.length.ages(), 10);
    assert!(game.civilization_draft.is_none());
    assert!(!game.map.tiles.is_empty());
    assert_eq!(game.cache.get_objective_card(20).name(), "Science Focus");
    for attacker in 0..4 {
        for defender in 0..4 {
            assert!(!game.can_attack_player(attacker, defender));
        }
        assert!(game.can_attack_player(attacker, 4));
        assert!(game.can_attack_player(4, attacker));
        assert!(game.can_attack_player(attacker, 5));
    }
}
