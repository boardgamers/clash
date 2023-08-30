use crate::{resource_pile::ResourcePile, wonder::Wonder};

#[must_use]
#[rustfmt::skip]
pub fn get_all() -> Vec<Wonder> {
    vec![
        Wonder::builder("X", ResourcePile::new(3, 3, 3, 0, -1, 0, 4), vec![]).build()
    ]
}

#[must_use]
pub fn get_wonder_by_name(name: &str) -> Option<Wonder> {
    get_all().into_iter().find(|wonder| wonder.name == name)
}
