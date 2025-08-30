use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::map::{Block, Terrain};
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use std::collections::HashSet;
use crate::leader::leader_position;
use crate::payment::{add_unlimited_token_conversion, base_resources, PaymentConversion, PaymentConversionType};

pub(crate) fn egypt() -> Civilization {
    Civilization::new(
        "Egypt",
        vec![flood(), architecture()],
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

fn architecture() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Architecture,
        SpecialAdvanceRequirement::Advance(Advance::Engineering),
        "Architecture",
        "When paying for buildings and wonders, \
        you may pay culture tokens with mood tokens (or vice versa)",
    )
        .add_transient_event_listener(
         |event| &mut event.building_cost,
         3,
         |i, _b, _, p| {
             add_unlimited_token_conversion(&mut i.cost.conversions);
             i.info.add_log(p, "May replace resources with mood tokens");
         },
     )
        .add_transient_event_listener(
            |event| &mut event.wonder_cost,
            1,
            move |i, _, game, p| {
                add_unlimited_token_conversion(&mut i.cost.conversions);
                i.info.add_log(p, "May replace resources with mood tokens");
            },
        )
    .build()
}

