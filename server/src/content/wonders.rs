use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::map::Terrain::Fertile;
use crate::payment::{PaymentConversionType, PaymentOptions};
use crate::{resource_pile::ResourcePile, wonder::Wonder};
use std::collections::HashSet;

#[must_use]
pub fn get_all_uncached() -> Vec<Wonder> {
    vec![
        // todo add effects
        Wonder::builder(
            "Pyramids",
            "todo",
            PaymentOptions::resources_with_discount(ResourcePile::new(3, 3, 3, 0, 0, 0, 4), PaymentConversionType::MayNotOverpay(1)),
            vec![],
        )
        .build(),
        // add other effects
        Wonder::builder(
            "Great Gardens",
            "The city with this wonder may Collect any type of resource from Grassland spaces including ideas and gold.",
            PaymentOptions::resources(ResourcePile::new(5, 5, 2, 0, 0, 0, 5)),
            vec![Advance::Irrigation],
        )
                    .add_transient_event_listener(
                |events| &mut events.terrain_collect_options,
                1,
                |m,(),()| {
                    m.insert(Fertile, HashSet::from([
                        ResourcePile::food(1),
                        ResourcePile::wood(1),
                        ResourcePile::ore(1),
                        ResourcePile::ideas(1),
                        ResourcePile::gold(1),
                    ]));
                }
            )
        .build(),
    ]
}
