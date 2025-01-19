use crate::layout_ui::icon_pos;
use crate::player_ui::bottom_icon_with_label;
use crate::render_context::RenderContext;
use macroquad::math::vec2;
use server::resource::{new_resource_map, resource_types, ResourceType};
use server::resource_pile::ResourcePile;

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

pub fn show_resource_pile(rc: &RenderContext, p: &ResourcePile, must_show: &[ResourceType]) {
    let resource_map = new_resource_map(p);
    let show: Vec<ResourceType> = resource_types()
        .into_iter()
        .filter(|r| resource_map[r] > 0 || must_show.contains(r))
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
