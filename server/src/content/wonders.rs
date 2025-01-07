use crate::game::Game;
use crate::position::Position;
use crate::{resource_pile::ResourcePile, wonder::Wonder};

#[must_use]
#[rustfmt::skip]
pub fn get_all() -> Vec<Wonder> {
    vec![
        Wonder::builder("Pyramids", ResourcePile::new(3, 3, 3, 0, -1, 0, 4), vec![]).build()
    ]
}

#[must_use]
pub fn get_wonder_by_name(name: &str) -> Option<Wonder> {
    get_all().into_iter().find(|wonder| wonder.name == name)
}

pub fn construct_wonder(
    game: &mut Game,
    player_index: usize,
    city_position: Position,
    wonder: String,
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
    if !city.can_build_wonder(&wonder, &game.players[player_index], game)
        || !payment.can_afford(&wonder.cost)
    {
        panic!("Illegal action");
    }
    game.players[player_index].loose_resources(payment);

    game.build_wonder(wonder, city_position, player_index);
}
