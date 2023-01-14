use crate::city::{Building, City, MoodState};
use crate::civilization::Civilization;
use crate::hexagon::Position;
use crate::player::Player;
use crate::resource_pile::ResourcePile;
use crate::wonder::Wonder;

#[test]
fn conquer_test() {
    let mut old = Player::new(Civilization::new("old civ", vec![], vec![]));
    old.set_name(String::from("old"));
    let mut new = Player::new(Civilization::new("new civ", vec![], vec![]));
    new.set_name(String::from("new"));

    let wonder = Wonder::builder("wonder", ResourcePile::default(), vec![]).build();

    let position = Position::new(0, 0, 0);
    old.cities.push(City::new(old.name(), position.clone()));
    old.with_city(&position, |p, c| {
        c.build_wonder(wonder, p);
        c.increase_size(&Building::Academy, p);
        c.increase_size(&Building::Obelisk, p);
    });

    assert_eq!(6.0, old.victory_points());

    old.conquer_city(&position, &mut new);

    let c = new.get_city(&position).unwrap();
    assert_eq!(String::from("new"), c.player);
    assert_eq!(MoodState::Angry, c.mood_state);

    assert_eq!(3.0, old.victory_points());
    assert_eq!(3.0, new.victory_points());
    assert_eq!(0, old.wonders.len());
    assert_eq!(1, new.wonders.len());
    assert_eq!(1, old.influenced_buildings);
    assert_eq!(0, new.influenced_buildings);
}
