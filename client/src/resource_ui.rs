use server::resource_pile::ResourcePile;
use std::collections::HashMap;
use std::fmt;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub enum ResourceType {
    Food,
    Wood,
    Ore,
    Ideas,
    Gold,
    MoodTokens,
    CultureTokens,
    Discount, //discount on building cost, which can be used for any resource that is not a token
}

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

pub fn new_resource_map(p: &ResourcePile) -> HashMap<ResourceType, u32> {
    let mut m: HashMap<ResourceType, u32> = HashMap::new();
    add_resource(&mut m, p.food, ResourceType::Food);
    add_resource(&mut m, p.wood, ResourceType::Wood);
    add_resource(&mut m, p.ore, ResourceType::Ore);
    add_resource(&mut m, p.ideas, ResourceType::Ideas);
    add_resource(&mut m, p.gold as u32, ResourceType::Gold);
    add_resource(&mut m, p.mood_tokens, ResourceType::MoodTokens);
    add_resource(&mut m, p.culture_tokens, ResourceType::CultureTokens);
    add_resource(&mut m, 0, ResourceType::Discount);
    m
}

fn add_resource(m: &mut HashMap<ResourceType, u32>, amount: u32, resource_type: ResourceType) {
    m.insert(resource_type, amount);
}

pub fn resource_name(t: ResourceType) -> &'static str {
    match t {
        ResourceType::Food => "Food",
        ResourceType::Wood => "Wood",
        ResourceType::Ore => "Ore",
        ResourceType::Ideas => "Ideas",
        ResourceType::Gold => "Gold",
        ResourceType::MoodTokens => "Mood",
        ResourceType::CultureTokens => "Culture",
        ResourceType::Discount => panic!("Discount is not a resource type"),
    }
}
