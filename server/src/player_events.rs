use crate::{events::EventMut, player::Player, city::{City, Building}, wonder::Wonder, resource_pile::ResourcePile};

#[derive(Default)]
pub struct PlayerEvents {
    pub city_size_increase: EventMut<Player, City, Building>,
    pub building_cost: EventMut<ResourcePile, City, Building>,
    pub wonder_cost: EventMut<ResourcePile, City, Wonder>,
}
