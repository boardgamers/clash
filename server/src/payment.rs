use crate::events::EventOrigin;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PaymentConversion {
    pub from: Vec<ResourcePile>, // alternatives
    pub to: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl PaymentConversion {
    #[must_use]
    pub fn unlimited(from: Vec<ResourcePile>, to: ResourcePile) -> Self {
        PaymentConversion {
            from,
            to,
            limit: None,
        }
    }

    #[must_use]
    pub fn limited(from: Vec<ResourcePile>, to: ResourcePile, limit: u32) -> Self {
        PaymentConversion {
            from,
            to,
            limit: Some(limit),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PaymentOptions {
    pub default: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conversions: Vec<PaymentConversion>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<EventOrigin>,
}

impl PaymentOptions {
    #[must_use]
    pub fn first_valid_payment(&self, available: &ResourcePile) -> Option<ResourcePile> {
        let discount_left = self
            .conversions
            .iter()
            .filter_map(|c| if c.to.is_empty() { c.limit } else { None })
            .sum::<u32>();
        if discount_left == 0 && available.has_at_least(&self.default) {
            return Some(self.default.clone());
        }

        self.conversions
            .iter()
            .permutations(self.conversions.len())
            .find_map(|conversions| {
                can_convert(available, &self.default, &conversions, 0, discount_left)
            })
    }

    #[must_use]
    pub fn is_valid_payment(&self, payment: &ResourcePile) -> bool {
        self.first_valid_payment(payment)
            .is_some_and(|p| &p == payment)
    }

    #[must_use]
    pub fn free() -> Self {
        Self::resources(ResourcePile::empty())
    }

    #[must_use]
    pub fn sum(cost: u32, types_by_preference: &[ResourceType]) -> Self {
        let mut conversions = vec![];
        types_by_preference.windows(2).for_each(|pair| {
            conversions.push(PaymentConversion::unlimited(
                vec![ResourcePile::of(pair[0], 1)],
                ResourcePile::of(pair[1], 1),
            ));
        });
        PaymentOptions {
            default: ResourcePile::of(types_by_preference[0], cost),
            conversions,
            modifiers: vec![],
        }
    }

    #[must_use]
    pub fn resources_with_discount(cost: ResourcePile, discount: u32) -> Self {
        let base_resources = vec![
            ResourcePile::food(1),
            ResourcePile::wood(1),
            ResourcePile::ore(1),
            ResourcePile::ideas(1),
        ];
        let mut conversions = vec![PaymentConversion {
            from: base_resources.clone(),
            to: ResourcePile::gold(1),
            limit: None,
        }];
        if discount > 0 {
            conversions.push(PaymentConversion::limited(
                base_resources.clone(),
                ResourcePile::empty(),
                discount,
            ));
        }
        PaymentOptions {
            default: cost,
            conversions,
            modifiers: vec![],
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

impl Display for PaymentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.default)?;
        // this is a bit ugly, make it nicer
        for conversion in &self.conversions {
            write!(f, " > {}", conversion.to.types().first().expect("no type"))?;
            if let Some(limit) = conversion.limit {
                write!(f, " (limit: {limit})")?;
            }
        }
        Ok(())
    }
}

#[must_use]
pub fn can_convert(
    available: &ResourcePile,
    current: &ResourcePile,
    conversions: &[&PaymentConversion],
    skip_from: usize,
    discount_left: u32,
) -> Option<ResourcePile> {
    if available.has_at_least(current) && discount_left == 0 {
        return Some(current.clone());
    }

    if conversions.is_empty() {
        return None;
    }
    let conversion = &conversions[0];
    if skip_from >= conversion.from.len() {
        return can_convert(available, current, &conversions[1..], 0, discount_left);
    }
    let from = &conversion.from[skip_from];

    let upper_limit = conversion.limit.unwrap_or(u32::MAX);
    for amount in 1..=upper_limit {
        if !current.has_at_least_times(from, amount)
            || (conversion.to.is_empty() && amount > discount_left)
        {
            return can_convert(
                available,
                current,
                conversions,
                skip_from + 1,
                discount_left,
            );
        }

        let mut current = current.clone();
        for _ in 0..amount {
            current -= from.clone();
            current += conversion.to.clone();
        }
        let new_discount_left = if conversion.to.is_empty() {
            discount_left - amount
        } else {
            discount_left
        };

        let can = can_convert(
            available,
            &current,
            conversions,
            skip_from + 1,
            new_discount_left,
        );
        if can.is_some() {
            return can;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ValidPaymentTestCase {
        name: String,
        options: PaymentOptions,
        valid: Vec<ResourcePile>,
        invalid: Vec<ResourcePile>,
    }

    #[test]
    fn test_find_valid_payment() {
        let cost = PaymentOptions {
            default: ResourcePile::food(1),
            conversions: vec![PaymentConversion {
                from: vec![ResourcePile::food(1)],
                to: ResourcePile::wood(1),
                limit: None,
            }],
            modifiers: vec![],
        };
        let available = ResourcePile::wood(1) + ResourcePile::ore(1);
        assert_eq!(
            Some(ResourcePile::wood(1)),
            cost.first_valid_payment(&available)
        );
    }

    #[test]
    fn test_is_valid_payment() {
        let test_cases = vec![
            ValidPaymentTestCase {
                name: "no conversions".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(1),
                    conversions: vec![],
                    modifiers: vec![],
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
                    modifiers: vec![],
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
                    modifiers: vec![],
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
                    modifiers: vec![],
                },
                valid: vec![
                    ResourcePile::food(2),
                    ResourcePile::wood(1) + ResourcePile::food(1),
                ],
                invalid: vec![ResourcePile::wood(2)],
            },
            ValidPaymentTestCase {
                name: "3 infantry with draft".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(3) + ResourcePile::ore(3),
                    conversions: vec![PaymentConversion {
                        from: vec![ResourcePile::food(1) + ResourcePile::ore(1)],
                        to: ResourcePile::mood_tokens(1),
                        limit: Some(1),
                    }],
                    modifiers: vec![],
                },
                valid: vec![
                    ResourcePile::food(3) + ResourcePile::ore(3),
                    ResourcePile::food(2) + ResourcePile::ore(2) + ResourcePile::mood_tokens(1),
                ],
                invalid: vec![
                    ResourcePile::food(1) + ResourcePile::ore(1) + ResourcePile::mood_tokens(2),
                    ResourcePile::mood_tokens(3),
                ],
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
                    modifiers: vec![],
                },
                valid: vec![ResourcePile::food(1)],
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
                    modifiers: vec![],
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
                    modifiers: vec![],
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
                    modifiers: vec![],
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
                    test_case.options.first_valid_payment(valid),
                    "{} valid {}",
                    test_case.name,
                    i
                );
            }
            for (i, invalid) in test_case.invalid.iter().enumerate() {
                assert_ne!(
                    Some(invalid.clone()),
                    test_case.options.first_valid_payment(invalid),
                    "{} invalid {}",
                    test_case.name,
                    i
                );
            }
        }
    }
}
