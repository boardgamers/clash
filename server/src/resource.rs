use crate::game::{Game, UndoContext};
use crate::resource_pile::ResourcePile;
use std::collections::HashMap;
use std::fmt;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub enum ResourceType {
    Food,
    Wood,
    Ore,
    Ideas,
    Gold,
    MoodTokens,    // is not a resource, but a token, with no limit
    CultureTokens, // is not a resource, but a token, with no limit
    Discount, //discount on building cost, which can be used for any resource that is not a token
}

#[must_use]
pub fn resource_types() -> Vec<ResourceType> {
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

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[must_use]
pub fn new_resource_map(p: &ResourcePile) -> HashMap<ResourceType, u32> {
    let mut m: HashMap<ResourceType, u32> = HashMap::new();
    add_resource(&mut m, p.food, ResourceType::Food);
    add_resource(&mut m, p.wood, ResourceType::Wood);
    add_resource(&mut m, p.ore, ResourceType::Ore);
    add_resource(&mut m, p.ideas, ResourceType::Ideas);
    add_resource(&mut m, p.gold as u32, ResourceType::Gold);
    add_resource(&mut m, p.mood_tokens, ResourceType::MoodTokens);
    add_resource(&mut m, p.culture_tokens, ResourceType::CultureTokens);
    m
}

fn add_resource(m: &mut HashMap<ResourceType, u32>, amount: u32, resource_type: ResourceType) {
    m.insert(resource_type, amount);
}

pub(crate) fn check_for_waste(game: &mut Game, player_index: usize) {
    let mut wasted_resources = ResourcePile::empty();
    for p in &mut game.players {
        if p.wasted_resources.is_empty() {
            continue;
        }
        assert_eq!(
            p.index, player_index,
            "non-active player {} has wasted resources: {:?}",
            p.index, p.wasted_resources
        );
        wasted_resources = p.wasted_resources.clone();
        p.wasted_resources = ResourcePile::empty();
    }
    if !wasted_resources.is_empty() {
        game.push_undo_context(UndoContext::WastedResources {
            resources: wasted_resources,
        });
    }
}
