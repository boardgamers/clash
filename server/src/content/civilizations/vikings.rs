use crate::civilization::Civilization;
use crate::map::{Block, Terrain};

pub(crate) fn vikings() -> Civilization {
    Civilization::new(
        "Vikings",
        vec![],
        vec![],
        Some(Block::new([
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Forest,
            Terrain::Water,
        ])),
    )
}
