use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, AdvanceBuilder};
use crate::content::advances::{advance_group_builder, AdvanceGroup};
use crate::content::custom_actions::CustomActionType::{AbsolutePower, ForcedLabor};

pub(crate) fn autocracy() -> AdvanceGroup {
    advance_group_builder(
        "Autocracy",
        vec![nationalism(), absolute_power(), forced_labor()],
    )
}

fn nationalism() -> AdvanceBuilder {
    Advance::builder("Nationalism", "todo")
}

fn absolute_power() -> AdvanceBuilder {
    Advance::builder(
        "Absolute Power",
        "Once per turn, as a free action, you may spend 2 mood tokens to get an additional action",
    )
    .add_custom_action(AbsolutePower)
}

fn forced_labor() -> AdvanceBuilder {
    Advance::builder(
        "Forced Labor",
        "Once per turn, as a free action, you may spend 1 mood token to treat your Angry cities as neutral for the rest of the turn",
    )
        .add_custom_action(ForcedLabor)
}
