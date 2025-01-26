use crate::{civilization::Civilization, leader::Leader};

#[must_use]
pub fn get_all() -> Vec<Civilization> {
    vec![
        Civilization::new(
            "test0",
            vec![],
            vec![
                Leader::builder("Alexander", "", "", "", "").build(),
                Leader::builder("Kleopatra", "", "", "", "").build(),
            ],
        ),
        Civilization::new("test1", vec![], vec![]),
        Civilization::new(
            "Maya",
            vec![],
            vec![Leader::builder("Kʼinich Janaab Pakal I", "", "", "", "").build()],
        ),
    ]
}

#[must_use]
pub fn get_civilization_by_name(name: &str) -> Option<Civilization> {
    get_all()
        .into_iter()
        .find(|civilization| civilization.name == name)
}

#[cfg(test)]
pub mod tests {
    use crate::civilization::Civilization;

    #[must_use]
    pub fn get_test_civilization() -> Civilization {
        Civilization::new("test", vec![], vec![])
    }
}
