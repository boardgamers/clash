use crate::advance::{Advance, gain_advance_without_payment};
use crate::city::{City, MoodState, activate_city};
use crate::city_pieces::{BUILDINGS, Building, gain_building};
use crate::consts::MAX_CITY_PIECES;
use crate::content::persistent_events::PersistentEventType;
use crate::events::{EventOrigin, EventPlayer};
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

#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum ConstructDiscount {
    NoCityActivation,
    NoResourceCost,
}

pub const BUILDING_ALREADY_EXISTS: &str = "Building already exists";
pub const NOT_ENOUGH_RESOURCES: &str = "Not enough resources";

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
    discounts: &[ConstructDiscount],
) -> Result<CostInfo, String> {
    if !city.pieces.can_add_building(building) {
        return Err(BUILDING_ALREADY_EXISTS.to_string());
    }
    if !player.is_building_available(building, game) {
        return Err("All non-destroyed buildings are built".to_string());
    }
    let advance = game.cache.get_building_advance(building);
    if !player.can_use_advance(advance) {
        return Err(format!("Missing advance: {}", advance.name(game)));
    }

    let cost_info = player.building_cost(game, building, trigger);
    can_construct_anything(
        city,
        player,
        !discounts.contains(&ConstructDiscount::NoCityActivation) && cost_info.activate_city,
    )?;
    if city.mood_state == MoodState::Angry {
        return Err("City is angry".to_string());
    }
    let can_afford = discounts.contains(&ConstructDiscount::NoResourceCost)
        || player.can_afford(&cost_info.cost);
    if !can_afford {
        return Err(NOT_ENOUGH_RESOURCES.to_string());
    }
    Ok(cost_info)
}

pub(crate) fn can_construct_anything(
    city: &City,
    player: &Player,
    city_activation: bool,
) -> Result<(), String> {
    if city_activation && !city.can_activate() {
        return Err("Can't activate".to_string());
    }
    if city.player_index != player.index {
        return Err("Not your city".to_string());
    }
    if city.pieces.amount() >= MAX_CITY_PIECES {
        return Err("City already has maximum size".to_string());
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
) -> Result<(), String> {
    let player = &game.players[player_index];
    let city = player.get_city(c.city_position);
    let cost = can_construct(
        city,
        c.city_piece,
        player,
        game,
        game.execute_cost_trigger(),
        &[],
    )?;
    if matches!(c.city_piece, Building::Port) {
        let port_position = c.port_position.as_ref().expect("Illegal action");
        assert!(
            city.position.neighbors().contains(port_position),
            "Illegal action"
        );
    } else if c.port_position.is_some() {
        panic!("Illegal action");
    }

    cost.pay(game, &c.payment);
    do_construct(game, player_index, c, cost.activate_city, cost.origin());
    Ok(())
}

pub(crate) fn do_construct(
    game: &mut Game,
    player_index: usize,
    c: &Construct,
    activate: bool,
    origin: &EventOrigin,
) {
    construct(
        game,
        player_index,
        c.city_piece,
        c.city_position,
        c.port_position,
        activate,
        origin,
    );

    on_construct(game, player_index, ConstructInfo::new(c.city_piece));
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
    if let Some(port_position) = port_position {
        game.player_mut(player)
            .get_city_mut(city_position)
            .port_position = Some(port_position);
    }
    gain_building(
        game,
        &EventPlayer::new(player, origin.clone()),
        building,
        city_position,
    );
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct ConstructAdvanceBonus {
    pub advance: Advance,
    pub origin: EventOrigin,
}

impl ConstructAdvanceBonus {
    #[must_use]
    pub fn new(advance: Advance, origin: EventOrigin) -> Self {
        Self { advance, origin }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct ConstructInfo {
    pub building: Building,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gained_advance: Option<ConstructAdvanceBonus>,
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
    ) && let Some(b) = i.gained_advance
    {
        gain_advance_without_payment(
            game,
            b.advance,
            &EventPlayer::new(player_index, b.origin),
            ResourcePile::empty(),
            true,
        );
    }
}

#[must_use]
pub fn available_buildings(
    game: &Game,
    player: usize,
    city: Position,
    discounts: &[ConstructDiscount],
) -> Vec<(Building, CostInfo)> {
    let player = game.player(player);
    let city = player.get_city(city);
    BUILDINGS
        .into_iter()
        .filter_map(|b| {
            can_construct(city, b, player, game, CostTrigger::NoModifiers, discounts)
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
