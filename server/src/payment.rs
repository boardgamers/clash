use crate::resource::ResourceType;
use crate::resource_pile::{CostWithDiscount, ResourcePile};
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
    #[must_use] pub const fn resources(cost: ResourcePile) -> Self {
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
