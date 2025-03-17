use crate::ability_initializer::AbilityInitializerSetup;
use crate::content::advances::IRRIGATION;
use crate::game::Game;
use crate::map::Terrain::Fertile;
use crate::payment::{PaymentConversionType, PaymentOptions};
use crate::position::Position;
use crate::utils::remove_element;
use crate::{resource_pile::ResourcePile, wonder::Wonder};
use std::collections::HashSet;

#[must_use]
pub fn get_all() -> Vec<Wonder> {
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
            vec![IRRIGATION],
        )
            .add_player_event_listener(
                |events| &mut events.terrain_collect_options,
                1,
                |m,(),(), ()| {
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

///
/// # Panics
/// Panics if wonder does not exist
#[must_use]
pub fn get_wonder(name: &str) -> Wonder {
    get_all()
        .into_iter()
        .find(|wonder| wonder.name == name)
        .expect("wonder not found")
}

///
///
/// # Panics
///
/// Panics if city does not exist or if player does not have enough resources
pub fn construct_wonder(
    game: &mut Game,
    player_index: usize,
    city_position: Position,
    name: &str,
    payment: ResourcePile,
) {
    if remove_element(
        &mut game.get_player_mut(player_index).wonder_cards,
        &name.to_string(),
    )
    .is_none()
    {
        panic!("wonder not found");
    }
    let wonder = get_wonder(name);

    let city = game.players[player_index].get_city(city_position);
    assert!(
        city.can_build_wonder(&wonder, &game.players[player_index], game),
        "Illegal action"
    );
    game.players[player_index].lose_resources(payment);

    game.build_wonder(wonder, city_position, player_index);
}
