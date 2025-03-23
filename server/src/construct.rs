use serde::{Deserialize, Serialize};
use crate::city_pieces::Building;
use crate::content::custom_phase_actions::CurrentEventType;
use crate::content::wonders;
use crate::game::Game;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::utils::remove_element;
use crate::wonder::{ConstructWonder, Wonder};

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

pub(crate) fn construct(game: &mut Game, player_index: usize, c: &Construct) {
    let player = &game.players[player_index];
    let city = player.get_city(c.city_position);
    let cost = player.construct_cost(game, c.city_piece, Some(&c.payment));
    city.can_construct(c.city_piece, player, game)
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
    game.players[player_index].construct(
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

pub(crate) fn construct_wonder_with_card(
    game: &mut Game,
    player_index: usize,
    c: &ConstructWonder,
) {
    let name = &c.wonder;
    if remove_element(
        &mut game.get_player_mut(player_index).wonder_cards,
        &name.to_string(),
    )
    .is_none()
    {
        panic!("wonder not found");
    }
    let wonder = wonders::get_wonder(name);

    let city_position = c.city_position;
    let p = game.get_player(player_index);
    let city = p.get_city(city_position);

    if !player.can_afford(&player.wonder_cost(wonder, self, None).cost) {
        return Err("Not enough resources".to_string());
    }
    
    let cost = p.wonder_cost().construct_cost(game, c.city_piece, Some(&c.payment));

    city.can_build_wonder(&wonder, &p, game)
        .map_err(|e| panic!("{e}"))
        .ok();

    game.players[player_index].lose_resources(c.payment.clone());

    construct_wonder(game, wonder, city_position, player_index);
}

pub(crate) fn construct_wonder(game: &mut Game, wonder: Wonder, city_position: Position, player_index: usize) {
    let wonder = wonder;
    (wonder.listeners.initializer)(game, player_index);
    (wonder.listeners.one_time_initializer)(game, player_index);
    let player = &mut game.players[player_index];
    player.wonders_build.push(wonder.name.clone());
    let name = wonder.name.clone();
    player
        .get_city_mut(city_position)
        .pieces
        .wonders
        .push(wonder);

    on_construct_wonder(game, player_index, name);
}

pub(crate) fn on_construct_wonder(game: &mut Game, player_index: usize, name: String) {
    let _ = game.trigger_current_event(
        &[player_index],
        |e| &mut e.on_construct_wonder,
        name,
        CurrentEventType::ConstructWonder,
    );
}

