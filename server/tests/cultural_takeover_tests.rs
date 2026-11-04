use serde_json::{Value, json};
use server::action::{Action, execute_action};
use server::cache::Cache;
use server::city_pieces::Building;
use server::content::effects::PermanentEffect;
use server::content::persistent_events::{EventResponse, SelectedStructure};
use server::cultural_influence::InfluenceCultureAttempt;
use server::game::{Game, GameContext};
use server::game_data::GameData;
use server::map::Terrain;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::structure::Structure;
use server::unit::UnitType;

fn source() -> Position {
    Position::from_offset("C3")
}
fn target() -> Position {
    source().neighbors()[0]
}

// Synthetic board based only on the repository's public test fixture.
fn fixture(building: bool, owner: usize, success: bool) -> Game {
    let mut data: Value =
        serde_json::from_str(include_str!("test_games/base/recruit.json")).unwrap();
    let p = &mut data["players"][0];
    p["civilization"] = json!("India");
    p["advances"] = json!(["Farming", "Mining", "Bartering"]);
    p["cities"] =
        json!([{ "position": source(), "mood_state":"Happy", "city_pieces":{"market":0}}]);
    p["units"] = json!([]);
    p["next_unit_id"] = json!(0);
    p["action_cards"] = json!([15]);
    p["objective_cards"] = json!([]);
    p["resources"] =
        json!({"food":2,"wood":7,"ore":7,"ideas":7,"gold":7,"mood_tokens":7,"culture_tokens":7});
    p["resource_limit"] = p["resources"].clone();
    data["players"][1]["objective_cards"] = json!([]);
    data["players"][1]["units"] = json!([]);
    data["players"].as_array_mut().unwrap().extend([
        json!({"id":2,"civilization":"Barbarians","next_unit_id":2}),
        json!({"id":3,"civilization":"Pirates","next_unit_id":0}),
    ]);
    data["players"][owner]["cities"] = json!([{
        "position":target(), "mood_state":"Angry",
        "city_pieces":if building { json!({"market":owner}) } else { json!({}) }
    }]);
    // Keep an eligible settlement available so the actual card can be played.
    if let Some(cities) = data["players"][2]["cities"].as_array_mut() {
        cities.push(json!({"position":source().neighbors()[1],"mood_state":"Angry"}));
    }
    if owner != 2 {
        data["players"][2]["cities"] =
            json!([{"position":source().neighbors()[1],"mood_state":"Angry"}]);
    }
    data["players"][2]["units"] = json!([
        {"id":0,"position":target(),"unit_type":"Infantry"},
        {"id":1,"position":target(),"unit_type":"Infantry"}
    ]);
    data["actions_left"] = json!(3);
    data["dice_roll_outcomes"] = if success { json!([10]) } else { json!([0]) };
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.map.tiles.clear();
    game.map.unexplored_blocks.clear();
    for position in std::iter::once(source()).chain(source().neighbors()) {
        game.map.tiles.insert(position, Terrain::Fertile);
    }
    execute_action(game, Action::Playing(PlayingAction::ActionCard(15)), 0)
}

fn influence(game: Game, building: bool) -> Game {
    execute_action(
        game,
        Action::Playing(PlayingAction::InfluenceCultureAttempt(
            InfluenceCultureAttempt::new(
                SelectedStructure::new(
                    target(),
                    if building {
                        Structure::Building(Building::Market)
                    } else {
                        Structure::CityCenter
                    },
                ),
                PlayingActionType::InfluenceCultureAttempt,
            ),
        )),
        0,
    )
}

fn assert_consumed(game: &Game) {
    assert!(
        !game
            .permanent_effects
            .contains(&PermanentEffect::CulturalTakeover)
    );
}

#[test]
fn cultural_takeover_does_not_replace_army_after_building_influence() {
    for owner in [2, 1] {
        let game = fixture(true, owner, true);
        let barbarians = game
            .player(2)
            .units
            .iter()
            .map(|u| (u.id, u.position, u.unit_type))
            .collect::<Vec<_>>();
        let game = influence(game, true);
        assert_eq!(game.get_any_city(target()).player_index, owner);
        assert_eq!(
            game.get_any_city(target())
                .pieces
                .building_owner(Building::Market),
            Some(0)
        );
        assert_eq!(
            game.player(2)
                .units
                .iter()
                .map(|u| (u.id, u.position, u.unit_type))
                .collect::<Vec<_>>(),
            barbarians
        );
        assert!(game.player(0).units.is_empty());
        assert!(
            game.current_event_handler().is_none(),
            "building influence must not ask for a replacement unit"
        );
        assert!(game.successful_cultural_influence);
        assert_consumed(&game);
    }
}

#[test]
fn cultural_takeover_transfers_settlement_and_replaces_army() {
    let game = influence(fixture(false, 2, true), false);
    assert_eq!(game.get_any_city(target()).player_index, 0);
    assert!(game.player(2).units.is_empty());
    assert!(game.current_event_handler().is_some());
    assert_consumed(&game);
    let game = execute_action(
        game,
        Action::Response(EventResponse::SelectUnitType(UnitType::Infantry)),
        0,
    );
    assert_eq!(game.player(0).units.len(), 1);
    assert_eq!(game.player(0).units[0].unit_type, UnitType::Infantry);
    assert_eq!(game.player(0).units[0].position, target());
    assert!(game.current_event_handler().is_none());
}

#[test]
fn cultural_takeover_failed_influence_leaves_city_and_army_unchanged() {
    for building in [false, true] {
        let game = fixture(building, 2, false);
        let barbarians = game
            .player(2)
            .units
            .iter()
            .map(|u| (u.id, u.position, u.unit_type))
            .collect::<Vec<_>>();
        let game = influence(game, building);
        let game = execute_action(
            game,
            Action::Response(EventResponse::Payment(vec![ResourcePile::empty()])),
            0,
        );
        assert_eq!(game.get_any_city(target()).player_index, 2);
        if building {
            assert_eq!(
                game.get_any_city(target())
                    .pieces
                    .building_owner(Building::Market),
                Some(2)
            );
        }
        assert_eq!(
            game.player(2)
                .units
                .iter()
                .map(|u| (u.id, u.position, u.unit_type))
                .collect::<Vec<_>>(),
            barbarians
        );
        assert!(game.player(0).units.is_empty());
        assert!(game.current_event_handler().is_none());
        assert!(!game.successful_cultural_influence);
        assert_consumed(&game);
    }
}

#[test]
fn cultural_takeover_empty_settlement_needs_no_replacement() {
    let mut game = fixture(false, 2, true);
    game.player_mut(2).units.clear();
    let game = influence(game, false);
    assert_eq!(game.get_any_city(target()).player_index, 0);
    assert!(game.player(0).units.is_empty());
    assert!(game.current_event_handler().is_none());
    assert_consumed(&game);
}

#[test]
fn cultural_takeover_does_not_allow_large_or_other_player_settlements() {
    for (building, owner) in [(true, 2), (false, 1)] {
        let game = fixture(building, owner, true);
        assert!(
            server::cultural_influence::influence_culture_boost_cost(
                &game,
                0,
                &SelectedStructure::new(target(), Structure::CityCenter),
                &PlayingActionType::InfluenceCultureAttempt,
                false,
                false,
                owner,
            )
            .is_err()
        );
    }
}
