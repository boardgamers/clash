use crate::resource::ResourceType;
use crate::utils;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    iter::Sum,
    ops::{Add, AddAssign, Mul, SubAssign},
};

#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq, Debug, Hash)]
pub struct ResourcePile {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub food: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub wood: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub ore: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub ideas: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub gold: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub mood_tokens: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub culture_tokens: u32,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_u32(n: &u32) -> bool {
    *n == 0
}

impl ResourcePile {
    #[must_use]
    pub const fn new(
        food: u32,
        wood: u32,
        ore: u32,
        ideas: u32,
        gold: u32,
        mood_tokens: u32,
        culture_tokens: u32,
    ) -> Self {
        Self {
            food,
            wood,
            ore,
            ideas,
            gold,
            mood_tokens,
            culture_tokens,
        }
    }

    #[must_use]
    pub const fn of(resource_type: ResourceType, amount: u32) -> Self {
        let mut p = Self::empty();
        p.add_type(resource_type, amount as i32);
        p
    }

    #[must_use]
    pub fn get(&self, resource_type: &ResourceType) -> u32 {
        match resource_type {
            ResourceType::Food => self.food,
            ResourceType::Wood => self.wood,
            ResourceType::Ore => self.ore,
            ResourceType::Ideas => self.ideas,
            ResourceType::Gold => self.gold,
            ResourceType::MoodTokens => self.mood_tokens,
            ResourceType::CultureTokens => self.culture_tokens,
        }
    }

    #[must_use]
    pub fn has_at_least(&self, other: &ResourcePile, times: u32) -> bool {
        self.food >= other.food * times
            && self.wood >= other.wood * times
            && self.ore >= other.ore * times
            && self.ideas >= other.ideas * times
            && self.gold >= other.gold * times
            && self.mood_tokens >= other.mood_tokens * times
            && self.culture_tokens >= other.culture_tokens * times
    }

    ///
    /// # Panics
    /// Panics if `resource_type` is `Discount`
    pub const fn add_type(&mut self, resource_type: ResourceType, amount: i32) {
        match resource_type {
            ResourceType::Food => self.food = (self.food as i32 + amount) as u32,
            ResourceType::Wood => self.wood = (self.wood as i32 + amount) as u32,
            ResourceType::Ore => self.ore = (self.ore as i32 + amount) as u32,
            ResourceType::Ideas => self.ideas = (self.ideas as i32 + amount) as u32,
            ResourceType::Gold => self.gold = (self.gold as i32 + amount) as u32,
            ResourceType::MoodTokens => {
                self.mood_tokens = (self.mood_tokens as i32 + amount) as u32;
            }
            ResourceType::CultureTokens => {
                self.culture_tokens = (self.culture_tokens as i32 + amount) as u32;
            }
        }
    }

    #[must_use]
    pub const fn food(amount: u32) -> Self {
        Self::new(amount, 0, 0, 0, 0, 0, 0)
    }

    #[must_use]
    pub const fn wood(amount: u32) -> Self {
        Self::new(0, amount, 0, 0, 0, 0, 0)
    }

    #[must_use]
    pub const fn ore(amount: u32) -> Self {
        Self::new(0, 0, amount, 0, 0, 0, 0)
    }

    #[must_use]
    pub const fn ideas(amount: u32) -> Self {
        Self::new(0, 0, 0, amount, 0, 0, 0)
    }

    #[must_use]
    pub const fn gold(amount: u32) -> Self {
        Self::new(0, 0, 0, 0, amount, 0, 0)
    }

    #[must_use]
    pub const fn mood_tokens(amount: u32) -> Self {
        Self::new(0, 0, 0, 0, 0, amount, 0)
    }

    #[must_use]
    pub const fn culture_tokens(amount: u32) -> Self {
        Self::new(0, 0, 0, 0, 0, 0, amount)
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self::wood(0)
    }

    #[must_use]
    pub fn apply_resource_limit(&mut self, limit: &ResourcePile) -> ResourcePile {
        let mut waste = ResourcePile::empty();
        if self.food > limit.food {
            waste.food = self.food - limit.food;
            self.food = limit.food;
        }
        if self.wood > limit.wood {
            waste.wood = self.wood - limit.wood;
            self.wood = limit.wood;
        }
        if self.ore > limit.ore {
            waste.ore = self.ore - limit.ore;
            self.ore = limit.ore;
        }
        if self.ideas > limit.ideas {
            waste.ideas = self.ideas - limit.ideas;
            self.ideas = limit.ideas;
        }
        if self.gold > limit.gold {
            waste.gold = self.gold - limit.gold;
            self.gold = limit.gold;
        }
        waste
    }

