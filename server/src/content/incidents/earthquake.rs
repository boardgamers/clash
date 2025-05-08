use crate::ability_initializer::SelectedChoice;
use crate::city::{City, MoodState};
use crate::city_pieces::{Building, remove_building};
use crate::content::persistent_events::{
    PositionRequest, SelectedStructure, Structure, StructuresRequest, is_selected_structures_valid,
};
use crate::game::Game;
use crate::incident::{DecreaseMood, Incident, IncidentBaseEffect, MoodModifier};
use crate::player_events::{IncidentInfo, IncidentTarget};
use crate::position::Position;
use crate::wonder::{Wonder, deinit_wonder};
use itertools::Itertools;

pub(crate) fn earthquake_incidents() -> Vec<Incident> {
    vec![
        volcano(),
        earthquake(30, "Earthquake", IncidentTarget::ActivePlayer),
        earthquake(31, "Heavy Earthquake", IncidentTarget::AllPlayers),
        flood(32, "Flood", IncidentTarget::ActivePlayer),
        flood(33, "Heavy Flood", IncidentTarget::AllPlayers),
    ]
}

fn volcano() -> Incident {
    Incident::builder(
        29,
        "Vulcan",
        "If you have at least 4 cities: \
            Select one of your cities. \
            Kill all units in the city. \
            Remove all structures (center, buildings, wonders) from the game permanently. \
            Wonder effects are lost (exception: Pyramids). \
            The city center and buildings are worth 2 points each (according to the last owner), \
            wonders as usual.",
        IncidentBaseEffect::None,
    )
    .add_incident_position_request(
        IncidentTarget::ActivePlayer,
        0,
        |game, player_index, _incident| {
            let p = game.player(player_index);
            let cities = p.cities.iter().map(|c| c.position).collect_vec();
            let needed = 1..=1;
            (cities.len() >= 4).then_some(PositionRequest::new(
                cities,
                needed,
                "Select a city to be destroyed",
            ))
        },
        |game, s, _| {
            let pos = s.choice[0];
            let player_index = s.player_index;
            game.add_info_log_item(&format!(
                "{} selected city {} to be destroyed",
                s.player_name, pos
            ));
            let city = game.player(player_index).get_city(pos);
            let buildings = city.pieces.buildings(None);
            let wonders = city.pieces.wonders.iter().copied().collect_vec();
            for b in buildings {
                destroy_building(game, b, pos);
            }
            for wonder in wonders {
                destroy_wonder(game, pos, wonder);
            }
            destroy_city_center(game, pos);
        },
    )
    .build()
}

fn earthquake(id: u8, name: &str, target: IncidentTarget) -> Incident {
    Incident::builder(
        id,
        name,
        "If you have at least 3 cities: \
                      Select 1-3 structures (center, buildings, wonders) in your cities \
                      and remove them from the game permanently. \
                      Wonder effects are lost (exception: Pyramids). \
                      The mood of all affected cities is reduced. \
                      The city center and buildings are worth 2 points each \
                      (according to the last owner), wonders as usual.",
        IncidentBaseEffect::None,
    )
    .add_incident_structures_request(
        target,
        11,
        |game, player_index, _incident| {
            let p = game.player(player_index);
            let cities = &p.cities;
            (cities.len() >= 3).then_some(structures_request(cities))
        },
        |game, s, i| {
            apply_earthquake(game, s, i);
        },
    )
    .add_decrease_mood(
        IncidentTarget::AllPlayers,
        MoodModifier::Decrease,
        move |_p, _game, i| {
            let c = &i.player.must_reduce_mood;
            DecreaseMood::new(c.clone(), c.len() as u8)
        },
    )
    .build()
}

