use crate::advance::Advance;
use crate::advance::Bonus::CultureToken;
use crate::city_pieces::Building::Obelisk;
use crate::content::advances::{advance_group_builder, AdvanceGroup};

pub(crate) fn culture() -> AdvanceGroup {
    advance_group_builder(
        "Culture",
        vec![Advance::builder("Arts", "todo")
            .with_advance_bonus(CultureToken)
            .with_unlocked_building(Obelisk)],
    )
}
