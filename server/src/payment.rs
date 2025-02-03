use crate::resource::ResourceType;
use crate::resource_pile::{CostWithDiscount, ResourcePile};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::{Add, SubAssign};
use itertools::Itertools;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PaymentConversion {
    pub from: Vec<ResourcePile>, // alternatives
    pub to: ResourcePile,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PaymentOptions {
    pub default: ResourcePile,
    pub conversions: Vec<PaymentConversion>,
}

impl PaymentOptions {
    #[must_use]
    pub fn first_valid_payment(&self, available: &ResourcePile) -> Option<ResourcePile> {
        let discount_needed = self.conversions.iter().filter_map(|c| if c.to.is_empty() {c.limit} else { None }).sum::<u32>();
        if discount_needed == 0 && available.has_at_least(&self.default, 1) {
            return Some(self.default.clone());
        }
        
        self.conversions.iter().permutations(self.conversions.len()).find_map(|conversions| {
            can_convert(available, &self.default, &conversions, 0, discount_needed)
        })
    }

    #[must_use]
    pub fn is_valid_payment(&self, payment: &ResourcePile) -> bool {
        self.first_valid_payment(payment).is_some()
    }
    
    #[must_use]
    pub fn free() -> Self {
        Self::resources(ResourcePile::empty())
    }

    #[must_use]
    pub fn sum(cost: u32, types_by_preference: Vec<ResourceType>) -> Self {
        let mut conversions = vec![];
        types_by_preference.windows(2).for_each(|pair| {
            conversions.push(PaymentConversion {
                from: vec![ResourcePile::of(pair[0], 1)],
                to: ResourcePile::of(pair[1], 1),
                limit: None,
            });
        });
        PaymentOptions {
            default: ResourcePile::of(types_by_preference[0], cost),
            conversions,
        }
    }

    #[must_use]
    pub fn resources_with_discount(cost: ResourcePile, discount: u32) -> Self {
        let mut conversions = vec![
            PaymentConversion {
                from: vec![ResourcePile::food(1), ResourcePile::wood(1), ResourcePile::ore(1), ResourcePile::ideas(1)],
                to: ResourcePile::gold(1),
                limit: None,
            },
        ];
        if discount > 0 {
            conversions.push(PaymentConversion {
                from: vec![cost.clone()],
                to: ResourcePile::empty(),
                limit: Some(discount),
            });
        }
        PaymentOptions {
            default: cost,
            conversions,
        }
    }
    
    #[must_use]
    pub fn resources(cost: ResourcePile) -> Self {
        Self::resources_with_discount(cost, 0)
    }

    #[must_use]
    pub fn can_afford(&self, available: &ResourcePile) -> bool {
       self.first_valid_payment(available).is_some()
    }

    #[must_use]
    pub fn is_free(&self) -> bool {
        self.is_valid_payment(&ResourcePile::empty())
    }

    #[must_use]
    pub fn possible_resource_types(&self) -> Vec<ResourceType> {
        let mut vec = self.default.types();
        for conversion in &self.conversions {
            vec.extend(conversion.to.types());
        }
        vec
    }

    #[must_use]
    pub fn default_payment(&self) -> ResourcePile {
       self.default.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct SumPaymentOptions {
    pub cost: u32,
    pub types_by_preference: Vec<ResourceType>,
}

impl SumPaymentOptions {
    #[must_use]
    pub fn is_valid_payment(&self, payment: &ResourcePile) -> bool {
        self.types_by_preference
            .iter()
            .map(|t| payment.get(t))
            .sum::<u32>()
            == self.cost
    }
}

#[must_use] 
pub fn can_convert(
    available: &ResourcePile,
    current: &ResourcePile,
    conversions: &[&PaymentConversion],
    skip_from: usize,
    discount_needed: u32,
) -> Option<ResourcePile> {
    if available.has_at_least(current, 1) && discount_needed == 0 {
        return Some(current.clone());
    }
    
    if conversions.is_empty() {
        return None;
    }
    let conversion = &conversions[0];
    if skip_from >= conversion.from.len()  {
        return can_convert(available, current, &conversions[1..], 0, discount_needed);
    }
    let from = &conversion.from[skip_from];

    let upper_limit = conversion.limit.unwrap_or(u32::MAX);
    for amount in 1..=upper_limit {
        if !current.has_at_least(from, amount) {
            return can_convert(available, current, conversions, skip_from + 1, discount_needed);
        }

        let mut current = current.clone();
        for _ in 0..amount {
            current -= from.clone();
            current += conversion.to.clone();
        }
        let new_discount_needed = if conversion.to.is_empty() { discount_needed - amount } else { discount_needed };
     
        let can = can_convert(available, &current, conversions, skip_from + 1, new_discount_needed);
        if can.is_some() {
            return can;
        }
    }
    None
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum PaymentModel {
    Sum(SumPaymentOptions),
    Resources(CostWithDiscount),
}

impl PaymentModel {
    #[must_use]
    pub const fn free() -> Self {
        Self::resources(ResourcePile::empty())
    }

    #[must_use]
    pub const fn sum(cost: u32, types_by_preference: Vec<ResourceType>) -> Self {
        PaymentModel::Sum(SumPaymentOptions {
            cost,
            types_by_preference,
        })
    }

    #[must_use]
    pub const fn resources_with_discount(cost: ResourcePile, discount: u32) -> Self {
        PaymentModel::Resources(CostWithDiscount { cost, discount })
    }
    
    #[must_use]
    pub const fn resources(cost: ResourcePile) -> Self {
        Self::resources_with_discount(cost, 0)
    }

    #[must_use]
    pub fn can_afford(&self, available: &ResourcePile) -> bool {
        match self {
            PaymentModel::Sum(options) => {
                options
                    .types_by_preference
                    .iter()
                    .map(|t| available.get(t))
                    .sum::<u32>()
                    >= options.cost
            }
            PaymentModel::Resources(c) => c.can_afford(available),
        }
    }

    #[must_use]
    pub fn is_valid_payment(&self, payment: &ResourcePile) -> bool {
        match self {
            PaymentModel::Sum(options) => options.is_valid_payment(payment),
            PaymentModel::Resources(c) => {
                c.can_afford(payment)
                    && c.cost.resource_amount() - c.discount == payment.resource_amount()
            }
        }
    }

    #[must_use]
    pub fn is_free(&self) -> bool {
        self.is_valid_payment(&ResourcePile::empty())
    }

    #[must_use]
    pub fn possible_resource_types(&self) -> Vec<ResourceType> {
        match self {
            PaymentModel::Sum(options) => options.types_by_preference.clone(),
            PaymentModel::Resources(c) => c.cost.types(),
        }
    }

    #[must_use]
    pub fn default_payment(&self) -> ResourcePile {
        match self {
            PaymentModel::Sum(options) => {
                ResourcePile::of(options.types_by_preference[0], options.cost)
            }
            PaymentModel::Resources(c) => c.cost.clone(),
        }
    }
}

impl Display for PaymentModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentModel::Sum(options) => write!(
                f,
                "a total of {} from {}",
                options.cost,
                options
                    .types_by_preference
                    .iter()
                    .map(|t| format!("{t:?}"))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            PaymentModel::Resources(c) => {
                if c.discount > 0 {
                    write!(f, "{} with discount {}", c.cost, c.discount)
                } else {
                    c.cost.fmt(f)
                }
            }
        }
    }
}

impl Add for PaymentModel {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            PaymentModel::Sum(s) => match rhs {
                PaymentModel::Sum(r) => {
                    assert_eq!(s.types_by_preference, r.types_by_preference);
                    PaymentModel::sum(s.cost + r.cost, s.types_by_preference)
                }
                PaymentModel::Resources(_) => {
                    panic!("Cannot add Sum and Resources")
                }
            },
            PaymentModel::Resources(r) => match rhs {
                PaymentModel::Sum(_) => {
                    panic!("Cannot add Resources and Sum")
                }
                PaymentModel::Resources(r2) => PaymentModel::resources_with_discount(
                    r.cost + r2.cost,
                    r.discount + r2.discount,
                ),
            },
        }
    }
}

