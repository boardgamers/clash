use crate::game::{Game, UndoContext};
use crate::resource_pile::ResourcePile;
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

pub(crate) fn check_for_waste(game: &mut Game, player_index: usize) {
    for p in &game.players {
        assert!(
            p.wasted_resources.is_empty() || p.index == player_index,
            "non-active Player {} has wasted resources: {:?}",
            p.index,
            p.wasted_resources
        );
    }
    let wasted_resources = mem::replace(
        &mut game.players[player_index].wasted_resources,
        ResourcePile::empty(),
    );
    if !wasted_resources.is_empty() {
        game.add_to_last_log_item(&format!(". Could not store {wasted_resources}"));
        game.push_undo_context(UndoContext::WastedResources {
            resources: wasted_resources,
        });
    }
}
