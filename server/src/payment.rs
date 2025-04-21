use crate::events::EventOrigin;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::RangeInclusive;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum PaymentConversionType {
    Unlimited,
    MayOverpay(u8),
    MayNotOverpay(u8),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct PaymentConversion {
    pub from: Vec<ResourcePile>, // alternatives
    pub to: ResourcePile,
    #[serde(rename = "type")]
    pub payment_conversion_type: PaymentConversionType,
}

impl PaymentConversion {
    #[must_use]
    pub fn unlimited(from: ResourcePile, to: ResourcePile) -> Self {
        PaymentConversion::new(vec![from], to, PaymentConversionType::Unlimited)
    }

    #[must_use]
    pub fn limited(from: ResourcePile, to: ResourcePile, limit: u8) -> Self {
        PaymentConversion::new(vec![from], to, PaymentConversionType::MayNotOverpay(limit))
    }

    #[must_use]
    pub fn new(
        from: Vec<ResourcePile>,
        to: ResourcePile,
        payment_conversion_type: PaymentConversionType,
    ) -> Self {
        PaymentConversion {
            from,
            to,
            payment_conversion_type,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
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
            .filter_map(|c| {
                if c.to.is_empty() {
                    match c.payment_conversion_type {
                        PaymentConversionType::Unlimited => None,
                        PaymentConversionType::MayOverpay(i)
                        | PaymentConversionType::MayNotOverpay(i) => Some(i),
                    }
                } else {
                    None
                }
            })
            .sum::<u8>();
        if discount_left == 0 && available.has_at_least(&self.default) {
            return Some(self.default.clone());
        }
        let may_overpay = self.conversions.iter().any(|c| {
            matches!(
                c.payment_conversion_type,
                PaymentConversionType::MayOverpay(_)
            )
        });

        self.conversions
            .iter()
            .permutations(self.conversions.len())
            .find_map(|conversions| {
                can_convert(
                    available,
                    &self.default,
                    &conversions,
                    0,
                    discount_left,
                    may_overpay,
                )
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
    pub(crate) fn sum(cost: u8, types_by_preference: &[ResourceType]) -> Self {
        let mut conversions = vec![];
        types_by_preference.windows(2).for_each(|pair| {
            conversions.push(PaymentConversion::unlimited(
                ResourcePile::of(pair[0], 1),
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
    pub(crate) fn single_type(t: ResourceType, r: RangeInclusive<u8>) -> PaymentOptions {
        let max = r.clone().max().expect("range empty");
        let d = max - r.min().expect("range empty");
        PaymentOptions {
            default: ResourcePile::of(t, max),
            conversions: vec![PaymentConversion::new(
                vec![ResourcePile::of(t, 1)],
                ResourcePile::empty(),
                PaymentConversionType::MayOverpay(d),
            )],
            modifiers: vec![],
        }
    }

    #[must_use]
    pub(crate) fn tokens(cost: u8) -> Self {
        Self::sum(
            cost,
            &[ResourceType::MoodTokens, ResourceType::CultureTokens],
        )
    }

    #[must_use]
    pub(crate) fn resources_with_discount(
        cost: ResourcePile,
        discount_type: PaymentConversionType,
    ) -> Self {
        let base_resources = vec![
            ResourcePile::food(1),
            ResourcePile::wood(1),
            ResourcePile::ore(1),
            ResourcePile::ideas(1),
        ];

        let mut conversions = vec![PaymentConversion {
            from: base_resources.clone(),
            to: ResourcePile::gold(1),
            payment_conversion_type: PaymentConversionType::Unlimited,
        }];
        if !matches!(discount_type, PaymentConversionType::Unlimited) {
            conversions.push(PaymentConversion::new(
                base_resources.clone(),
                ResourcePile::empty(),
                discount_type,
            ));
        }

        PaymentOptions {
            default: cost,
            conversions,
            modifiers: vec![],
        }
    }

    #[must_use]
    pub(crate) fn resources(cost: ResourcePile) -> Self {
        Self::resources_with_discount(cost, PaymentConversionType::Unlimited)
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
            if let Some(to) = conversion.to.types().first() {
                write!(f, " > {to}")?;
            } else {
                write!(f, " > may reduce payment")?;
            }
            match conversion.payment_conversion_type {
                PaymentConversionType::Unlimited => {}
                PaymentConversionType::MayOverpay(i) => {
                    write!(f, " (up to: {i})")?;
                }
                PaymentConversionType::MayNotOverpay(i) => {
                    write!(f, " (limit: {i})")?;
                }
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
    discount_left: u8,
    may_overpay: bool,
) -> Option<ResourcePile> {
    if available.has_at_least(current) && (discount_left == 0 || may_overpay) {
        return Some(current.clone());
    }

    if conversions.is_empty() {
        return None;
    }
    let conversion = &conversions[0];
    if skip_from >= conversion.from.len() {
        return can_convert(
            available,
            current,
            &conversions[1..],
            0,
            discount_left,
            may_overpay,
        );
    }
    let from = &conversion.from[skip_from];

    let upper_limit = match conversion.payment_conversion_type {
        PaymentConversionType::Unlimited => u8::MAX,
        PaymentConversionType::MayOverpay(i) | PaymentConversionType::MayNotOverpay(i) => i,
    };

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
                may_overpay,
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
            may_overpay,
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
                payment_conversion_type: PaymentConversionType::Unlimited,
            }],
            modifiers: vec![],
        };
        let available = ResourcePile::wood(1) + ResourcePile::ore(1);
        assert_eq!(
            cost.first_valid_payment(&available),
            Some(ResourcePile::wood(1)),
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
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
                        payment_conversion_type: PaymentConversionType::Unlimited,
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
                        payment_conversion_type: PaymentConversionType::Unlimited,
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
                        payment_conversion_type: PaymentConversionType::MayNotOverpay(1),
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
                        payment_conversion_type: PaymentConversionType::MayNotOverpay(1),
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
                        payment_conversion_type: PaymentConversionType::MayNotOverpay(2),
                    }],
                    modifiers: vec![],
                },
                valid: vec![ResourcePile::food(1)],
                invalid: vec![ResourcePile::food(2)],
            },
            ValidPaymentTestCase {
                name: "discount with overpay".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(3),
                    conversions: vec![PaymentConversion {
                        from: vec![ResourcePile::food(1)],
                        to: ResourcePile::empty(),
                        payment_conversion_type: PaymentConversionType::MayOverpay(2),
                    }],
                    modifiers: vec![],
                },
                valid: vec![
                    ResourcePile::food(1),
                    ResourcePile::food(2),
                    ResourcePile::food(3),
                ],
                invalid: vec![ResourcePile::food(0)],
            },
            ValidPaymentTestCase {
                name: "food to wood to ore".to_string(),
                options: PaymentOptions {
                    default: ResourcePile::food(1),
                    conversions: vec![
                        PaymentConversion {
                            from: vec![ResourcePile::food(1)],
                            to: ResourcePile::wood(1),
                            payment_conversion_type: PaymentConversionType::Unlimited,
                        },
                        PaymentConversion {
                            from: vec![ResourcePile::wood(1)],
                            to: ResourcePile::ore(1),
                            payment_conversion_type: PaymentConversionType::Unlimited,
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
                            payment_conversion_type: PaymentConversionType::Unlimited,
                        },
                        PaymentConversion {
                            from: vec![ResourcePile::food(1)],
                            to: ResourcePile::wood(1),
                            payment_conversion_type: PaymentConversionType::Unlimited,
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
                        payment_conversion_type: PaymentConversionType::Unlimited,
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
