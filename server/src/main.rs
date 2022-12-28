use civilization::Civilization;
use hexagon::Hexagon;
use leader::Leader;
use player::{Player, PlayerEvents};
use player_setup::InitializerAndDeinitializer;
use resource_pile::ResourcePile;

mod building;
mod city;
mod civilization;
mod events;
mod hexagon;
mod landmark;
mod leader;
mod player;
mod player_setup;
mod resource_pile;
mod special_technology;
mod technology;
mod unit;
mod wonder;

fn main() {
    //demo
    let mut civilizations = vec![Civilization::new(
        "Rome",
        vec![],
        vec![Leader::create("Caesar")
            .add_event_listener(
                |player_events| &mut player_events.some_event,
                |value| *value += 1,
                0,
            )
            .build()],
        None,
    )];

    let mut player0 = Player::new("player0", civilizations.remove(0));
    player0.set_active_leader(0);
    let mut value = 0;
    player0.events.some_event.trigger(&mut value);
    println!("{}", value);
    player0.kill_leader();
    let mut value = 0;
    player0.events.some_event.trigger(&mut value);
    println!("{}", value);
}
