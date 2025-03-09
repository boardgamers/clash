use crate::city_pieces::Building;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::player_events::IncidentTarget;
use crate::resource_pile::ResourcePile;

pub(crate) fn trades() -> Vec<Incident> {
    vec![scientific_trade()]
}

fn scientific_trade() -> Incident {
    Incident::builder(
        45,
        "Scientific Trade",
        "Every player gains an amount of ideas equal to the number of cities that have an Academy or Observatory. You gain at least 2 ideas.",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
        .add_simple_incident_listener(
            IncidentTarget::AllPlayers, 0, |game, p, name, i| {
                let player = game.get_player_mut(p);
                let mut ideas = player
                    .cities
                    .iter()
                    .filter(|c| {
                        let b = c.pieces.buildings(None);
                        b.contains(&Building::Academy) || b.contains(&Building::Observatory)
                    })
                    .count();
                if i.active_player == p {
                    ideas = ideas.max(2);
                }

                let pile = ResourcePile::ideas(ideas as u32);
                player.gain_resources(pile.clone());
                game.add_info_log_item(&format!("{} gained {}", name, pile));
            }).build()
}