    #[must_use]
    pub fn has_common_resource(&self, other: &Self) -> bool {
        self.food > 0 && other.food > 0
            || self.wood > 0 && other.wood > 0
            || self.ore > 0 && other.ore > 0
            || self.ideas > 0 && other.ideas > 0
            || self.gold > 0 && other.gold > 0
            || self.mood_tokens > 0 && other.mood_tokens > 0
            || self.culture_tokens > 0 && other.culture_tokens > 0
    }

    #[must_use]
    pub fn resource_amount(&self) -> u32 {
        self.food
            + self.wood
            + self.ore
            + self.ideas
            + self.gold
            + self.mood_tokens
            + self.culture_tokens
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resource_amount() == 0
    }

    #[must_use]
    pub fn types(&self) -> std::vec::Vec<ResourceType> {
        let mut types = Vec::new();
        if self.food > 0 {
            types.push(ResourceType::Food);
        }
        if self.wood > 0 {
            types.push(ResourceType::Wood);
        }
        if self.ore > 0 {
            types.push(ResourceType::Ore);
        }
        if self.ideas > 0 {
            types.push(ResourceType::Ideas);
        }
        if self.gold > 0 {
            types.push(ResourceType::Gold);
        }
        if self.mood_tokens > 0 {
            types.push(ResourceType::MoodTokens);
        }
        if self.culture_tokens > 0 {
            types.push(ResourceType::CultureTokens);
        }
        types
    }
}

impl AddAssign for ResourcePile {
    fn add_assign(&mut self, rhs: Self) {
        self.food += rhs.food;
        self.wood += rhs.wood;
        self.ore += rhs.ore;
        self.ideas += rhs.ideas;
        self.gold += rhs.gold;
        self.mood_tokens += rhs.mood_tokens;
        self.culture_tokens += rhs.culture_tokens;
    }
}

impl Add for ResourcePile {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.food + rhs.food,
            self.wood + rhs.wood,
            self.ore + rhs.ore,
            self.ideas + rhs.ideas,
            self.gold + rhs.gold,
            self.mood_tokens + rhs.mood_tokens,
            self.culture_tokens + rhs.culture_tokens,
        )
    }
}

impl SubAssign for ResourcePile {
    fn sub_assign(&mut self, rhs: Self) {
        self.food = self.food.saturating_sub(rhs.food);
        self.wood = self.wood.saturating_sub(rhs.wood);
        self.ore = self.ore.saturating_sub(rhs.ore);
        self.ideas = self.ideas.saturating_sub(rhs.ideas);
        self.gold = self.gold - rhs.gold;
        self.mood_tokens = self.mood_tokens.saturating_sub(rhs.mood_tokens);
        self.culture_tokens = self.culture_tokens.saturating_sub(rhs.culture_tokens);
    }
}

impl Mul<u32> for ResourcePile {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        Self::new(
            self.food * rhs,
            self.wood * rhs,
            self.ore * rhs,
            self.ideas * rhs,
            self.gold * rhs,
            self.mood_tokens * rhs,
            self.culture_tokens * rhs,
        )
    }
}

impl Sum for ResourcePile {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut sum = Self::empty();
        for addend in iter {
            sum += addend;
        }
        sum
    }
}

impl Display for ResourcePile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut resources = Vec::new();
        if self.food > 0 {
            resources.push(format!("{} food", self.food));
        }
        if self.wood > 0 {
            resources.push(format!("{} wood", self.wood));
        }
        if self.ore > 0 {
            resources.push(format!("{} ore", self.ore));
        }
        if self.ideas > 0 {
            resources.push(format!(
                "{} {}",
                self.ideas,
                if self.ideas == 1 { "idea" } else { "ideas" }
            ));
        }
        if self.gold > 0 {
            resources.push(format!("{} gold", self.gold));
        }
        if self.mood_tokens > 0 {
            resources.push(format!(
                "{} {}",
                self.mood_tokens,
                if self.mood_tokens == 1 {
                    "mood token"
                } else {
                    "mood tokens"
                }
            ));
        }
        if self.culture_tokens > 0 {
            resources.push(format!(
                "{} {}",
                self.culture_tokens,
                if self.culture_tokens == 1 {
                    "culture token"
                } else {
                    "culture tokens"
                }
            ));
        }
        write!(f, "{}", utils::format_list(&resources, "nothing"))
    }
}

