use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo};

pub(crate) fn rome() -> Civilization {
    Civilization::new(
        "Rome",
        vec![
            // todo ignore famine events
            // todo sanitation cost 0 resources or free action
            SpecialAdvanceInfo::builder(
                SpecialAdvance::Aqueduct,
                Advance::Engineering,
                "Aqueduct",
                "Ignore Famine events. \
                Sanitation cost is reduced to 0 resources or a free action",
            )
            .build(),
        ],
        vec![],
    )
}
