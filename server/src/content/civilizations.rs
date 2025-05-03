use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::map::Terrain;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use crate::{civilization::Civilization, leader::Leader};

pub const BARBARIANS: &str = "Barbarians";
pub const PIRATES: &str = "Pirates";

#[must_use]
pub fn get_all() -> Vec<Civilization> {
    vec![
        Civilization::new(BARBARIANS, vec![], vec![]),
        Civilization::new(PIRATES, vec![], vec![]),
        Civilization::new("test1", vec![], vec![]),
        Civilization::new("test2", vec![], vec![]),
        Civilization::new("test3", vec![], vec![]),
        Civilization::new("test4", vec![], vec![]),
    ]
}

fn test0() -> Civilization {
    Civilization::new(
        "test0",
        vec![],
        vec![
            Leader::builder("Alexander", "", "", "", "").build(),
            Leader::builder("Kleopatra", "", "", "", "").build(),
        ],
    )
}

fn maya() -> Civilization {
    Civilization::new(
        "Maya",
        vec![
            // todo add other effects
            SpecialAdvance::builder(Advance::Terrace, "Terrace", Advance::Irrigation)
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
                if c.round == 1
                    && game
                        .try_get_any_city(c.defender_position)
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

#[must_use]
pub fn get_civilization(name: &str) -> Option<Civilization> {
    match name {
        "Maya" => Some(maya()),   // still needs to be implemented
        "test0" => Some(test0()), // for testing
        _ => get_all()
            .into_iter()
            .find(|civilization| civilization.name == name),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::civilization::Civilization;

    #[must_use]
    pub fn get_test_civilization() -> Civilization {
        Civilization::new("test", vec![], vec![])
    }
}