impl SubAssign for PaymentModel {
    fn sub_assign(&mut self, rhs: Self) {
        match self {
            PaymentModel::Sum(s) => match rhs {
                PaymentModel::Sum(r) => {
                    assert_eq!(s.types_by_preference, r.types_by_preference);
                    s.cost -= r.cost;
                }
                PaymentModel::Resources(_) => {
                    panic!("Cannot subtract Resources from Sum")
                }
            },
            PaymentModel::Resources(r) => match rhs {
                PaymentModel::Sum(_) => {
                    panic!("Cannot subtract Sum from Resources")
                }
                PaymentModel::Resources(r2) => {
                    r.cost -= r2.cost;
                    r.discount -= r2.discount;
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ResourceType;

    struct ValidPaymentTestCase {
        name: String,
        options: PaymentOptions,
        valid: Vec<ResourcePile>,
        invalid: Vec<ResourcePile>,
    }

    #[test]
    fn test_find_valid_payment() {
        let options = PaymentOptions {
            default: ResourcePile::food(1),
            conversions: vec![PaymentConversion {
                from: vec![ResourcePile::food(1)],
                to: ResourcePile::wood(1),
                limit: None,
            }],
        };
        let available = ResourcePile::wood(1) + ResourcePile::ore(1);
        assert_eq!(Some(ResourcePile::wood(1)), options.first_valid_payment(&available));
    }

    #[test]
    fn test_is_valid_payment() {
        let test_cases = vec![
            ValidPaymentTestCase {
                name: "no conversions".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(1),
                    conversions: vec![],
                },
                valid: vec![ResourcePile::food(1)],
                invalid: vec![ResourcePile::food(2)],
            },
            ValidPaymentTestCase {
                name: "food to wood".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(1),
                    conversions: vec![PaymentConversion {
                        from: vec![ResourcePile::food(1)],
                        to: ResourcePile::wood(1),
                        limit: None,
                    }],
                },
                valid: vec![ResourcePile::food(1), ResourcePile::wood(1)],
                invalid: vec![ResourcePile::food(2), ResourcePile::ore(1)],
            },
            ValidPaymentTestCase {
                name: "food to wood with amount".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(2),
                    conversions: vec![PaymentConversion {
                        from: vec![ResourcePile::food(1)],
                        to: ResourcePile::wood(1),
                        limit: None,
                    }],
                },
                valid: vec![
                    ResourcePile::food(2),
                    ResourcePile::food(1) + ResourcePile::wood(1),
                    ResourcePile::wood(2),
                ],
                invalid: vec![ResourcePile::ore(2)],
            },
            ValidPaymentTestCase {
                name: "food to wood with limit".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(2),
                    conversions: vec![PaymentConversion {
                        from: vec![ResourcePile::food(1)],
                        to: ResourcePile::wood(1),
                        limit: Some(1),
                    }],
                },
                valid: vec![
                    ResourcePile::food(2),
                    ResourcePile::wood(1) + ResourcePile::food(1),
                ],
                invalid: vec![ResourcePile::wood(2)],
            },
            ValidPaymentTestCase {
                name: "discount must be used".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(3),
                    conversions: vec![PaymentConversion {
                        from: vec![ResourcePile::food(1)],
                        to: ResourcePile::empty(),
                        limit: Some(2),
                    }],
                },
                valid: vec![
                    ResourcePile::food(1),
                ],
                invalid: vec![ResourcePile::food(2)],
            },
            ValidPaymentTestCase {
                name: "food to wood to ore".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(1),
                    conversions: vec![
                        PaymentConversion {
                            from: vec![ResourcePile::food(1)],
                            to: ResourcePile::wood(1),
                            limit: None,
                        },
                        PaymentConversion {
                            from: vec![ResourcePile::wood(1)],
                            to: ResourcePile::ore(1),
                            limit: None,
                        },
                    ],
                },
                valid: vec![
                    ResourcePile::food(1),
                    ResourcePile::wood(1),
                    ResourcePile::ore(1),
                ],
                invalid: vec![ResourcePile::ideas(2)],
            },
            ValidPaymentTestCase {
                name: "food to wood to ore with reversed conversion order".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(1),
                    conversions: vec![
                        PaymentConversion {
                            from: vec![ResourcePile::wood(1)],
                            to: ResourcePile::ore(1),
                            limit: None,
                        },
                        PaymentConversion {
                            from: vec![ResourcePile::food(1)],
                            to: ResourcePile::wood(1),
                            limit: None,
                        },
                    ],
                },
                valid: vec![
                    ResourcePile::food(1),
                    ResourcePile::wood(1),
                    ResourcePile::ore(1),
                ],
                invalid: vec![ResourcePile::ideas(2)],
            },
            ValidPaymentTestCase {
                name: "gold can replace anything but mood and culture tokens".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(1)
                        + ResourcePile::wood(1)
                        + ResourcePile::mood_tokens(1),
                    conversions: vec![PaymentConversion {
                        from: vec![
                            ResourcePile::food(1),
                            ResourcePile::wood(1),
                            ResourcePile::ore(1),
                            ResourcePile::ideas(1),
                        ],
                        to: ResourcePile::gold(1),
                        limit: None,
                    }],
                },
                valid: vec![
                    ResourcePile::food(1) + ResourcePile::wood(1) + ResourcePile::mood_tokens(1),
                    ResourcePile::food(1) + ResourcePile::gold(1) + ResourcePile::mood_tokens(1),
                    ResourcePile::wood(1) + ResourcePile::gold(1) + ResourcePile::mood_tokens(1),
                    ResourcePile::gold(2) + ResourcePile::mood_tokens(1),
                ],
                invalid: vec![ResourcePile::gold(3)],
            },
        ];
        for test_case in test_cases {
            for (i, valid) in test_case.valid.iter().enumerate() {
                assert_eq!(
                    Some(valid.clone()),
                    test_case.options.first_valid_payment(&valid),
                    "{} valid {}",
                    test_case.name,
                    i
                );
            }
            for (i, invalid) in test_case.invalid.iter().enumerate() {
                assert_ne!(
                    Some(invalid.clone()),
                    test_case.options.first_valid_payment(&invalid),
                    "{} invalid {}",
                    test_case.name,
                    i
                );
            }
        }
    }
}
