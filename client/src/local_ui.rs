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
    add_city(&mut game, player_index1, "A1");
    add_city(&mut game, player_index1, "C2");
    add_city(&mut game, player_index2, "C1");

    add_terrain(&mut game, "A1", Terrain::Fertile);
    add_terrain(&mut game, "A2", Terrain::Water);
    add_terrain(&mut game, "A3", Terrain::Unusable);
    add_terrain(&mut game, "B1", Terrain::Mountain);
    add_terrain(&mut game, "B2", Terrain::Forest);
    add_terrain(&mut game, "B3", Terrain::Fertile);
    add_terrain(&mut game, "C1", Terrain::Barren);
    add_terrain(&mut game, "C2", Terrain::Forest);
    add_terrain(&mut game, "C3", Terrain::Water);

    game
}

fn add_city(game: &mut Game, player_index1: usize, s: &str) {
    game.players[player_index1]
        .cities
        .push(City::new(player_index1, Position::from_offset(s)));
}

fn add_terrain(game: &mut Game, pos: &str, terrain: Terrain) {
    game.map.tiles.insert(Position::from_offset(pos), terrain);
}
