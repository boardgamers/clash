use crate::ability_initializer::SelectedChoice;
use crate::city::{City, MoodState};
use crate::city_pieces::Building;
use crate::consts::WONDER_VICTORY_POINTS;
use crate::content::custom_phase_actions::{PositionRequest, Structure, StructuresRequest};
use crate::content::incidents_famine::decrease_mood_incident_city;
use crate::content::wonders::get_wonder;
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::player_events::IncidentTarget;
use crate::position::Position;
use itertools::Itertools;

pub(crate) fn earthquakes() -> Vec<Incident> {
    vec![
        volcano(),
        earthquake(30, "Earthquake", IncidentTarget::ActivePlayer),
        earthquake(31, "Heavy Earthquake", IncidentTarget::AllPlayers),
    ]
}

fn volcano() -> Incident {
    Incident::builder(
        29,
        "Vulcan",
        "If you have at least 4 cities: Select one of your cities. Kill all units in the city. Remove all structures (center, buildings, wonders) from the game permanently. Wonder effects are lost (exception: Pyramids). The city center and buildings are worth 2 points each (according to the last owner), wonders as usual.",
        IncidentBaseEffect::None)
        .add_incident_position_request(
            IncidentTarget::ActivePlayer,
            0,
            |game, player_index, _incident| {
                let p = game.get_player(player_index);
                let cities = p
                    .cities
                    .iter()
                    .map(|c| c.position)
                    .collect_vec();
                (cities.len() >= 4)
                    .then_some(PositionRequest::new(
                        cities,
                        Some("Select a city to be destroyed".to_string()),
                    ))
            },
            |game, s| {
                let position = s.choice;
                let player_index = s.player_index;
                game.add_info_log_item(&format!(
                    "{} selected city {} to be destroyed",
                    s.player_name, position
                ));
                let city = game.get_player(player_index).get_city(position).expect("city should exist");
                let buildings = city.pieces.buildings(None);
                let wonders = city.pieces.wonders.iter().map(|w| w.name.clone()).collect_vec();
                for b in buildings {
                    destroy_building(game, b, position);
                }
                for wonder in wonders {
                    destroy_wonder(game, position, &wonder);
                }
                destroy_city_center(game, position);
            },
        ).build()
}

fn earthquake(id: u8, name: &str, target: IncidentTarget) -> Incident {
    let mut b = Incident::builder(
        id,
        name,
        "If you have at least 3 cities: Select 1-3 structures (center, buildings, wonders) in your cities and remove them from the game permanently. Wonder effects are lost (exception: Pyramids). The mood of all affected cities is reduced. The city center and buildings are worth 2 points each (according to the last owner), wonders as usual.", IncidentBaseEffect::None)
        .add_incident_structures_request(
            target,
            11,
            |game, player_index, _incident| {
                let p = game.get_player(player_index);
                let cities = &p.cities;
                (cities.len() >= 3)
                    .then_some(structures_request(cities))
            },
            |game, s| {
                apply_earthquake(game, s);
            },
        )
        .add_myths_payment(target, move |game, _p| {
            game.current_event_player().must_reduce_mood.len() as u32
        });
    b = earthquake_mood_city(b, 0);
    b = earthquake_mood_city(b, 1);
    b = earthquake_mood_city(b, 2);
    b.build()
}

fn apply_earthquake(game: &mut Game, s: &SelectedChoice<Vec<(Position, Structure)>>) {
    assert!(
        structures_request(&game.get_player(s.player_index).cities).is_valid(game, &s.choice),
        "structures should be valid"
    );
    let mut l = s.choice.clone();
    l.sort_by_key(|(_p, s)| {
        // city center last
        match s {
            Structure::CityCenter => 1,
            _ => 0,
        }
    });
    for (position, structure) in l {
        match structure {
            Structure::Building(b) => destroy_building(game, b, position),
            Structure::Wonder(name) => destroy_wonder(game, position, &name),
            Structure::CityCenter => destroy_city_center(game, position),
        }
    }

    game.current_event_mut().player.must_reduce_mood = s
        .choice
        .iter()
        .chunk_by(|(p, _s)| p)
        .into_iter()
        .map(|(&p, _g)| p)
        .filter(|p| {
            game.get_any_city(*p)
                .is_some_and(|c| !matches!(c.mood_state, MoodState::Angry))
        })
        .collect_vec();
}

fn structures_request(cities: &[City]) -> StructuresRequest {
    StructuresRequest::new(
        cities.iter().flat_map(destroyable_structures).collect_vec(),
        1..=3,
        Some("Select 3 structures to be destroyed".to_string()),
    )
}

fn destroyable_structures(city: &City) -> Vec<(Position, Structure)> {
    let pieces = &city.pieces;
    let s = vec![(city.position, Structure::CityCenter)];
    let w = pieces
        .wonders
        .iter()
        .map(|w| (city.position, Structure::Wonder(w.name.clone())))
        .collect_vec();
    let b = pieces
        .buildings(None)
        .iter()
        .map(|b| (city.position, Structure::Building(*b)))
        .collect_vec();
    vec![s, b, w].into_iter().flatten().collect_vec()
}

fn destroy_city_center(game: &mut Game, position: Position) {
    let city = game.get_any_city(position).expect("city should exist");
    let owner = city.player_index;
    let p = game.get_player_mut(owner);
    p.cities.remove(
        p.cities
            .iter()
            .position(|c| c.position == position)
            .expect("city should exist"),
    );
    p.event_victory_points += 2.0;
    p.destroyed_structures.cities += 1;
    game.add_info_log_item(&format!(
        "{} gained 2 points for the city center at {}",
        game.get_player(owner).get_name(),
        position
    ));
}

fn destroy_building(game: &mut Game, b: Building, position: Position) {
    let city = game.get_any_city(position).expect("city should exist");
    let city_owner = city.player_index;
    let owner = city
        .pieces
        .building_owner(b)
        .expect("building should exist");
    let o = game.get_player_mut(owner);
    o.event_victory_points += 2.0;
    o.destroyed_structures.add_building(b);
    game.get_player_mut(city_owner)
        .get_city_mut(position)
        .expect("city should exist")
        .pieces
        .remove_building(b);
    game.add_info_log_item(&format!(
        "{} gained 2 points for the {:?} at {}",
        game.get_player(owner).get_name(),
        b,
        position
    ));
}

fn destroy_wonder(game: &mut Game, position: Position, name: &str) {
    let owner = game
        .get_any_city(position)
        .expect("city should exist")
        .player_index;
    let wonder = get_wonder(name);
    (wonder.listeners.deinitializer)(game, owner);

    let a = WONDER_VICTORY_POINTS / 2.0;
    let p = game.get_player_mut(owner);
    p.get_city_mut(position)
        .expect("city should exist")
        .pieces
        .wonders
        .retain(|w| w.name != name);
    p.event_victory_points += a;
    game.add_info_log_item(&format!(
        "{} gained {} points for the {} at {}",
        game.get_player(owner).get_name(),
        a,
        name,
        position
    ));
}

fn earthquake_mood_city(b: IncidentBuilder, i: usize) -> IncidentBuilder {
    decrease_mood_incident_city(b, i as i32, move |game, _player_index| {
        let p = game.current_event_player();
        if p.payment.resource_amount() as usize + i >= p.must_reduce_mood.len() {
            return vec![];
        }

        game.current_event_player().must_reduce_mood.clone()
    })
}
