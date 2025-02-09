use crate::ability_initializer::AbilityInitializerSetup;
use crate::content::advances::IRRIGATION;
use crate::game::Game;
use crate::map::Terrain::Fertile;
use crate::payment::PaymentOptions;
use crate::position::Position;
use crate::{resource_pile::ResourcePile, wonder::Wonder};
use std::collections::HashSet;

#[must_use]
pub fn get_all() -> Vec<Wonder> {
    vec![
        // todo add effects
        Wonder::builder(
            "Pyramids",
            "todo",
            PaymentOptions::resources_with_discount(ResourcePile::new(3, 3, 3, 0, 0, 0, 4), 1),
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
                |m,(),()| {
                    m.insert(Fertile, HashSet::from([
                        ResourcePile::food(1),
                        ResourcePile::wood(1),
                        ResourcePile::ore(1),
                        ResourcePile::ideas(1),
                        ResourcePile::gold(1),
                    ]));
                },
                0
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
    wonder: &str,
    payment: ResourcePile,
) {
    let wonder_cards_index = game.players[player_index]
        .wonder_cards
        .iter()
        .position(|wonder_card| wonder_card.name == wonder)
        .expect("Illegal action");
    let wonder = game.players[player_index]
        .wonder_cards
        .remove(wonder_cards_index);
    let city = game.players[player_index]
        .get_city(city_position)
        .expect("player should have city");
    assert!(
        city.can_build_wonder(&wonder, &game.players[player_index], game),
        "Illegal action"
    );
    game.players[player_index].lose_resources(payment);

    game.build_wonder(wonder, city_position, player_index);
}
