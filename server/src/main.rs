use civilization::Civilization;
use hexagon::Hexagon;
use leader::Leader;
use player::Player;
use resource_pile::ResourcePile;

use crate::{
    city::City, player::PlayerSetup, special_technology::SpecialTechnology,
    technology::Technology, wonder::Wonder,
};

mod building;
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
mod army;
mod wonder;

fn main() {
    //demo
    let technologies = vec![Technology::builder("Roads")
        .add_player_event_listener(
            |events| &mut events.some_event,
            |value, info| *value *= *value,
            -1,
        )
        .build()];

    let mut civilizations = vec![Civilization::new(
        "Rome",
        vec![SpecialTechnology::builder("Roman roads", "Roads")
            .add_player_event_listener(|events| &mut events.some_event, |value, info| *value *= 2, 1)
            .build()],
        vec![Leader::builder("Caesar")
            .add_player_event_listener(|events| &mut events.some_event, |value, info| *value += 1, 0)
            .build()],
        None,
    )];

    let mut wonders = vec![
        Wonder::builder("Pyramids", ResourcePile::default(), vec!["Roads"])
            .add_player_event_listener(|events| &mut events.some_event, |value, info| *value += 2, 0)
            .build(),
    ];

    let mut player0 = Player::new("player0", civilizations.remove(0));
    let mut city0 = City::new();
    player0.set_active_leader(0);
    player0.build_wonder(wonders.remove(0), &mut city0);
    player0.research_technology(&technologies[0]);

    let mut value = 0;
    player0.events.some_event.trigger(&mut value, &String::from("test"));
    println!("{}", value);
    player0.kill_leader();
    let mut value = 0;
    player0.events.some_event.trigger(&mut value, &String::from("second test"));
    println!("{}", value);
}
