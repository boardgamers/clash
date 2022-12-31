use std::{
    cmp,
    ops::{Add, AddAssign, SubAssign},
};

#[derive(Default, Clone)]
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
        food: u32,
        wood: u32,
        stone: u32,
        ideas: u32,
        gold: u32,
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

    pub fn wood(amount: u32) -> Self {
        Self::new(amount, 0, 0, 0, 0, 0, 0)
    }

    pub fn stone(amount: u32) -> Self {
        Self::new(0, amount, 0, 0, 0, 0, 0)
    }

    pub fn gold(amount: u32) -> Self {
        Self::new(0, 0, amount, 0, 0, 0, 0)
    }

    pub fn food(amount: u32) -> Self {
        Self::new(0, 0, 0, amount, 0, 0, 0)
    }

    pub fn ideas(amount: u32) -> Self {
        Self::new(0, 0, 0, 0, amount, 0, 0)
    }

    pub fn mood_tokens(amount: u32) -> Self {
        Self::new(0, 0, 0, 0, 0, amount, 0)
    }

    pub fn culture_tokens(amount: u32) -> Self {
        Self::new(0, 0, 0, 0, 0, 0, amount)
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

    pub fn apply_resource_limit(&mut self, limit: &ResourcePile) {
        if self.wood > limit.wood {
            self.wood = limit.wood;
        }
        if self.stone > limit.stone {
            self.stone = limit.stone;
        }
        if self.food > limit.food {
            self.food = limit.food;
        }
        if self.ideas > limit.ideas {
            self.ideas = limit.ideas;
        }
        if self.mood_tokens > limit.mood_tokens {
            self.mood_tokens = limit.mood_tokens;
        }
        if self.culture_tokens > limit.culture_tokens {
            self.culture_tokens = limit.culture_tokens;
        }
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

impl Add for ResourcePile {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.wood + rhs.wood,
            self.stone + rhs.stone,
            self.gold + rhs.gold,
            self.food + rhs.food,
            self.ideas + rhs.ideas,
            self.mood_tokens + rhs.mood_tokens,
            self.culture_tokens + rhs.culture_tokens,
        )
    }
}

impl SubAssign for ResourcePile {
    fn sub_assign(&mut self, rhs: Self) {
        self.wood = cmp::max(self.wood - rhs.wood, 0);
        self.stone = cmp::max(self.stone - rhs.stone, 0);
        self.gold = cmp::max(self.gold - rhs.gold, 0);
        self.food = cmp::max(self.food - rhs.food, 0);
        self.ideas = cmp::max(self.ideas - rhs.ideas, 0);
        self.mood_tokens = cmp::max(self.mood_tokens - rhs.mood_tokens, 0);
        self.culture_tokens = cmp::max(self.culture_tokens - rhs.culture_tokens, 0);
    }
}
