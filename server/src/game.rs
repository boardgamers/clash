pub struct Game {
    pub players: Vec<Player>,
    seed: String,
}

impl Game {
    pub fn new(players: usize, seed: String) -> Self {
        Self {
            players: vec![Player::new(); players],
            seed,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct GameData {
    players: Vec<PlayerData>,
    seed: String,
}
