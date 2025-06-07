pub(crate) mod china;
pub(crate) mod greece;
pub(crate) mod maya;
pub(crate) mod rome;

use crate::civilization::Civilization;
use crate::leader::{Leader, LeaderInfo};
use crate::leader_ability::LeaderAbility;

pub const BARBARIANS: &str = "Barbarians";
pub const PIRATES: &str = "Pirates";

#[must_use]
pub fn get_all_uncached() -> Vec<Civilization> {
    vec![
        Civilization::new(BARBARIANS, vec![], vec![],        None,
        ),
        Civilization::new(PIRATES, vec![], vec![],        None,
        ),
        rome::rome(),
        greece::greece(),
        china::china(),
        // not finished yet: maya
        // until the real civilizations are implemented
        Civilization::new(
            "Federation",
            vec![],
            vec![
                test_leader(Leader::Kirk, "James T. Kirk"),
                test_leader(Leader::Janeway, "Kathryn Janeway"),
                test_leader(Leader::Spock, "Spock"),
            ],
            None,
        ),
        Civilization::new(
            "Borg",
            vec![],
            vec![
                test_leader(Leader::BorgQueen, "Borg Queen"),
                test_leader(Leader::SevenOfNine, "Seven of Nine"),
                test_leader(Leader::Picard, "Jean Luc Picard"),
            ],
            None,
        ),
        Civilization::new(
            "Klingons",
            vec![],
            vec![
                test_leader(Leader::Khan, "Khan"),
                test_leader(Leader::Kahless, "Kahless"),
                test_leader(Leader::Worf, "Worf"),
            ],
            None,
        ),
        Civilization::new(
            "Romulans",
            vec![],
            vec![
                test_leader(Leader::Sela, "Sela"),
                test_leader(Leader::Narek, "Narek"),
                test_leader(Leader::Tomalak, "Tomalak"),
            ],
            None,
        ),
    ]
}

fn test_leader(leader: Leader, name: &str) -> LeaderInfo {
    LeaderInfo::new(
        leader,
        name,
        LeaderAbility::builder("", "").build(),
        LeaderAbility::builder("", "").build(),
    )
}
