use crate::{resource_pile::ResourcePile, wonder::Wonder};

pub fn get_wonders() -> Vec<Wonder> {
    vec![Wonder::builder("X", ResourcePile::new(3, 3, 3, 0, -1, 0, 4), vec![]).build()]
}

pub fn get_wonder_by_name(name: &str) -> Option<Wonder> {
    get_wonders().into_iter().find(|wonder| wonder.name == name)
}
