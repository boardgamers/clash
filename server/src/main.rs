use city::Building;
use civilization::Civilization;
use hexagon::Hexagon;
use leader::Leader;
use player::Player;
use resource_pile::ResourcePile;

use crate::{
    city::City, player::PlayerSetup, special_technology::SpecialTechnology, technology::Technology,
    wonder::Wonder,
};

mod army;
mod city;
mod civilization;
mod events;
mod hexagon;
mod landmark;
mod leader;
mod player;
mod resource_pile;
mod special_technology;
mod technology;
mod wonder;
mod game;

fn main() {
    //demo
    let technologies = vec![Technology::builder("Roads", None)
        .add_player_event_listener(
            |events| &mut events.some_event,
            |value, _, _| *value *= *value,
            -2,
        )
        .build()];

    let mut civilizations = vec![Civilization::new(
        "Rome",
        vec![SpecialTechnology::builder("Roman roads", "Roads")
            .add_player_event_listener(|events| &mut events.some_event, |value, _, _| *value *= 2, -1)
            .build()],
        vec![Leader::builder("Caesar")
            .add_player_event_listener(|events| &mut events.some_event, |value, _, _| *value += 1, 0)
            .build()],
        None,
    )];

    let mut wonders = vec![
        Wonder::builder("Pyramids", ResourcePile::default(), vec!["Roads"])
            .add_player_event_listener(|events| &mut events.some_event, |value, _, _| *value += 2, 0)
            .build(),
    ];

    let mut player0 = Player::new("player0", civilizations.remove(0));
    let mut city0 = City::new();
    player0.set_active_leader(0);
    player0.research_technology(&technologies[0]);
    city0.increase_size(Building::Wonder(wonders.remove(0)), &mut player0);

    player0
        .events
        .some_event
        .add_listener(|value, title, _| println!("{title}: {value}"), 0);

    let mut value = 0;
    player0
        .events
        .some_event
        .trigger(&mut value, &String::from("first test"), &());

    player0.kill_leader();
    player0.remove_technology(&technologies[0]);

    let mut value = 0;
    player0
        .events
        .some_event
        .trigger(&mut value, &String::from("second test"), &());
}
