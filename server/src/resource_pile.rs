use std::{
    cmp,
    fmt::Display,
    iter::Sum,
    ops::{Add, AddAssign, Mul, SubAssign},
};

use serde::{Deserialize, Serialize};

use crate::utils;

#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq, Debug, Hash)]
pub struct ResourcePile {
    pub food: u32,
    pub wood: u32,
    pub ore: u32,
    pub ideas: u32,
    pub gold: i32,
    pub mood_tokens: u32,
    pub culture_tokens: u32,
}

impl ResourcePile {
    #[must_use]
    pub fn new(
        food: u32,
        wood: u32,
        ore: u32,
        ideas: u32,
        gold: i32,
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
    pub fn food(amount: u32) -> Self {
        Self::new(amount, 0, 0, 0, 0, 0, 0)
    }

    #[must_use]
    pub fn wood(amount: u32) -> Self {
        Self::new(0, amount, 0, 0, 0, 0, 0)
    }

    #[must_use]
    pub fn ore(amount: u32) -> Self {
        Self::new(0, 0, amount, 0, 0, 0, 0)
    }

    #[must_use]
    pub fn ideas(amount: u32) -> Self {
        Self::new(0, 0, 0, amount, 0, 0, 0)
    }

    #[must_use]
    pub fn gold(amount: i32) -> Self {
        Self::new(0, 0, 0, 0, amount, 0, 0)
    }

    #[must_use]
    pub fn mood_tokens(amount: u32) -> Self {
        Self::new(0, 0, 0, 0, 0, amount, 0)
    }

    #[must_use]
    pub fn culture_tokens(amount: u32) -> Self {
        Self::new(0, 0, 0, 0, 0, 0, amount)
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn can_afford(&self, other: &Self) -> bool {
        let mut resource_deficit = 0;
        if other.food > self.food {
            resource_deficit += other.food - self.food;
        }
        if other.wood > self.wood {
            resource_deficit += other.wood - self.wood;
        }
        if other.ore > self.ore {
            resource_deficit += other.ore - self.ore;
        }
        if other.ideas > self.ideas {
            resource_deficit += other.ideas - self.ideas;
        }
        self.gold >= other.gold + resource_deficit as i32
            && self.mood_tokens >= other.mood_tokens
            && self.culture_tokens >= other.culture_tokens
    }

    pub fn apply_resource_limit(&mut self, limit: &ResourcePile) {
        if self.food > limit.food {
            self.food = limit.food;
        }
        if self.wood > limit.wood {
            self.wood = limit.wood;
        }
        if self.ore > limit.ore {
            self.ore = limit.ore;
        }
        if self.ideas > limit.ideas {
            self.ideas = limit.ideas;
        }
        if self.gold > limit.gold {
            self.gold = limit.gold;
        }
        if self.mood_tokens > limit.mood_tokens {
            self.mood_tokens = limit.mood_tokens;
        }
        if self.culture_tokens > limit.culture_tokens {
            self.culture_tokens = limit.culture_tokens;
        }
    }

    //this function assumes that `self` can afford `cost`
    #[must_use]
    pub fn get_payment_options(&self, cost: &Self) -> PaymentOptions {
        let mut gold_left = self.gold as u32;
        let mut gold_cost = cost.gold;
        let mut jokers_left = 0;
        if gold_cost >= 0 {
            gold_left -= gold_cost as u32;
        } else {
            jokers_left = (-gold_cost) as u32;
            gold_cost = 0;
        }
        if cost.food > self.food {
            let joker_cost = cost.food - self.food;
            if joker_cost > jokers_left {
                gold_left -= joker_cost - jokers_left;
                gold_cost += (joker_cost - jokers_left) as i32;
            }
            jokers_left = jokers_left.saturating_sub(joker_cost);
        }
        if cost.wood > self.wood {
            let joker_cost = cost.wood - self.wood;
            if joker_cost > jokers_left {
                gold_left -= joker_cost - jokers_left;
                gold_cost += (joker_cost - jokers_left) as i32;
            }
            jokers_left = jokers_left.saturating_sub(joker_cost);
        }
        if cost.ore > self.ore {
            let joker_cost = cost.ore - self.ore;
            if joker_cost > jokers_left {
                gold_left -= joker_cost - jokers_left;
                gold_cost += (joker_cost - jokers_left) as i32;
            }
            jokers_left = jokers_left.saturating_sub(joker_cost);
        }
        if cost.ideas > self.ideas {
            let joker_cost = cost.ideas - self.ideas;
            if joker_cost > jokers_left {
                gold_left -= joker_cost - jokers_left;
                gold_cost += (joker_cost - jokers_left) as i32;
            }
            jokers_left = jokers_left.saturating_sub(joker_cost);
        }
        let default = Self::new(
            cmp::min(cost.food, self.food),
            cmp::min(cost.wood, self.wood),
            cmp::min(cost.ore, self.ore),
            cmp::min(cost.ideas, self.ideas),
            gold_cost,
            cost.mood_tokens,
            cost.culture_tokens,
        );
        PaymentOptions::new(default, gold_left, jokers_left)
    }

    //this function assumes that `self` can afford `cost`
    #[must_use]
    pub fn get_advance_payment_options(&self, cost: u32) -> AdvancePaymentOptions {
        let mut idea_cost = 0;
        let mut food_cost = 0;
        let mut gold_cost = 0;
        for _ in 0..cost {
            if idea_cost < self.ideas {
                idea_cost += 1;
                continue;
            }
            if food_cost < self.food {
                food_cost += 1;
                continue;
            }
            gold_cost += 1;
        }
        let food_left = self.food - food_cost;
        let gold_left = self.gold as u32 - gold_cost;
        let default = ResourcePile::ideas(idea_cost)
            + ResourcePile::food(food_cost)
            + ResourcePile::gold(gold_cost as i32);
        AdvancePaymentOptions::new(default, food_left, gold_left)
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
    pub fn is_valid_payment(&self, payment: &Self) -> bool {
        payment.can_afford(self) && self.resource_amount() == payment.resource_amount()
    }

    #[must_use]
    pub fn resource_amount(&self) -> u32 {
        self.food
            + self.wood
            + self.ore
            + self.ideas
            + self.gold as u32
            + self.mood_tokens
            + self.culture_tokens
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
            self.gold * rhs as i32,
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
        if self.gold < 0 {
            resources.push(format!("{} discount", -self.gold));
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

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PaymentOptions {
    pub default: ResourcePile,
    pub gold_left: u32,
    pub discount: u32,
}

impl PaymentOptions {
    #[must_use]
    pub fn new(default: ResourcePile, gold_left: u32, jokers_left: u32) -> Self {
        Self {
            default,
            gold_left,
            discount: jokers_left,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct AdvancePaymentOptions {
    pub default: ResourcePile,
    pub food_left: u32,
    pub gold_left: u32,
}

impl AdvancePaymentOptions {
    #[must_use]
    pub fn new(default: ResourcePile, food_left: u32, gold_left: u32) -> Self {
        Self {
            default,
            food_left,
            gold_left,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AdvancePaymentOptions, PaymentOptions, ResourcePile};

    fn assert_can_afford(name: &str, cost: &ResourcePile) {
        let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
        assert!(player_has.can_afford(cost), "{name}");
    }

    fn assert_cannot_afford(name: &str, cost: &ResourcePile) {
        let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
        assert!(!player_has.can_afford(cost), "{name}");
    }

    fn assert_payment_options(name: &str, cost: &ResourcePile, options: &PaymentOptions) {
        let budget = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
        assert_eq!(options, &budget.get_payment_options(cost), "{name}");
    }

    fn assert_advance_payment_options(
        name: &str,
        budget: &ResourcePile,
        options: &AdvancePaymentOptions,
    ) {
        assert_eq!(options, &budget.get_advance_payment_options(2), "{name}");
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
        assert_can_afford("use 6 gold as wood", &ResourcePile::wood(7));
        assert_cannot_afford("6 gold is not enough", &ResourcePile::wood(8));

        assert_cannot_afford(
            "gold cannot be converted to mood",
            &ResourcePile::mood_tokens(7),
        );
        assert_cannot_afford(
            "gold cannot be converted to culture",
            &ResourcePile::culture_tokens(8),
        );

        assert_can_afford(
            "negative gold means rebate",
            &(ResourcePile::gold(-2) + ResourcePile::wood(9)),
        );
        assert_cannot_afford(
            "negative gold cannot rebate mood",
            &(ResourcePile::gold(-2) + ResourcePile::mood_tokens(9)),
        );
        assert_cannot_afford(
            "negative gold cannot rebate culture",
            &(ResourcePile::gold(-2) + ResourcePile::mood_tokens(8)),
        );

        assert_can_afford("payment costs gold", &ResourcePile::wood(5));
        assert_cannot_afford(
            "gold cannot be converted, because it's already used for payment",
            &(ResourcePile::wood(7) + ResourcePile::gold(1)),
        );
    }

    #[test]
    fn resource_limit_test() {
        let mut resources = ResourcePile::new(3, 6, 9, 9, 0, 10, 6);
        resources.apply_resource_limit(&ResourcePile::new(7, 5, 7, 10, 3, 7, 6));
        assert_eq!(ResourcePile::new(3, 5, 7, 9, 0, 7, 6), resources);
    }

    #[test]
    fn payment_options_test() {
        assert_payment_options(
            "no gold use",
            &ResourcePile::new(1, 1, 3, 2, 0, 2, 4),
            &(PaymentOptions::new(ResourcePile::new(1, 1, 3, 2, 0, 2, 4), 5, 0)),
        );
        assert_payment_options(
            "use some gold",
            &ResourcePile::new(2, 2, 3, 5, 2, 0, 0),
            &(PaymentOptions::new(ResourcePile::new(1, 2, 3, 4, 4, 0, 0), 1, 0)),
        );
        assert_payment_options(
            "jokers",
            &(ResourcePile::ore(4) + ResourcePile::ideas(4) + ResourcePile::gold(-3)),
            &(PaymentOptions::new(ResourcePile::ore(3) + ResourcePile::ideas(4), 5, 2)),
        );
    }

    #[test]
    fn advance_payment_options_test() {
        assert_advance_payment_options(
            "enough of all resources",
            &(ResourcePile::food(3) + ResourcePile::ideas(3) + ResourcePile::gold(3)),
            &(AdvancePaymentOptions::new(ResourcePile::ideas(2), 3, 3)),
        );
        assert_advance_payment_options(
            "using food",
            &(ResourcePile::food(3) + ResourcePile::gold(3)),
            &(AdvancePaymentOptions::new(ResourcePile::food(2), 1, 3)),
        );
        assert_advance_payment_options(
            "using 1 gold",
            &(ResourcePile::ideas(1) + ResourcePile::gold(3)),
            &(AdvancePaymentOptions::new(ResourcePile::ideas(1) + ResourcePile::gold(1), 0, 2)),
        );
        assert_advance_payment_options(
            "one possible payment",
            &(ResourcePile::food(1) + ResourcePile::gold(1)),
            &(AdvancePaymentOptions::new(ResourcePile::food(1) + ResourcePile::gold(1), 0, 0)),
        );
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
