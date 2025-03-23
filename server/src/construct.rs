use crate::city::{City, MoodState};
use crate::city_pieces::Building;
use crate::consts::MAX_CITY_SIZE;
use crate::content::custom_phase_actions::CurrentEventType;
use crate::game::Game;
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
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
) -> Result<ResourcePile, String> {
    can_construct_anything(city, player)?;
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
    if !player.can_afford(&player.construct_cost(game, building, None).cost) {
        // construct cost event listener?
        return Err("Not enough resources".to_string());
    }
    //todo check cost
    Ok(())
}

pub(crate) fn can_construct_anything(city: &City, player: &Player) -> Result<(), String> {
    if city.player_index != player.index {
        return Err("Not your city".to_string());
    }
    if city.pieces.amount() >= MAX_CITY_SIZE {
        return Err("City is full".to_string());
    }
    if city.pieces.amount() >= player.cities.len() {
        return Err("Need more cities".to_string());
    }

    Ok(())
}

pub(crate) fn construct(game: &mut Game, player_index: usize, c: &Construct) {
    let player = &game.players[player_index];
    let city = player.get_city(c.city_position);
    let cost = player.construct_cost(game, c.city_piece, Some(&c.payment));
    let cost can_construct(city, c.city_piece, player, game)
        .map_err(|e| panic!("{e}"))
        .ok();
    if matches!(c.city_piece, Building::Port) {
        let port_position = c.port_position.as_ref().expect("Illegal action");
        assert!(
            city.position.neighbors().contains(port_position),
            "Illegal action"
        );
    } else if c.port_position.is_some() {
        panic!("Illegal action");
    }
    game.get_player_mut(player_index).construct(
        c.city_piece,
        c.city_position,
        c.port_position,
        cost.activate_city,
    );
    if matches!(c.city_piece, Building::Academy) {
        game.players[player_index].gain_resources(ResourcePile::ideas(2)); //todo move to on_construct
        game.add_info_log_item("Academy gained 2 ideas");
    }
    cost.pay(game, &c.payment);
    on_construct(game, player_index, c.city_piece);
}

pub(crate) fn on_construct(game: &mut Game, player_index: usize, building: Building) {
    let _ = game.trigger_current_event(
        &[player_index],
        |e| &mut e.on_construct,
        building,
        CurrentEventType::Construct,
    );
}
