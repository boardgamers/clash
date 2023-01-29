use super::custom_actions::CustomActionType::*;
use crate::{wonder::Wonder, resource_pile::ResourcePile};

pub fn get_wonders() -> Vec<Wonder> {
    vec![Wonder::builder("test", ResourcePile::new(3, 3, 3, 0, 0, 0, 4), vec![]).build()]
}

pub fn get_wonder_by_name(name: &str) -> Option<Wonder> {
    get_wonders().into_iter().find(|wonder| wonder.name == name)
}
