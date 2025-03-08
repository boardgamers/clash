use serde::{Deserialize, Serialize};
use std::fmt;

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
