pub struct ResourcePile {
    pub wood: u32,
    pub stone: u32,
    pub gold: u32,
    pub food: u32,
    pub ideas: u32,
    pub mood_tokens: u32,
    pub culture_tokens: u32,
}

impl ResourcePile {
    pub fn new(
        wood: u32,
        stone: u32,
        gold: u32,
        food: u32,
        ideas: u32,
        mood_tokens: u32,
        culture_tokens: u32,
    ) -> Self {
        Self {
            wood,
            stone,
            gold,
            food,
            ideas,
            mood_tokens,
            culture_tokens,
        }
    }

    pub fn empty() -> Self {
        Self {
            wood: 0,
            stone: 0,
            gold: 0,
            food: 0,
            ideas: 0,
            mood_tokens: 0,
            culture_tokens: 0,
        }
    }
}
