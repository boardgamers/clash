use crate::resource::ResourceType;
use crate::resource_pile::{CostWithDiscount, ResourcePile};
use std::fmt::Display;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SumPaymentOptions {
    pub cost: u32,
    pub types_by_preference: &'static [ResourceType],
}

impl SumPaymentOptions {
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
    pub const fn empty() -> Self {
        Self::resources(ResourcePile::empty())
    }

    #[must_use]
    pub const fn sum(cost: u32, types_by_preference: &'static [ResourceType]) -> Self {
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

    #[must_use]
    pub fn is_free(&self) -> bool {
        self.is_valid_payment(&ResourcePile::empty())
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
