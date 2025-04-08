use crate::common::JsonTest;
use server::ai_actions::city_collection;
use server::collect::PositionCollection;
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::new("ai");

#[test]
fn collect_city() {
    let game = &JSON.load_game("collect");
    let p = game.player(0);
    let collect = city_collection(game, p, p.get_city(Position::from_offset("C2")));
    let mut c = collect.collections;
    c.sort_by_key(|x| (x.position));
    assert_eq!(
        c,
        vec![
            PositionCollection::new(Position::from_offset("C2"), ResourcePile::wood(1)),
            PositionCollection::new(Position::from_offset("C3"), ResourcePile::food(1)),
            PositionCollection::new(Position::from_offset("D1"), ResourcePile::food(1)),
        ]
    )
}
