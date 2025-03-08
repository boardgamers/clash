use crate::game::Game;
use crate::resource_pile::ResourcePile;
use crate::undo::UndoContext;
use serde::{Deserialize, Serialize};
use std::{fmt, mem};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub enum ResourceType {
    Food,
    Wood,
    Ore,
    Ideas,
    Gold,
    MoodTokens,    // is not a resource, but a token, with no limit
    CultureTokens, // is not a resource, but a token, with no limit
}

impl ResourceType {
    #[must_use]
    pub fn is_token(&self) -> bool {
        matches!(self, ResourceType::MoodTokens | ResourceType::CultureTokens)
    }

    #[must_use]
    pub fn is_resource(&self) -> bool {
        !self.is_token()
    }

    #[must_use]
    pub fn is_gold(&self) -> bool {
        matches!(self, ResourceType::Gold)
    }

    #[must_use]
    pub fn all() -> Vec<ResourceType> {
        vec![
            ResourceType::Food,
            ResourceType::Wood,
            ResourceType::Ore,
            ResourceType::Ideas,
            ResourceType::Gold,
            ResourceType::MoodTokens,
            ResourceType::CultureTokens,
        ]
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub(crate) fn check_for_waste(game: &mut Game) {
    let map: Vec<usize> = game.players.iter().map(|p| p.index).collect();
    for p in map {
        let wasted_resources =
            mem::replace(&mut game.players[p].wasted_resources, ResourcePile::empty());
        if !wasted_resources.is_empty() {
            game.add_info_log_item(&format!(
                "{} could not store {wasted_resources}",
                game.player_name(p)
            ));
            game.push_undo_context(UndoContext::WastedResources {
                resources: wasted_resources,
                player_index: p,
            });
        }
    }
}
