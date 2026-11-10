//! Differential regression for the search pruning. Keep the original exhaustive
//! traversal here as a small-input oracle, independent of bounds/flattening.
use super::*;

fn exhaustive_conversion(
    available: &ResourcePile,
    current: &ResourcePile,
    conversions: &[&PaymentConversion],
    skip_from: usize,
    discount_left: u8,
    may_overpay: bool,
    exact: bool,
) -> Option<ResourcePile> {
    if (if exact {
        available == current
    } else {
        available.has_at_least(current)
    }) && (discount_left == 0 || may_overpay)
    {
        return Some(current.clone());
    }

    if conversions.is_empty() {
        return None;
    }
    let conversion = &conversions[0];
    if skip_from >= conversion.from.len() {
        return exhaustive_conversion(
            available,
            current,
            &conversions[1..],
            0,
            discount_left,
            may_overpay,
            exact,
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
            return exhaustive_conversion(
                available,
                current,
                conversions,
                skip_from + 1,
                discount_left,
                may_overpay,
                exact,
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

        let can = exhaustive_conversion(
            available,
            &current,
            conversions,
            skip_from + 1,
            new_discount_left,
            may_overpay,
            exact,
        );
        if can.is_some() {
            return can;
        }
    }
    // A usable conversion can still lead to an unaffordable payment. Try the
    // remaining alternatives against the original cost before giving up.
    exhaustive_conversion(
        available,
        current,
        conversions,
        skip_from + 1,
        discount_left,
        may_overpay,
        exact,
    )
}

#[test]
fn pruning_preserves_exact_and_affordable_payments() {
    let gold = PaymentConversion::resource_options(
        base_resources(),
        ResourcePile::gold(1),
        PaymentConversionType::Unlimited,
    );
    let mut architecture_from = base_resources();
    architecture_from.extend([ResourcePile::culture_tokens(1), ResourcePile::gold(1)]);
    let architecture = PaymentConversion::resource_options(
        architecture_from,
        ResourcePile::mood_tokens(1),
        PaymentConversionType::Unlimited,
    );
    let mut scenarios = vec![
        (
            ResourcePile::new(2, 1, 0, 0, 0, 0, 1),
            vec![gold.clone(), architecture],
            0,
            false,
        ),
        // Expanding, chained and reversed-order exchanges must not be mistaken
        // for fixed-total payments or currently unaffordable final payments.
        (
            ResourcePile::food(2),
            vec![
                PaymentConversion::unlimited(ResourcePile::wood(1), ResourcePile::gold(1)),
                PaymentConversion::unlimited(ResourcePile::food(1), ResourcePile::wood(2)),
            ],
            0,
            false,
        ),
        (
            ResourcePile::food(2) + ResourcePile::wood(2),
            vec![
                PaymentConversion::limited(
                    ResourcePile::food(1) + ResourcePile::wood(1),
                    ResourcePile::mood_tokens(1),
                    1,
                ),
                PaymentConversion::unlimited(ResourcePile::mood_tokens(1), ResourcePile::gold(2)),
            ],
            0,
            false,
        ),
        (
            ResourcePile::culture_tokens(2) + ResourcePile::mood_tokens(1),
            vec![
                PaymentConversion::unlimited(
                    ResourcePile::mood_tokens(1),
                    ResourcePile::culture_tokens(1),
                ),
                PaymentConversion::unlimited(
                    ResourcePile::culture_tokens(1),
                    ResourcePile::mood_tokens(1),
                ),
            ],
            0,
            false,
        ),
        (
            ResourcePile::culture_tokens(2) + ResourcePile::mood_tokens(1),
            vec![PaymentConversion::resource_options(
                vec![
                    ResourcePile::culture_tokens(1),
                    ResourcePile::mood_tokens(1),
                ],
                ResourcePile::captives(1),
                PaymentConversionType::Unlimited,
            )],
            0,
            false,
        ),
    ];
    for (discount, may_overpay) in [
        (PaymentConversionType::MayNotOverpay(1), false),
        (PaymentConversionType::MayOverpay(1), true),
    ] {
        scenarios.push((
            ResourcePile::food(2) + ResourcePile::wood(1),
            vec![
                gold.clone(),
                PaymentConversion::resource_options(
                    base_resources(),
                    ResourcePile::empty(),
                    discount,
                ),
            ],
            1,
            may_overpay,
        ));
    }
    for (current, conversions, discount_left, may_overpay) in scenarios {
        for order in conversions.iter().permutations(conversions.len()) {
            // Exhaust every small stock/payment across all six relevant types.
            for mut index in 0..3_usize.pow(6) {
                let mut pile = ResourcePile::empty();
                for resource in [
                    ResourceType::Food,
                    ResourceType::Wood,
                    ResourceType::Gold,
                    ResourceType::MoodTokens,
                    ResourceType::CultureTokens,
                    ResourceType::Captives,
                ] {
                    *pile.get_mut(&resource) = (index % 3) as u8;
                    index /= 3;
                }
                for exact in [false, true] {
                    for skip_from in [0, 1] {
                        assert_eq!(
                            find_conversion(
                                &pile,
                                &current,
                                &order,
                                skip_from,
                                discount_left,
                                may_overpay,
                                exact
                            ),
                            exhaustive_conversion(
                                &pile,
                                &current,
                                &order,
                                skip_from,
                                discount_left,
                                may_overpay,
                                exact
                            ),
                            "current={current:?}, available={pile:?}, conversions={order:?}, exact={exact}, skip={skip_from}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn alternative_costs_keep_their_own_payment_totals() {
    let mut cost = PaymentOptions::new(
        ResourcePile::food(2),
        vec![PaymentConversion::unlimited(
            ResourcePile::food(1),
            ResourcePile::gold(1),
        )],
        vec![],
        check_event_origin(),
    );
    cost.alternatives.push(ResourcePile::food(1));
    cost.alternatives.push(ResourcePile::culture_tokens(3));
    for payment in [
        ResourcePile::food(2),
        ResourcePile::food(1),
        ResourcePile::gold(2),
        ResourcePile::gold(1),
        ResourcePile::culture_tokens(3),
    ] {
        assert!(cost.is_valid_payment(&payment), "{payment}");
        let expected = if payment == ResourcePile::gold(2) {
            // The cheaper alternative is recommended, but spending two gold
            // remains an exact legal manual choice.
            ResourcePile::gold(1)
        } else {
            payment.clone()
        };
        assert_eq!(cost.first_valid_payment(&payment), Some(expected));
    }
    assert!(!cost.is_valid_payment(&ResourcePile::culture_tokens(2)));
    assert!(!cost.is_valid_payment(&ResourcePile::gold(3)));
}