// Used for payments where gold can be used to replace other resources (not tokens)
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CostWithDiscount {
    pub cost: ResourcePile,
    pub discount: u32,
}

impl CostWithDiscount {
    #[must_use]
    pub fn can_afford(&self, available: &ResourcePile) -> bool {
        let cost = &self.cost;
        let mut resource_deficit = 0;
        if cost.food > available.food {
            resource_deficit += cost.food - available.food;
        }
        if cost.wood > available.wood {
            resource_deficit += cost.wood - available.wood;
        }
        if cost.ore > available.ore {
            resource_deficit += cost.ore - available.ore;
        }
        if cost.ideas > available.ideas {
            resource_deficit += cost.ideas - available.ideas;
        }
        available.gold + self.discount >= cost.gold + resource_deficit
            && available.mood_tokens >= cost.mood_tokens
            && available.culture_tokens >= cost.culture_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::ResourcePile;
    use crate::payment::PaymentOptions;

    fn assert_can_afford(name: &str, cost: &ResourcePile, discount: u32) {
        let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
        let can_afford =
            PaymentOptions::resources_with_discount(cost.clone(), discount).can_afford(&player_has);
        assert!(can_afford, "{name}");
    }

    fn assert_cannot_afford(name: &str, cost: &ResourcePile, discount: u32) {
        let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
        let can_afford =
            PaymentOptions::resources_with_discount(cost.clone(), discount).can_afford(&player_has);
        assert!(!can_afford, "{name}");
    }

    fn assert_to_string(resource_pile: &ResourcePile, expected: &str) {
        assert_eq!(
            expected.to_string(),
            resource_pile.to_string(),
            "expected {expected} but found {resource_pile}"
        );
    }

    #[test]
    fn can_afford_test() {
        assert_can_afford("use 6 gold as wood", &ResourcePile::wood(7), 0);
        assert_cannot_afford("6 gold is not enough", &ResourcePile::wood(8), 0);

        assert_cannot_afford(
            "gold cannot be converted to mood",
            &ResourcePile::mood_tokens(7),
            0,
        );
        assert_cannot_afford(
            "gold cannot be converted to culture",
            &ResourcePile::culture_tokens(8),
            0,
        );

        assert_can_afford("negative gold means rebate", &(ResourcePile::wood(9)), 2);
        assert_cannot_afford(
            "discount cannot rebate mood",
            &(ResourcePile::mood_tokens(9)),
            2,
        );
        assert_cannot_afford(
            "discount cannot rebate culture",
            &(ResourcePile::mood_tokens(8)),
            2,
        );

        assert_can_afford("payment costs gold", &ResourcePile::wood(5), 0);
        assert_cannot_afford(
            "gold cannot be converted, because it's already used for payment",
            &(ResourcePile::wood(7) + ResourcePile::gold(1)),
            0,
        );
    }

    #[test]
    fn resource_limit_test() {
        let mut resources = ResourcePile::new(3, 6, 9, 9, 0, 10, 6);
        let waste = resources.apply_resource_limit(&ResourcePile::new(7, 5, 7, 10, 3, 7, 6));
        assert_eq!(ResourcePile::new(3, 5, 7, 9, 0, 10, 6), resources);
        assert_eq!(ResourcePile::new(0, 1, 2, 0, 0, 0, 0), waste);
    }

    #[test]
    fn resource_pile_display_test() {
        assert_to_string(&ResourcePile::empty(), "nothing");
        assert_to_string(&ResourcePile::ore(1), "1 ore");
        assert_to_string(&ResourcePile::mood_tokens(2), "2 mood tokens");
        assert_to_string(
            &(ResourcePile::food(3) + ResourcePile::culture_tokens(1)),
            "3 food and 1 culture token",
        );
        assert_to_string(
            &(ResourcePile::ideas(5) + ResourcePile::wood(1) + ResourcePile::gold(10)),
            "1 wood, 5 ideas and 10 gold",
        );
    }
}
