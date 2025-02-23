use crate::ability_initializer::AbilityInitializerSetup;
use crate::content::advances::IRRIGATION;
use crate::map::Terrain;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use crate::{civilization::Civilization, leader::Leader};

pub const BARBARIANS: &str = "Barbarians";

#[must_use]
pub fn get_all() -> Vec<Civilization> {
    vec![
        Civilization::new(BARBARIANS, vec![], vec![]),
        Civilization::new(
            "test0",
            vec![],
            vec![
                Leader::builder("Alexander", "", "", "", "").build(),
                Leader::builder("Kleopatra", "", "", "", "").build(),
            ],
        ),
        Civilization::new("test1", vec![], vec![]),
        Civilization::new(
            "Maya",
            vec![
                // todo add other effects
                SpecialAdvance::builder("Terrace", IRRIGATION)
                    .add_player_event_listener(
                        |events| &mut events.terrain_collect_options,
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
                        2,
                    )
                    .build(),
            ],
            vec![Leader::builder("Kʼinich Janaab Pakal I",
                                 "Shield of the sun",
                                 "ignore the first hit in a battle with an Obelisk", "", "")
                .add_player_event_listener(
                    |events| &mut events.on_combat_round,
                    |s, c, game| {
                        if c.round == 1 && game.get_any_city(c.defender_position).is_some_and(|city| city.pieces.obelisk.is_some()) {
                            s.roll_log.push("Kʼinich Janaab Pakal I ignores the first hit in a battle with an Obelisk".to_string());
                            s.hit_cancels += 1;
                        }
                    },
                    4,
                )
                .build()],
        ),
    ]
}

#[must_use]
pub fn get_civilization(name: &str) -> Option<Civilization> {
    get_all()
        .into_iter()
        .find(|civilization| civilization.name == name)
}

#[cfg(test)]
pub mod tests {
    use crate::civilization::Civilization;

    #[must_use]
    pub fn get_test_civilization() -> Civilization {
        Civilization::new("test", vec![], vec![])
    }
}
