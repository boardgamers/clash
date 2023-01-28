use crate::{
    city::{Building, City},
    events::EventMut,
    hexagon::Position,
    player::Player,
    resource_pile::ResourcePile,
    wonder::Wonder,
};

#[derive(Default)]
pub struct PlayerEvents {
    pub city_size_increase: EventMut<Player, Position, Building>,
    pub building_cost: EventMut<ResourcePile, City, Building>,
    pub wonder_cost: EventMut<ResourcePile, City, Wonder>,
}

impl PlayerEvents {
    pub fn new() -> PlayerEvents {
        Self::default()
    }
}
