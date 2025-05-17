use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::leader::Leader;
use crate::map::Terrain;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo};

pub(crate) fn maya() -> Civilization {
    Civilization::new(
        "Maya",
        vec![
            // todo add other effects
            SpecialAdvanceInfo::builder(
                SpecialAdvance::Terrace,
                Advance::Irrigation,
                "Terrace",
                "todo",
            )
            .add_transient_event_listener(
                |events| &mut events.terrain_collect_options,
                2,
                |m, (), ()| {
                    m.insert(
                        Terrain::Mountain,
                        std::collections::HashSet::from([
                            ResourcePile::food(1),
                            ResourcePile::wood(1),
                            ResourcePile::ore(1),
                        ]),
                    );
                },
            )
            .build(),
        ],
        vec![
            Leader::builder(
                "Kʼinich Janaab Pakal I",
                "Shield of the sun",
                "ignore the first hit in a battle with an Obelisk",
                "",
                "",
            )
            .add_combat_round_start_listener(4, |game, c, s, _role| {
                if c.first_round()
                    && game
                        .try_get_any_city(c.defender_position())
                        .is_some_and(|city| city.pieces.obelisk.is_some())
                {
                    s.roll_log.push(
                        "Kʼinich Janaab Pakal I ignores the first hit in a battle with an Obelisk"
                            .to_string(),
                    );
                    s.hit_cancels += 1;
                }
            })
            .build(),
        ],
    )
}
