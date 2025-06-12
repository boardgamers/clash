use crate::ability_initializer::SelectedChoice;
use crate::events::{EventOrigin, EventPlayer, check_event_origin};
use crate::player::Player;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::wonder::Wonder;
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
    pub origin: EventOrigin,
}

impl PaymentOptions {
    #[must_use]
    pub fn new(
        default: ResourcePile,
        conversions: Vec<PaymentConversion>,
        modifiers: Vec<EventOrigin>,
        origin: EventOrigin,
    ) -> Self {
        PaymentOptions {
            default,
            conversions,
            modifiers,
            origin,
        }
    }

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
        Self::fixed_resources(ResourcePile::empty(), check_event_origin())
    }

    #[must_use]
    pub(crate) fn sum(
        player: &Player,
        origin: EventOrigin,
        cost: u8,
        types_by_preference: &[ResourceType],
    ) -> Self {
        payment_options_sum(cost, types_by_preference, origin.clone())
            .add_extra_options(player, origin)
    }

    #[must_use]
    pub(crate) fn single_type(
        player: &Player,
        origin: EventOrigin,
        t: ResourceType,
        r: RangeInclusive<u8>,
    ) -> PaymentOptions {
        let max = r.clone().max().expect("range empty");
        let d = max - r.min().expect("range empty");
        PaymentOptions::new(
            ResourcePile::of(t, max),
            vec![PaymentConversion::new(
                vec![ResourcePile::of(t, 1)],
                ResourcePile::empty(),
                PaymentConversionType::MayOverpay(d),
            )],
            vec![],
            origin.clone(),
        )
        .add_extra_options(player, origin)
    }

    fn add_extra_options(mut self, player: &Player, _origin: EventOrigin) -> Self {
        if player.wonders_owned.contains(Wonder::Colosseum) {
            self.conversions.push(PaymentConversion::unlimited(
                ResourcePile::of(ResourceType::CultureTokens, 1),
                ResourcePile::of(ResourceType::MoodTokens, 1),
            ));
            self.conversions.push(PaymentConversion::unlimited(
                ResourcePile::of(ResourceType::MoodTokens, 1),
                ResourcePile::of(ResourceType::CultureTokens, 1),
            ));
        }
        self
    }

    #[must_use]
    pub(crate) fn tokens(player: &Player, origin: EventOrigin, cost: u8) -> Self {
        Self::sum(player, origin, cost, &[
            ResourceType::MoodTokens,
            ResourceType::CultureTokens,
        ])
    }

    #[must_use]
    pub(self) fn resources_with_discount(
        cost: ResourcePile,
        discount_type: PaymentConversionType,
        origin: EventOrigin,
    ) -> Self {
        let base = base_resources();

        let mut conversions = vec![PaymentConversion::new(
            base.clone(),
            ResourcePile::gold(1),
            PaymentConversionType::Unlimited,
        )];
        if !matches!(discount_type, PaymentConversionType::Unlimited) {
            conversions.push(PaymentConversion::new(
                base.clone(),
                ResourcePile::empty(),
                discount_type,
            ));
        }

        PaymentOptions::new(cost, conversions, vec![], origin)
    }

    #[must_use]
    pub(crate) fn resources(player: &Player, origin: EventOrigin, cost: ResourcePile) -> Self {
        Self::resources_with_discount(cost, PaymentConversionType::Unlimited, origin.clone())
            .add_extra_options(player, origin)
    }

    #[must_use]
    pub(crate) fn fixed_resources(cost: ResourcePile, origin: EventOrigin) -> Self {
        Self::resources_with_discount(cost, PaymentConversionType::Unlimited, origin)
    }

    #[must_use]
    pub fn can_afford(&self, available: &ResourcePile) -> bool {
        self.is_free() || self.first_valid_payment(available).is_some()
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ResourceReward {
    #[serde(flatten)]
    pub payment_options: PaymentOptions,
}

impl ResourceReward {
    #[must_use]
    pub(crate) fn sum(cost: u8, types_by_preference: &[ResourceType], origin: EventOrigin) -> Self {
        Self {
            payment_options: payment_options_sum(cost, types_by_preference, origin),
        }
    }

    #[must_use]
    pub(crate) fn tokens(gain: u8, origin: EventOrigin) -> Self {
        Self::sum(
            gain,
            &[ResourceType::MoodTokens, ResourceType::CultureTokens],
            origin,
        )
    }

    pub(crate) fn selected_choice(
        &self,
        player: &EventPlayer,
        r: ResourcePile,
        actively_selected: bool,
    ) -> SelectedChoice<ResourcePile> {
        SelectedChoice::new(
            &player.with_origin(self.payment_options.origin.clone()),
            actively_selected,
            r,
        )
    }
}

pub(crate) struct PaymentOptionsBuilder {
    pub origin: EventOrigin,
}

impl PaymentOptionsBuilder {
    #[must_use]
    pub fn new(origin: EventOrigin) -> Self {
        PaymentOptionsBuilder { origin }
    }

    #[must_use]
    pub fn resources(self, player: &Player, cost: ResourcePile) -> PaymentOptions {
        PaymentOptions::resources(player, self.origin, cost)
    }

    #[must_use]
    pub fn tokens(self, player: &Player, cost: u8) -> PaymentOptions {
        PaymentOptions::tokens(player, self.origin, cost)
    }

    #[must_use]
    pub fn sum(
        self,
        player: &Player,
        cost: u8,
        types_by_preference: &[ResourceType],
    ) -> PaymentOptions {
        PaymentOptions::sum(player, self.origin, cost, types_by_preference)
    }

    #[must_use]
    pub fn single_type(
        self,
        player: &Player,
        t: ResourceType,
        r: RangeInclusive<u8>,
    ) -> PaymentOptions {
        PaymentOptions::single_type(player, self.origin, t, r)
    }
}

pub(crate) struct RewardBuilder {
    pub origin: EventOrigin,
}

impl RewardBuilder {
    #[must_use]
    pub fn new(origin: EventOrigin) -> Self {
        RewardBuilder { origin }
    }

    #[must_use]
    pub fn sum(self, gain: u8, types_by_preference: &[ResourceType]) -> ResourceReward {
        ResourceReward::sum(gain, types_by_preference, self.origin)
    }

    #[must_use]
    pub fn tokens(self, gain: u8) -> ResourceReward {
        ResourceReward::tokens(gain, self.origin)
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

pub(crate) fn payment_options_sum(
    cost: u8,
    types_by_preference: &[ResourceType],
    origin: EventOrigin,
) -> PaymentOptions {
    let mut conversions = vec![];
    types_by_preference.windows(2).for_each(|pair| {
        conversions.push(PaymentConversion::unlimited(
            ResourcePile::of(pair[0], 1),
            ResourcePile::of(pair[1], 1),
        ));
    });
    PaymentOptions::new(
        ResourcePile::of(types_by_preference[0], cost),
        conversions,
        vec![],
        origin,
    )
}

pub(crate) fn base_resources() -> Vec<ResourcePile> {
    vec![
        ResourcePile::food(1),
        ResourcePile::wood(1),
        ResourcePile::ore(1),
        ResourcePile::ideas(1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::check_event_origin;

    struct ValidPaymentTestCase {
        name: String,
        options: PaymentOptions,
        valid: Vec<ResourcePile>,
        invalid: Vec<ResourcePile>,
    }

    fn assert_can_afford(name: &str, cost: &ResourcePile, discount: u8) {
        let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
        let can_afford = PaymentOptions::resources_with_discount(
            cost.clone(),
            PaymentConversionType::MayNotOverpay(discount),
            check_event_origin(),
        )
        .can_afford(&player_has);
        assert!(can_afford, "{name}");
    }

    fn assert_cannot_afford(name: &str, cost: &ResourcePile, discount: u8) {
        let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
        let can_afford = PaymentOptions::resources_with_discount(
            cost.clone(),
            PaymentConversionType::MayNotOverpay(discount),
            check_event_origin(),
        )
        .can_afford(&player_has);
        assert!(!can_afford, "{name}");
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
    fn test_find_valid_payment() {
        let cost = PaymentOptions::new(
            ResourcePile::food(1),
            vec![PaymentConversion::new(
                vec![ResourcePile::food(1)],
                ResourcePile::wood(1),
                PaymentConversionType::Unlimited,
            )],
            vec![],
            check_event_origin(),
        );
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
                options: PaymentOptions::new(
                    ResourcePile::food(1),
                    vec![],
                    vec![],
                    check_event_origin(),
                ),
                valid: vec![ResourcePile::food(1)],
                invalid: vec![ResourcePile::food(2)],
            },
            ValidPaymentTestCase {
                name: "food to wood".to_string(),
                options: PaymentOptions::new(
                    ResourcePile::food(1),
                    vec![PaymentConversion::new(
                        vec![ResourcePile::food(1)],
                        ResourcePile::wood(1),
                        PaymentConversionType::Unlimited,
                    )],
                    vec![],
                    check_event_origin(),
                ),
                valid: vec![ResourcePile::food(1), ResourcePile::wood(1)],
                invalid: vec![ResourcePile::food(2), ResourcePile::ore(1)],
            },
            ValidPaymentTestCase {
                name: "food to wood with amount".to_string(),
                options: PaymentOptions::new(
                    ResourcePile::food(2),
                    vec![PaymentConversion::new(
                        vec![ResourcePile::food(1)],
                        ResourcePile::wood(1),
                        PaymentConversionType::Unlimited,
                    )],
                    vec![],
                    check_event_origin(),
                ),
                valid: vec![
                    ResourcePile::food(2),
                    ResourcePile::food(1) + ResourcePile::wood(1),
                    ResourcePile::wood(2),
                ],
                invalid: vec![ResourcePile::ore(2)],
            },
            ValidPaymentTestCase {
                name: "food to wood with limit".to_string(),
                options: PaymentOptions::new(
                    ResourcePile::food(2),
                    vec![PaymentConversion::new(
                        vec![ResourcePile::food(1)],
                        ResourcePile::wood(1),
                        PaymentConversionType::MayNotOverpay(1),
                    )],
                    vec![],
                    check_event_origin(),
                ),
                valid: vec![
                    ResourcePile::food(2),
                    ResourcePile::wood(1) + ResourcePile::food(1),
                ],
                invalid: vec![ResourcePile::wood(2)],
            },
            ValidPaymentTestCase {
                name: "3 infantry with draft".to_string(),
                options: PaymentOptions::new(
                    ResourcePile::food(3) + ResourcePile::ore(3),
                    vec![PaymentConversion::new(
                        vec![ResourcePile::food(1) + ResourcePile::ore(1)],
                        ResourcePile::mood_tokens(1),
                        PaymentConversionType::MayNotOverpay(1),
                    )],
                    vec![],
                    check_event_origin(),
                ),
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
                options: PaymentOptions::new(
                    ResourcePile::food(3),
                    vec![PaymentConversion::new(
                        vec![ResourcePile::food(1)],
                        ResourcePile::empty(),
                        PaymentConversionType::MayNotOverpay(2),
                    )],
                    vec![],
                    check_event_origin(),
                ),
                valid: vec![ResourcePile::food(1)],
                invalid: vec![ResourcePile::food(2)],
            },
            ValidPaymentTestCase {
                name: "discount with overpay".to_string(),
                options: PaymentOptions::new(
                    ResourcePile::food(3),
                    vec![PaymentConversion::new(
                        vec![ResourcePile::food(1)],
                        ResourcePile::empty(),
                        PaymentConversionType::MayOverpay(2),
                    )],
                    vec![],
                    check_event_origin(),
                ),
                valid: vec![
                    ResourcePile::food(1),
                    ResourcePile::food(2),
                    ResourcePile::food(3),
                ],
                invalid: vec![ResourcePile::food(0)],
            },
            ValidPaymentTestCase {
                name: "food to wood to ore".to_string(),
                options: PaymentOptions::new(
                    ResourcePile::food(1),
                    vec![
                        PaymentConversion::new(
                            vec![ResourcePile::food(1)],
                            ResourcePile::wood(1),
                            PaymentConversionType::Unlimited,
                        ),
                        PaymentConversion::new(
                            vec![ResourcePile::wood(1)],
                            ResourcePile::ore(1),
                            PaymentConversionType::Unlimited,
                        ),
                    ],
                    vec![],
                    check_event_origin(),
                ),
                valid: vec![
                    ResourcePile::food(1),
                    ResourcePile::wood(1),
                    ResourcePile::ore(1),
                ],
                invalid: vec![ResourcePile::ideas(2)],
            },
            ValidPaymentTestCase {
                name: "food to wood to ore with reversed conversion order".to_string(),
                options: PaymentOptions::new(
                    ResourcePile::food(1),
                    vec![
                        PaymentConversion::new(
                            vec![ResourcePile::wood(1)],
                            ResourcePile::ore(1),
                            PaymentConversionType::Unlimited,
                        ),
                        PaymentConversion::new(
                            vec![ResourcePile::food(1)],
                            ResourcePile::wood(1),
                            PaymentConversionType::Unlimited,
                        ),
                    ],
                    vec![],
                    check_event_origin(),
                ),
                valid: vec![
                    ResourcePile::food(1),
                    ResourcePile::wood(1),
                    ResourcePile::ore(1),
                ],
                invalid: vec![ResourcePile::ideas(2)],
            },
            ValidPaymentTestCase {
                name: "gold can replace anything but mood and culture tokens".to_string(),
                options: PaymentOptions::new(
                    ResourcePile::food(1) + ResourcePile::wood(1) + ResourcePile::mood_tokens(1),
                    vec![PaymentConversion::new(
                        vec![
                            ResourcePile::food(1),
                            ResourcePile::wood(1),
                            ResourcePile::ore(1),
                            ResourcePile::ideas(1),
                        ],
                        ResourcePile::gold(1),
                        PaymentConversionType::Unlimited,
                    )],
                    vec![],
                    check_event_origin(),
                ),
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
