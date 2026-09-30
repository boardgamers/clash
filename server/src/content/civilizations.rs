pub(crate) mod aztecs;
pub(crate) mod carthage;
pub(crate) mod celts;
pub(crate) mod china;
pub(crate) mod construction;
pub(crate) mod egypt;
pub(crate) mod greece;
pub(crate) mod huns;
pub(crate) mod india;
pub(crate) mod japan;
pub(crate) mod maya;
pub(crate) mod persia;
pub(crate) mod phoenicia;
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
        babylonia::babylonia(),
        india::india(),
        egypt::egypt(),
        phoenicia::phoenicia(),
        maya::maya(),
        persia::persia(),
        japan::japan(),
        huns::huns(),
        celts::celts(),
        aztecs::aztecs(),
        carthage::carthage(),
    ]
}
pub(crate) mod babylonia;
