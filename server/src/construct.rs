use crate::advance::{Advance, gain_advance_without_payment};
use crate::city::{City, MoodState, activate_city};
use crate::city_pieces::Building;
use crate::consts::MAX_CITY_PIECES;
use crate::content::ability::construct_event_origin;
use crate::content::persistent_events::PersistentEventType;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::map::Terrain;
use crate::player::{CostTrigger, Player};
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
///
/// # Panics
/// Panics if the required advance is not found
pub fn can_construct(
    city: &City,
    building: Building,
    player: &Player,
    game: &Game,
    trigger: CostTrigger,
) -> Result<CostInfo, String> {
    let advance = game.cache.get_building_advance(building);
    if !player.can_use_advance(advance) {
        return Err(format!("Missing advance: {}", advance.name(game)));
    }

    can_construct_anything(city, player)?;
    if city.mood_state == MoodState::Angry {
        return Err("City is angry".to_string());
    }
    if !city.pieces.can_add_building(building) {
        return Err("Building already exists".to_string());
    }
    if !player.is_building_available(building, game) {
        return Err("All non-destroyed buildings are built".to_string());
    }
    let cost_info = player.building_cost(game, building, trigger);
    if !player.can_afford(&cost_info.cost) {
        // construct cost event listener?
        return Err("Not enough resources".to_string());
    }
    Ok(cost_info)
}

pub(crate) fn can_construct_anything(city: &City, player: &Player) -> Result<(), String> {
    if !city.can_activate() {
        return Err("Can't activate".to_string());
    }
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

pub(crate) fn execute_construct(
    game: &mut Game,
    player_index: usize,
    c: &Construct,
    cost_modifier: impl Fn(CostInfo) -> CostInfo + Copy + Send + Sync,
) -> Result<(), String> {
    let player = &game.players[player_index];
    let city = player.get_city(c.city_position);
    let cost = cost_modifier(can_construct(
        city,
        c.city_piece,
        player,
        game,
        game.execute_cost_trigger(),
    )?);
    if matches!(c.city_piece, Building::Port) {
        let port_position = c.port_position.as_ref().expect("Illegal action");
        assert!(
            city.position.neighbors().contains(port_position),
            "Illegal action"
        );
    } else if c.port_position.is_some() {
        panic!("Illegal action");
    }

    let port_pos = if let Some(port_position) = c.port_position {
        let adjacent_water_tiles = c
            .city_position
            .neighbors()
            .iter()
            .filter(|neighbor| game.map.is_sea(**neighbor))
            .count();
        if adjacent_water_tiles > 1 {
            format!(" at the water tile {port_position}")
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let city_piece = c.city_piece;
    let city_position = c.city_position;

    construct(
        game,
        player_index,
        c.city_piece,
        c.city_position,
        c.port_position,
        cost.activate_city,
        cost.origin(),
    );
    cost.pay(game, &c.payment);
    game.log_with_origin(
        player_index,
        &construct_event_origin(),
        &format!("Build a {city_piece} in the city {city_position}{port_pos}"),
    );

    on_construct(game, player_index, ConstructInfo::new(c.city_piece));
    Ok(())
}

pub(crate) fn construct(
    game: &mut Game,
    player: usize,
    building: Building,
    city_position: Position,
    port_position: Option<Position>,
    activate: bool,
    origin: &EventOrigin,
) {
    if activate {
        activate_city(city_position, game, origin);
    }
    let city = game.player_mut(player).get_city_mut(city_position);
    city.pieces.set_building(building, player);
    if let Some(port_position) = port_position {
        city.port_position = Some(port_position);
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct ConstructInfo {
    pub building: Building,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gained_advance: Option<Advance>,
}

impl ConstructInfo {
    #[must_use]
    pub fn new(building: Building) -> Self {
        Self {
            building,
            gained_advance: None,
        }
    }
}

pub(crate) fn on_construct(game: &mut Game, player_index: usize, info: ConstructInfo) {
    if let Some(i) = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.construct,
        info,
        PersistentEventType::Construct,
    ) {
        if let Some(advance) = i.gained_advance {
            gain_advance_without_payment(game, advance, player_index, ResourcePile::empty(), true);
        }
    }
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
        .filter_map(|b| {
            can_construct(city, b, player, game, CostTrigger::NoModifiers)
                .ok()
                .map(|i| (b, i))
        })
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
