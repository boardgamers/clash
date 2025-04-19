use crate::city::{City, MoodState};
use crate::city_pieces::Building;
use crate::consts::MAX_CITY_PIECES;
use crate::content::persistent_events::PersistentEventType;
use crate::game::Game;
use crate::map::Terrain;
use crate::player::Player;
use crate::player_events::CostInfo;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Construct {
    pub city_position: Position,
    pub city_piece: Building,
    pub payment: ResourcePile,
    pub port_position: Option<Position>,
}

impl Construct {
    #[must_use]
    pub fn new(city_position: Position, city_piece: Building, payment: ResourcePile) -> Self {
        Self {
            city_position,
            city_piece,
            payment,
            port_position: None,
        }
    }

    #[must_use]
    pub fn with_port_position(mut self, port_position: Option<Position>) -> Self {
        self.port_position = port_position;
        self
    }
}

///
/// # Errors
/// Returns an error if the building cannot be built
pub fn can_construct(
    city: &City,
    building: Building,
    player: &Player,
    game: &Game,
) -> Result<CostInfo, String> {
    can_construct_anything(city, player)?;
    if !city.can_activate() {
        return Err("Can't activate".to_string());
    }
    if city.mood_state == MoodState::Angry {
        return Err("City is angry".to_string());
    }
    if !city.pieces.can_add_building(building) {
        return Err("Building already exists".to_string());
    }
    if !player
        .advances
        .iter()
        .any(|a| a.unlocked_building == Some(building))
    {
        return Err("Building not researched".to_string());
    }
    if !player.is_building_available(building, game) {
        return Err("All non-destroyed buildings are built".to_string());
    }
    let cost_info = player.building_cost(game, building, None);
    if !player.can_afford(&cost_info.cost) {
        // construct cost event listener?
        return Err("Not enough resources".to_string());
    }
    Ok(cost_info)
}

pub(crate) fn can_construct_anything(city: &City, player: &Player) -> Result<(), String> {
    if city.player_index != player.index {
        return Err("Not your city".to_string());
    }
    if city.pieces.amount() >= MAX_CITY_PIECES {
        return Err("City is full".to_string());
    }
    if city.size() >= player.cities.len() {
        return Err("Need more cities".to_string());
    }

    Ok(())
}

pub(crate) fn construct(game: &mut Game, player_index: usize, c: &Construct) -> Result<(), String> {
    let player = &game.players[player_index];
    let city = player.get_city(c.city_position);
    let cost = can_construct(city, c.city_piece, player, game)?;
    if matches!(c.city_piece, Building::Port) {
        let port_position = c.port_position.as_ref().expect("Illegal action");
        assert!(
            city.position.neighbors().contains(port_position),
            "Illegal action"
        );
    } else if c.port_position.is_some() {
        panic!("Illegal action");
    }
    game.player_mut(player_index).construct(
        c.city_piece,
        c.city_position,
        c.port_position,
        cost.activate_city,
    );
    cost.pay(game, &c.payment);
    on_construct(game, player_index, c.city_piece);
    Ok(())
}

pub(crate) fn on_construct(game: &mut Game, player_index: usize, building: Building) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.construct,
        building,
        PersistentEventType::Construct,
    );
}

#[must_use]
pub fn available_buildings(
    game: &Game,
    player: usize,
    city: Position,
) -> Vec<(Building, CostInfo)> {
    let player = game.player(player);
    let city = player.get_city(city);
    Building::all()
        .into_iter()
        .filter_map(|b| can_construct(city, b, player, game).ok().map(|i| (b, i)))
        .collect()
}

#[must_use]
pub fn new_building_positions(
    game: &Game,
    building: Building,
    city: &City,
) -> Vec<Option<Position>> {
    if building != Building::Port {
        return vec![None];
    }

    game.map
        .tiles
        .iter()
        .filter_map(|(p, t)| {
            if *t == Terrain::Water && city.position.is_neighbor(*p) {
                Some(Some(*p))
            } else {
                None
            }
        })
        .collect()
}
