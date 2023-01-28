use super::custom_actions::CustomActionType::*;
use crate::wonder::Wonder;

pub fn get_wonders() -> Vec<Wonder> {
    vec![]
}

pub fn get_wonder_by_name(name: &str) -> Option<Wonder> {
    get_wonders().into_iter().find(|wonder| wonder.name == name)
}
