use server::city::City;
use server::game::Game;
use server::map::Terrain;
use server::position::Position;
use server::resource_pile::ResourcePile;

pub fn setup_local_game() -> Game {
    let mut game = Game::new(2, "a".repeat(32));
    let player_index1 = 0;
    let player_index2 = 1;
    game.players[player_index1].gain_resources(ResourcePile::new(50, 50, 50, 50, 50, 50, 50));
    game.players[player_index2].gain_resources(ResourcePile::new(50, 50, 50, 50, 50, 50, 50));
    game.players[player_index1]
        .cities
        .push(City::new(player_index1, Position::from_offset("A1")));
    game.players[player_index1]
        .cities
        .push(City::new(player_index1, Position::from_offset("C2")));
    game.players[player_index2]
        .cities
        .push(City::new(player_index2, Position::from_offset("C1")));

    game.map
        .tiles
        .insert(Position::from_offset("A1"), Terrain::Fertile);

    game
}
