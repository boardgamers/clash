use crate::collect::CollectContext;
use crate::game::Game;
use crate::payment::PaymentModel;
use crate::playing_actions::PlayingActionType;
use crate::{
    city::City, city_pieces::Building, events::EventMut, player::Player, position::Position,
    resource_pile::ResourcePile, wonder::Wonder,
};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct PlayerEvents {
    pub on_construct: EventMut<Player, Position, Building>,
    pub on_undo_construct: EventMut<Player, Position, Building>,
    pub construct_cost: EventMut<ResourcePile, City, Building>,
    pub on_construct_wonder: EventMut<Player, Position, Wonder>,
    pub on_undo_construct_wonder: EventMut<Player, Position, Wonder>,
    pub wonder_cost: EventMut<PaymentModel, City, Wonder>,
    pub on_advance: EventMut<Player, String, ()>,
    pub on_undo_advance: EventMut<Player, String, ()>,
    pub advance_cost: EventMut<u32, String>,
    pub is_playing_action_available: EventMut<bool, PlayingActionType, Player>,
    pub collect_options: EventMut<HashMap<Position, Vec<ResourcePile>>, CollectContext, Game>,
}

impl PlayerEvents {
    pub fn new() -> PlayerEvents {
        Self::default()
    }
}
