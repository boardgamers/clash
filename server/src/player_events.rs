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
    pub on_construct: EventMut<Player, Position, Building>,
    pub construct_cost: EventMut<ResourcePile, City, Building>,
    pub on_construct_wonder: EventMut<Player, Position, Wonder>,
    pub wonder_cost: EventMut<ResourcePile, City, Wonder>,
    pub on_advance: EventMut<Player, String, ()>,
    pub advance_cost: EventMut<u32, String>,
}

impl PlayerEvents {
    pub fn new() -> PlayerEvents {
        Self::default()
    }
}
