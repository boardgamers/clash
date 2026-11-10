use serde_json::{Value, json};
use server::action::try_execute_action;
use server::advance::Advance;
use server::city_pieces::Building;
use server::content::persistent_events::{
    PersistentEventRequest, SelectedStructure, StructuresRequest,
};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::structure::Structure;
use server::web_view::view;
use server::wonder::Wonder;

mod common;

#[test]
fn structure_choices_describe_public_owners_relative_to_the_city() {
    let game = common::JsonTest::child("incidents", "earthquake").load_game("earthquake");
    let mut game = try_execute_action(
        game,
        common::advance_action(Advance::Storage, ResourcePile::gold(2)),
        0,
    )
    .unwrap();
    let before = serde_json::to_value(game.cloned_data()).unwrap();
    let displayed = view(&game, Some(0));
    let choices = displayed["decision"]["options"].as_array().unwrap();
    let target = |structure: Value| {
        choices
            .iter()
            .find(|o| o["position"] == "C2" && o["mapTarget"]["structure"] == structure)
            .unwrap()["mapTarget"]
            .clone()
    };
    assert_eq!(
        target(json!({"Building":"Temple"}))["owner"],
        json!({"player":1,"civilization":"Greece"})
    );
    for structure in [
        json!("CityCenter"),
        json!({"Building":"Fortress"}),
        json!({"Wonder":"GreatGardens"}),
    ] {
        let target = target(structure);
        assert_eq!(target["cityOwner"], 0);
        assert_eq!(target["owner"]["player"], 0);
    }
    assert_eq!(before, serde_json::to_value(game.cloned_data()).unwrap());
    assert!(view(&game, Some(1))["decision"].is_null());

    // A shared structure request may target someone else's city (e.g. influence).
    // Owner metadata must not use the acting player's identity as the city owner.
    game.current_event_handler_mut().unwrap().request =
        PersistentEventRequest::SelectStructures(StructuresRequest::new(
            vec![
                SelectedStructure::new(Position::from_offset("A1"), Structure::CityCenter),
                SelectedStructure::new(
                    Position::from_offset("A1"),
                    Structure::Building(Building::Fortress),
                ),
                // Future construction candidates have no existing owner.
                SelectedStructure::new(
                    Position::from_offset("A1"),
                    Structure::Building(Building::Academy),
                ),
                SelectedStructure::new(
                    Position::from_offset("A1"),
                    Structure::Wonder(Wonder::Colosseum),
                ),
            ],
            1..=1,
            "Choose a structure",
        ));
    let displayed = view(&game, Some(0));
    let choices = displayed["decision"]["options"].as_array().unwrap();
    assert!(choices.iter().all(|o| o["mapTarget"]["cityOwner"] == 1));
    let owner = |structure: Value| {
        choices
            .iter()
            .find(|o| o["mapTarget"]["structure"] == structure)
            .unwrap()["mapTarget"]["owner"]
            .clone()
    };
    assert_eq!(owner(json!("CityCenter"))["player"], 1);
    assert_eq!(
        owner(json!({"Building":"Fortress"})),
        json!({"player":0,"civilization":"Rome"})
    );
    assert!(owner(json!({"Building":"Academy"})).is_null());
    assert!(owner(json!({"Wonder":"Colosseum"})).is_null());
}
