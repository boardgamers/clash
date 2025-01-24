use crate::resource::ResourceType;
use crate::resource_pile::{CostWithDiscount, PaymentOptions, ResourcePile};
use std::fmt::Display;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SumPaymentOptions {
    pub cost: u32,
    pub types_by_preference: Vec<ResourceType>,
}

impl SumPaymentOptions {
    // #[must_use]
    // pub fn new(
    //     default: ResourcePile,
    //     left: ResourcePile,
    //     cost: u32,
    //     types_by_preference: &[ResourceType],
    //     can_afford: bool,
    // ) -> Self {
    //     Self {
    //         default,
    //         left,
    //         cost,
    //         types_by_preference: types_by_preference.to_vec(),
    //         can_afford,
    //     }
    // }

    #[must_use]
    pub fn is_valid_payment(&self, payment: &ResourcePile) -> bool {
        self.types_by_preference
            .iter()
            .map(|t| payment.get(*t))
            .sum::<u32>()
            == self.cost
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum PaymentModel {
    Sum(SumPaymentOptions),
    Resources(CostWithDiscount),
}

impl PaymentModel {
    #[must_use]
    pub fn sum(cost: u32, types_by_preference: &[ResourceType]) -> Self {
        get_sum_payment_model(cost, types_by_preference)
    }

    #[must_use]
    pub const fn resources_with_discount(cost: ResourcePile, discount: u32) -> Self {
        PaymentModel::Resources(CostWithDiscount { cost, discount })
    }
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
                    .map(|t| available.get(*t))
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
                    .map(|t| format!("{:?}", t))
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

#[must_use]
fn get_sum_payment_model(cost: u32, types_by_preference: &[ResourceType]) -> PaymentModel {
    PaymentModel::Sum(SumPaymentOptions {
        cost,
        types_by_preference: types_by_preference.to_vec(),
    })

    // let mut left = ResourcePile::empty();
    // for t in types_by_preference {
    //     left.add_type(*t, available.get(*t) as i32);
    // }
    // let default_type = types_by_preference[0];
    // let mut default_payment = ResourcePile::empty();
    //
    // for _ in 0..cost {
    //     let t = types_by_preference
    //         .iter()
    //         .find(|t| left.get(**t) > 0)
    //         .unwrap_or(&default_type);
    //     if left.get(*t) > 0 {
    //         left.add_type(*t, -1);
    //     }
    //     default_payment.add_type(*t, 1);
    // }
    //
    // PaymentModel::Sum(SumPaymentOptions::new(
    //     default_payment,
    //     left,
    //     cost,
    //     types_by_preference,
    //     can_afford,
    // ))
}

#[cfg(test)]
mod tests {
    use crate::payment::{get_sum_payment_model, PaymentModel, SumPaymentOptions};
    use crate::resource::ResourceType;
    use crate::resource_pile::ResourcePile;

    fn assert_sum_payment_options(
        name: &str,
        budget: &ResourcePile,
        want_default: ResourcePile,
        left_default: ResourcePile,
        can_afford: bool,
    ) {
        let model = PaymentModel::sum(
            2,
            &[ResourceType::Ideas, ResourceType::Food, ResourceType::Gold],
        );
        let want = PaymentModel::Sum(SumPaymentOptions::new(
            want_default,
            left_default,
            2,
            &[ResourceType::Ideas, ResourceType::Food, ResourceType::Gold],
            can_afford,
        ));
        assert_eq!(model, want, "{name}");
        assert_eq!(can_afford, want.can_afford(budget), "{name}");
    }

    #[test]
    fn advance_payment_options_test() {
        assert_sum_payment_options(
            "enough of all resources",
            &(ResourcePile::food(3) + ResourcePile::ideas(3) + ResourcePile::gold(3)),
            ResourcePile::ideas(2),
            ResourcePile::food(3) + ResourcePile::ideas(1) + ResourcePile::gold(3),
            true,
        );
        assert_sum_payment_options(
            "using food",
            &(ResourcePile::food(3) + ResourcePile::gold(3)),
            ResourcePile::food(2),
            ResourcePile::food(1) + ResourcePile::gold(3),
            true,
        );
        assert_sum_payment_options(
            "using 1 gold",
            &(ResourcePile::ideas(1) + ResourcePile::gold(3)),
            ResourcePile::ideas(1) + ResourcePile::gold(1),
            ResourcePile::gold(2),
            true,
        );
        assert_sum_payment_options(
            "one possible payment",
            &(ResourcePile::food(1) + ResourcePile::gold(1)),
            ResourcePile::food(1) + ResourcePile::gold(1),
            ResourcePile::empty(),
            true,
        );
        assert_sum_payment_options(
            "cannot afford",
            &(ResourcePile::gold(1)),
            ResourcePile::ideas(1) + ResourcePile::gold(1),
            ResourcePile::empty(),
            false,
        );
    }
}
