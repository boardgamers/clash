use crate::{civilization::Civilization, leader::Leader};
use crate::ability_initializer::AbilityInitializerSetup;

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
            vec![Leader::builder("Kʼinich Janaab Pakal I", "", "", "", "")
                .add_player_event_listener(
                    |events| &mut events.on_combat_round,
                    |s, c, game| {
                        //todo
                    },
                    0,
                )
                .build()],
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
