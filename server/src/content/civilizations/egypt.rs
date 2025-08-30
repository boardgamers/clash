use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::map::{Block, Terrain};
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use std::collections::HashSet;

pub(crate) fn egypt() -> Civilization {
    Civilization::new(
        "Egypt",
        vec![flood()],
        vec![],
        Some(Block::new([
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Barren,
            Terrain::Barren,
        ])),
    )
}

fn flood() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Flood,
        SpecialAdvanceRequirement::Advance(Advance::Irrigation),
        "Flood",
        "Your cities may Collect food or wood from Barren spaces. \
        You may found a city on Barren land.",
    )
    .add_transient_event_listener(
        |event| &mut event.terrain_collect_options,
        0,
        |m, (), (), _| {
            // override choice from Irrigation
            m.insert(
                Terrain::Barren,
                HashSet::from([ResourcePile::wood(1), ResourcePile::food(1)]),
            );
        },
    )
    .build()
}
