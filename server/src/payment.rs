use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SumPaymentOptions {
    pub cost: u32,
    pub types_by_preference: Vec<ResourceType>,
    pub default: ResourcePile,
    pub left: ResourcePile,
}

impl SumPaymentOptions {
    #[must_use]
    pub fn new(
        default: ResourcePile,
        left: ResourcePile,
        cost: u32,
        types_by_preference: &[ResourceType],
    ) -> Self {
        Self {
            default,
            left,
            cost,
            types_by_preference: types_by_preference.to_vec(),
        }
    }

    #[must_use]
    pub fn is_valid(&self, payment: &ResourcePile) -> bool {
        self.types_by_preference
            .iter()
            .map(|t| payment.amount(*t))
            .sum::<u32>()
            == self.cost
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum PaymentModel {
    Sum(SumPaymentOptions),
}

impl PaymentModel {
    #[must_use]
    pub fn is_valid(&self, payment: &ResourcePile) -> bool {
        match self {
            PaymentModel::Sum(options) => options.is_valid(payment),
        }
    }

    #[must_use]
    pub fn default(&self) -> &ResourcePile {
        match self {
            PaymentModel::Sum(options) => &options.default,
        }
    }
}

#[must_use]
pub fn get_sum_payment_options(
    pile: &ResourcePile,
    cost: u32,
    types_by_preference: &[ResourceType],
) -> SumPaymentOptions {
    let mut left = ResourcePile::empty();
    for t in types_by_preference {
        left.add_type(*t, pile.amount(*t) as i32);
    }
    let default_type = types_by_preference[0];
    let mut default_payment = ResourcePile::empty();

    for _ in 0..cost {
        let t = types_by_preference
            .iter()
            .find(|t| left.amount(**t) > 0)
            .unwrap_or(&default_type);
        left.add_type(*t, -1);
        default_payment.add_type(*t, 1);
    }

    SumPaymentOptions::new(default_payment, left, cost, types_by_preference)
}

#[cfg(test)]
mod tests {
    use crate::payment::{get_sum_payment_options, PaymentModel, SumPaymentOptions};
    use crate::resource::ResourceType;
    use crate::resource_pile::ResourcePile;

    fn assert_sum_payment_options(
        name: &str,
        budget: &ResourcePile,
        want_default: ResourcePile,
        left_default: ResourcePile,
    ) {
        let model = PaymentModel::Sum(get_sum_payment_options(
            budget,
            2,
            &[ResourceType::Ideas, ResourceType::Food, ResourceType::Gold],
        ));
        let want = PaymentModel::Sum(SumPaymentOptions::new(
            want_default,
            left_default,
            2,
            &[ResourceType::Ideas, ResourceType::Food, ResourceType::Gold],
        ));
        assert_eq!(model, want, "{name}");
    }

    #[test]
    fn advance_payment_options_test() {
        assert_sum_payment_options(
            "enough of all resources",
            &(ResourcePile::food(3) + ResourcePile::ideas(3) + ResourcePile::gold(3)),
            ResourcePile::ideas(2),
            ResourcePile::food(3) + ResourcePile::ideas(1) + ResourcePile::gold(3),
        );
        assert_sum_payment_options(
            "using food",
            &(ResourcePile::food(3) + ResourcePile::gold(3)),
            ResourcePile::food(2),
            ResourcePile::food(1) + ResourcePile::gold(3),
        );
        assert_sum_payment_options(
            "using 1 gold",
            &(ResourcePile::ideas(1) + ResourcePile::gold(3)),
            ResourcePile::ideas(1) + ResourcePile::gold(1),
            ResourcePile::gold(2),
        );
        assert_sum_payment_options(
            "one possible payment",
            &(ResourcePile::food(1) + ResourcePile::gold(1)),
            ResourcePile::food(1) + ResourcePile::gold(1),
            ResourcePile::empty(),
        );
    }
}
