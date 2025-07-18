pub(crate) mod china;
pub(crate) mod greece;
pub(crate) mod maya;
pub(crate) mod rome;
pub mod vikings;

use crate::civilization::Civilization;

pub const BARBARIANS: &str = "Barbarians";
pub const PIRATES: &str = "Pirates";
pub const CHOOSE_CIV: &str = "Choose Civilization";

#[must_use]
pub fn get_all_uncached() -> Vec<Civilization> {
    vec![
        Civilization::new(CHOOSE_CIV, vec![], vec![], None),
        Civilization::new(BARBARIANS, vec![], vec![], None),
        Civilization::new(PIRATES, vec![], vec![], None),
        rome::rome(),
        greece::greece(),
        china::china(),
        vikings::vikings(),
        // not finished yet: maya
    ]
}
