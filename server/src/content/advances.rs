use crate::advance::Advance;

pub fn get_technologies() -> Vec<Advance> {
    vec![]
}

pub fn get_advance_by_name(name: &str) -> Option<Advance> {
    get_technologies()
        .into_iter()
        .find(|technology| technology.name == name)
}
