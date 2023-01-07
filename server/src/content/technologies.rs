use crate::technology::Technology;

pub fn get_technologies() -> Vec<Technology> {
    vec![]
}

pub fn get_technology_by_name(name: &str) -> Option<Technology> {
    get_technologies()
        .into_iter()
        .find(|technology| technology.name == name)
}
