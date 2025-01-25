use crate::resource::ResourceType;
use crate::resource_pile::{CostWithDiscount, ResourcePile};
use std::fmt::Display;
use std::ops::{Add, SubAssign};

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
    pub const fn free() -> Self {
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
