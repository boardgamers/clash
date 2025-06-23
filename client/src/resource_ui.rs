use crate::layout_ui::icon_pos;
use crate::player_ui::bottom_icon_with_label;
use crate::render_context::RenderContext;
use macroquad::math::vec2;
use server::resource::ResourceType;
use server::resource_pile::ResourcePile;
use std::collections::HashMap;

pub(crate) fn resource_name(t: ResourceType) -> &'static str {
    match t {
        ResourceType::Food => "Food",
        ResourceType::Wood => "Wood",
        ResourceType::Ore => "Ore",
        ResourceType::Ideas => "Ideas",
        ResourceType::Gold => "Gold",
        ResourceType::MoodTokens => "Mood",
        ResourceType::CultureTokens => "Culture",
    }
}

#[must_use]
pub(crate) fn new_resource_map(p: &ResourcePile) -> HashMap<ResourceType, u8> {
    let mut m: HashMap<ResourceType, u8> = HashMap::new();
    add_resource(&mut m, p.food, ResourceType::Food);
    add_resource(&mut m, p.wood, ResourceType::Wood);
    add_resource(&mut m, p.ore, ResourceType::Ore);
    add_resource(&mut m, p.ideas, ResourceType::Ideas);
    add_resource(&mut m, p.gold, ResourceType::Gold);
    add_resource(&mut m, p.mood_tokens, ResourceType::MoodTokens);
    add_resource(&mut m, p.culture_tokens, ResourceType::CultureTokens);
    m
}

fn add_resource(m: &mut HashMap<ResourceType, u8>, amount: u8, resource_type: ResourceType) {
    m.insert(resource_type, amount);
}

pub(crate) fn show_resource_pile(rc: &RenderContext, p: &ResourcePile) {
    let resource_map = new_resource_map(p);
    let show: Vec<ResourceType> = ResourceType::all()
        .into_iter()
        .filter(|r| resource_map[r] > 0)
        .collect();
    for (i, r) in show.iter().rev().enumerate() {
        let x = (show.len() - i) as i8 - 3;
        let a = resource_map[r];

        bottom_icon_with_label(
            rc,
            &format!("{a}"),
            &rc.assets().resources[r],
            icon_pos(x, -2) + vec2(0., 10.),
            resource_name(*r),
        );
    }
}
