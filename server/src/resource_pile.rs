use std::ops::{AddAssign, SubAssign};

#[derive(Default)]
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

    pub fn can_afford(&self, other: &Self) -> bool {
        let mut resource_deficit = 0;
        if other.wood > self.wood {
            resource_deficit += other.wood - self.wood;
        }
        if other.stone > self.stone {
            resource_deficit += other.stone - self.stone;
        }
        if other.food > self.food {
            resource_deficit += other.food - self.food;
        }
        if other.ideas > self.ideas {
            resource_deficit += other.ideas - self.ideas;
        }
        self.gold >= other.gold + resource_deficit
            && self.mood_tokens >= other.mood_tokens
            && self.culture_tokens >= other.culture_tokens
    }
}

impl AddAssign for ResourcePile {
    fn add_assign(&mut self, rhs: Self) {
        self.wood += rhs.wood;
        self.stone += rhs.stone;
        self.gold += rhs.gold;
        self.food += rhs.food;
        self.ideas += rhs.ideas;
        self.mood_tokens += rhs.mood_tokens;
        self.culture_tokens += rhs.culture_tokens;
    }
}

impl SubAssign for ResourcePile {
    fn sub_assign(&mut self, rhs: Self) {
        self.wood -= rhs.wood;
        self.stone -= rhs.stone;
        self.gold -= rhs.gold;
        self.food -= rhs.food;
        self.ideas -= rhs.ideas;
        self.mood_tokens -= rhs.mood_tokens;
        self.culture_tokens -= rhs.culture_tokens;
    }
}
