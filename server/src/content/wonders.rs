use crate::game::Game;
use crate::payment::PaymentModel;
use crate::position::Position;
use crate::{resource_pile::ResourcePile, wonder::Wonder};

#[must_use]
#[rustfmt::skip]
pub fn get_all() -> Vec<Wonder> {
    vec![
        Wonder::builder("Pyramids", 
            PaymentModel::resources_with_discount (
            ResourcePile::new(3, 3, 3, 0, 0, 0, 4), 1), vec![]).build()
    ]
}

#[must_use]
pub fn get_wonder_by_name(name: &str) -> Option<Wonder> {
    get_all().into_iter().find(|wonder| wonder.name == name)
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
    game.players[player_index].loose_resources(payment);

    game.build_wonder(wonder, city_position, player_index);
}
