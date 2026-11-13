use server::action::Action;
use server::advance::Advance;
use server::city_pieces::Building;
use server::content::persistent_events::{EventResponse, SelectedStructure};
use server::game::Game;
use server::game_api::execute;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::structure::Structure;
mod common;
use common::{JsonTest, advance_action, move_action};

fn notes(game: &Game) -> Vec<(usize, String)> {
    let log = serde_json::to_value(&game.log).unwrap();
    log.as_array()
        .unwrap()
        .iter()
        .flat_map(|a| a["rounds"].as_array().unwrap())
        .flat_map(|r| r["turns"].as_array().unwrap())
        .flat_map(|t| t["actions"].as_array().unwrap())
        .flat_map(|a| a["items"].as_array().unwrap())
        .filter_map(|i| {
            Some((
                i["player"].as_u64()? as usize,
                i["Text"].as_str()?.to_owned(),
            ))
        })
        .collect()
}

#[test]
fn earthquake_reports_surviving_capacity_for_the_actual_piece_owner() {
    let mut game = JsonTest::child("incidents", "earthquake").load_game("earthquake");
    game.player_mut(0).destroyed_structures.cities = 2;
    game = execute(
        game,
        advance_action(Advance::Storage, ResourcePile::gold(2)),
        0,
    );
    let points = |g: &Game, p| {
        g.player(p)
            .special_victory_points
            .iter()
            .map(|v| v.points)
            .sum::<f32>()
    };
    let before = [points(&game, 0), points(&game, 1)];
    game = execute(
        game,
        Action::Response(EventResponse::SelectStructures(vec![
            SelectedStructure::new(Position::from_offset("B2"), Structure::CityCenter),
            SelectedStructure::new(
                Position::from_offset("C2"),
                Structure::Building(Building::Temple),
            ),
        ])),
        0,
    );
    assert_eq!(game.player(0).city_limit(), 4);
    assert_eq!(game.player(0).cities.len(), 3);
    assert!(game.player(0).is_city_available());
    assert_eq!(game.player(1).building_piece_limit(Building::Temple), 4);
    assert_eq!(game.player(0).building_piece_limit(Building::Temple), 5);
    assert_eq!(points(&game, 0) - before[0], 2.0);
    assert_eq!(points(&game, 1) - before[1], 2.0);
    let notes = notes(&game);
    assert!(notes.contains(&(
        0,
        "City center at B2 permanently destroyed; city limit reduced to 4".into()
    )));
    assert!(notes.contains(&(
        1,
        "Temple at C2 permanently destroyed; total piece limit reduced to 4".into()
    )));
    assert!(
        !notes
            .iter()
            .any(|(p, text)| *p == 0 && text.starts_with("Temple at"))
    );
}

#[test]
fn forced_raze_explains_unavailable_centers_without_permanent_loss() {
    let fixture = JsonTest::new("combat");
    for forced in [true, false] {
        let mut game = fixture.load_game("capture_and_raze_city");
        if !forced {
            game.player_mut(0).destroyed_structures.cities = 0;
        }
        let limits = [game.player(0).city_limit(), game.player(1).city_limit()];
        game = execute(
            game,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            0,
        );
        let notes = notes(&game);
        assert_eq!(
            notes
                .iter()
                .any(|(p, text)| *p == 0
                    && text == "Razed C1 because no city centers were available"),
            forced
        );
        assert!(
            !notes
                .iter()
                .any(|(_, text)| text.contains("permanently destroyed"))
        );
        assert_eq!(
            [game.player(0).city_limit(), game.player(1).city_limit()],
            limits
        );
        assert_eq!(
            game.try_get_any_city(Position::from_offset("C1")).is_none(),
            forced
        );
    }
}
