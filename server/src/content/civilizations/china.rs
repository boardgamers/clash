use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::map::Terrain;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};

pub(crate) fn china() -> Civilization {
    Civilization::new("China", vec![rice()], vec![])
}

fn rice() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::RiceCultivation,
        SpecialAdvanceRequirement::Advance(Advance::Irrigation),
        "Rice Cultivation",
        "Collect add additional +1 food from up to 2 Grassland spaces outside your city, \
        if occupied by one of your settlers.",
    )
    .add_transient_event_listener(
        |event| &mut event.collect_total,
        2,
        |i, game, collections| {
            let city = game.get_any_city(i.city);
            let food = collections
                .iter()
                .filter(|c| {
                    let pos = c.position;
                    pos.distance(city.position) > 0
                        && game.map.get(pos) == Some(&Terrain::Fertile)
                        && game
                            .player(i.info.player)
                            .units
                            .iter()
                            .any(|u| u.position == pos && u.unit_type.is_settler())
                })
                .count()
                .min(2);
            i.total += ResourcePile::food(food as u8);
            i.info.log.push(format!(
                "Rice Cultivation added {}",
                ResourcePile::food(food as u8)
            ));
        },
    )
    .build()
}
