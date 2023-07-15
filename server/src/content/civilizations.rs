use crate::{civilization::Civilization, leader::Leader};

pub fn get_civilizations() -> Vec<Civilization> {
    vec![
        Civilization::new("test0", vec![], vec![]),
        Civilization::new("test1", vec![], vec![]),
    ]
}

pub fn get_civilization_by_name(name: &str) -> Option<Civilization> {
    get_civilizations()
        .into_iter()
        .find(|civilization| civilization.name == name)
}

pub fn get_leader_by_name(name: &str, civilization: &str) -> Option<Leader> {
    get_civilization_by_name(civilization)?
        .leaders
        .into_iter()
        .find(|leader| leader.name == name)
}

#[cfg(test)]
pub mod tests {
    use crate::civilization::Civilization;

    pub fn get_test_civilization() -> Civilization {
        Civilization::new("test", vec![], vec![])
    }
}
