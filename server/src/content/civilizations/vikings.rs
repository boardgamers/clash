use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::map::{Block, Terrain};
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};

pub(crate) fn vikings() -> Civilization {
    Civilization::new(
        "Vikings",
        vec![ship_construction()],
        vec![],
        Some(Block::new([
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Forest,
            Terrain::Water,
        ])),
    )
}

fn ship_construction() -> SpecialAdvanceInfo {
    // todo The cost of Navigation is reduced to 0 resources
    SpecialAdvanceInfo::builder(
        SpecialAdvance::ShipConstruction,
        SpecialAdvanceRequirement::Advance(Advance::Fishing),
        "Ship Construction",
        "May move settlers and infantry in and out of water (if there is no enemy),\
        converting them to ships and back (or being carried by other ships). \
        The cost of Navigation is reduced to 0 resources.",
    )
    .build()
}
