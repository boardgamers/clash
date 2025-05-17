mod rome;
pub(crate) mod maya;

use crate::{civilization::Civilization, leader::Leader};

pub const BARBARIANS: &str = "Barbarians";
pub const PIRATES: &str = "Pirates";

#[must_use]
pub fn get_all_uncached() -> Vec<Civilization> {
    vec![
        Civilization::new(BARBARIANS, vec![], vec![]),
        Civilization::new(PIRATES, vec![], vec![]),
        rome::rome(),
        // not finished yet: maya
        // until the real civilizations are implemented
        Civilization::new("Federation", vec![], vec![
            Leader::builder("James T. Kirk", "", "", "", "").build(),
            Leader::builder("Kathryn Janeway", "", "", "", "").build(),
            Leader::builder("Spock", "", "", "", "").build(),
        ]),
        Civilization::new("Borg", vec![], vec![
            Leader::builder("Borg Queen", "", "", "", "").build(),
            Leader::builder("Seven of Nine", "", "", "", "").build(),
            Leader::builder("Jean Luc Picard", "", "", "", "").build(),
        ]),
        Civilization::new("Klingons", vec![], vec![
            Leader::builder("Khan", "", "", "", "").build(),
            Leader::builder("Kahless", "", "", "", "").build(),
            Leader::builder("Worf", "", "", "", "").build(),
        ]),
        Civilization::new("Romulans", vec![], vec![
            Leader::builder("Sela", "", "", "", "").build(),
            Leader::builder("Narek", "", "", "", "").build(),
            Leader::builder("Tomalak", "", "", "", "").build(),
        ]),
    ]
}
