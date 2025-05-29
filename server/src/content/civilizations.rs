pub(crate) mod greece;
pub(crate) mod maya;
pub(crate) mod rome;

use crate::civilization::Civilization;
use crate::leader::{Leader, LeaderAbility};

pub const BARBARIANS: &str = "Barbarians";
pub const PIRATES: &str = "Pirates";

#[must_use]
pub fn get_all_uncached() -> Vec<Civilization> {
    vec![
        Civilization::new(BARBARIANS, vec![], vec![]),
        Civilization::new(PIRATES, vec![], vec![]),
        rome::rome(),
        greece::greece(),
        // not finished yet: maya
        // until the real civilizations are implemented
        Civilization::new(
            "Federation",
            vec![],
            vec![
                test_leader("James T. Kirk"),
                test_leader("Kathryn Janeway"),
                test_leader("Spock"),
            ],
        ),
        Civilization::new(
            "Borg",
            vec![],
            vec![
                test_leader("Borg Queen"),
                test_leader("Seven of Nine"),
                test_leader("Jean Luc Picard"),
            ],
        ),
        Civilization::new(
            "Klingons",
            vec![],
            vec![
                test_leader("Khan"),
                test_leader("Kahless"),
                test_leader("Worf"),
            ],
        ),
        Civilization::new(
            "Romulans",
            vec![],
            vec![
                test_leader("Sela"),
                test_leader("Narek"),
                test_leader("Tomalak"),
            ],
        ),
    ]
}

fn test_leader(name: &str) -> Leader {
    Leader::new(
        name,
        LeaderAbility::builder("", "").build(),
        LeaderAbility::builder("", "").build(),
    )
}