fn apply_earthquake(
    game: &mut Game,
    s: &SelectedChoice<Vec<SelectedStructure>>,
    i: &mut IncidentInfo,
) {
    assert!(
        is_selected_structures_valid(game, &s.choice),
        "structures should be valid"
    );
    let mut l = s.choice.clone();
    l.sort_by_key(|s| {
        // city center last
        match s.structure {
            Structure::CityCenter => 1,
            _ => 0,
        }
    });
    for st in l {
        let position = st.position;
        match st.structure {
            Structure::Building(b) => destroy_building(game, b, position),
            Structure::Wonder(name) => destroy_wonder(game, position, name),
            Structure::CityCenter => destroy_city_center(game, position),
        }
    }

    i.player.must_reduce_mood = s
        .choice
        .iter()
        .chunk_by(|s| s.position)
        .into_iter()
        .map(|(p, _g)| p)
        .filter(|p| {
            game.try_get_any_city(*p)
                .is_some_and(|c| !matches!(c.mood_state, MoodState::Angry))
        })
        .collect_vec();
}

fn structures_request(cities: &[City]) -> StructuresRequest {
    StructuresRequest::new(
        cities.iter().flat_map(destroyable_structures).collect_vec(),
        1..=3,
        "Select 1-3 structures to be destroyed",
    )
}

fn destroyable_structures(city: &City) -> Vec<SelectedStructure> {
    let pieces = &city.pieces;
    let s = vec![SelectedStructure::new(city.position, Structure::CityCenter)];
    let w = pieces
        .wonders
        .iter()
        .map(|w| SelectedStructure::new(city.position, Structure::Wonder(*w)))
        .collect_vec();
    let b = pieces
        .buildings(None)
        .iter()
        .map(|b| SelectedStructure::new(city.position, Structure::Building(*b)))
        .collect_vec();
    vec![s, b, w].into_iter().flatten().collect_vec()
}

fn destroy_city_center(game: &mut Game, position: Position) {
    let city = game.get_any_city(position);
    let owner = city.player_index;
    let p = game.player_mut(owner);
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
        game.player_name(owner),
        position
    ));
}

fn destroy_building(game: &mut Game, b: Building, position: Position) {
    let city = game.get_any_city(position);
    let city_owner = city.player_index;
    let owner = city
        .pieces
        .building_owner(b)
        .expect("building should exist");
    let o = game.player_mut(owner);
    o.event_victory_points += 2.0;
    o.destroyed_structures.add_building(b);
    remove_building(game.player_mut(city_owner).get_city_mut(position), b);
    game.add_info_log_item(&format!(
        "{} gained 2 points for the {} at {}",
        game.player_name(owner),
        b,
        position
    ));
}

fn destroy_wonder(game: &mut Game, position: Position, name: Wonder) {
    let owner = game.get_any_city(position).player_index;
    deinit_wonder(game, owner, name);

    let a = name.info(game).owned_victory_points;
    let p = game.player_mut(owner);
    p.wonders_owned.remove(name);
    let city = p.get_city_mut(position);
    city.pieces.wonders.retain(|w| *w != name);
    p.event_victory_points += a as f32;
    game.add_info_log_item(&format!(
        "{} gained {} points for the {} at {}",
        game.player_name(owner),
        a,
        name.name(game),
        position
    ));
}

fn flood(id: u8, name: &str, target: IncidentTarget) -> Incident {
    Incident::builder(
        id,
        name,
        "Select one of your cities that is adjacent to water. \
                      Decrease the mood in that city.",
        IncidentBaseEffect::None,
    )
    .add_decrease_mood(target, MoodModifier::Decrease, |p, game, _| {
        DecreaseMood::new(non_angry_shore_cites(game, p.index), 1)
    })
    .build()
}

fn non_angry_shore_cites(game: &Game, player_index: usize) -> Vec<Position> {
    let p = game.player(player_index);
    p.cities
        .iter()
        .filter(|c| {
            c.position.neighbors().iter().any(|p| game.map.is_sea(*p))
                && !matches!(c.mood_state, MoodState::Angry)
        })
        .map(|c| c.position)
        .collect_vec()
}
