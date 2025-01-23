use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SumPaymentOptions {
    pub cost: u32,
    pub types_by_preference: Vec<ResourceType>,
    pub default: ResourcePile,
    pub left: ResourcePile,
    pub can_afford: bool,
}

impl SumPaymentOptions {
    #[must_use]
    pub fn new(
        default: ResourcePile,
        left: ResourcePile,
        cost: u32,
        types_by_preference: &[ResourceType],
        can_afford: bool,
    ) -> Self {
        Self {
            default,
            left,
            cost,
            types_by_preference: types_by_preference.to_vec(),
            can_afford,
        }
    }

    #[must_use]
    pub fn is_valid(&self, payment: &ResourcePile) -> bool {
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
}

impl PaymentModel {
    #[must_use]
    pub fn can_afford(&self) -> bool {
        match self {
            PaymentModel::Sum(options) => options.can_afford,
        }
    }

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
    
    pub fn add_type(&mut self, t: ResourceType, amount: i32) {
        match self {
            PaymentModel::Sum(options) => {
                //todo remove from left?
                options.default.add_type(t, amount);
            }
        }
    }
    
    #[must_use]
    pub fn left(&self) -> &ResourcePile {
        match self {
            PaymentModel::Sum(options) => &options.left,
        }
    }
    
    #[must_use]
    pub fn with_default(&self, default: ResourcePile) -> PaymentModel {
        match self {
            PaymentModel::Sum(options) => PaymentModel::Sum(SumPaymentOptions::new(
                default,
                options.left.clone(),
                options.cost,
                &options.types_by_preference,
                options.can_afford,
            )),
        }
    }
    
    #[must_use]
    pub fn show_types(&self) -> Vec<ResourceType> {
        match self {
            PaymentModel::Sum(options) => options.types_by_preference.clone(),
        }
    }
}

///
/// # Panics
/// Panics if the pile is empty or contains more than one resource type
#[must_use]
pub fn get_single_resource_payment_model(
    available: &ResourcePile,
    cost: &ResourcePile,
) -> PaymentModel {
    let resource_type = ResourceType::all()
        .into_iter()
        .find(|t| cost.get(*t) > 0)
        .expect("exactly one resource must be present");

    assert!(
        resource_type.is_resource(),
        "resource type must be a resource"
    );
    assert!(!resource_type.is_gold(), "resource type must not be gold");
    let amount = cost.get(resource_type);
    assert_eq!(
        cost.resource_amount(),
        amount,
        "exactly one resource must be present"
    );

    get_sum_payment_model(available, amount, &[resource_type, ResourceType::Gold])
}

#[must_use]
pub fn get_sum_payment_model(
    available: &ResourcePile,
    cost: u32,
    types_by_preference: &[ResourceType],
) -> PaymentModel {
    let mut left = ResourcePile::empty();
    for t in types_by_preference {
        left.add_type(*t, available.get(*t) as i32);
    }
    let default_type = types_by_preference[0];
    let mut default_payment = ResourcePile::empty();

    let mut can_afford = true;
    for _ in 0..cost {
        let t = types_by_preference
            .iter()
            .find(|t| left.get(**t) > 0)
            .unwrap_or(&default_type);
        if left.get(*t) == 0 {
            can_afford = false;
        } else {
            left.add_type(*t, -1);
        }
        default_payment.add_type(*t, 1);
    }

    PaymentModel::Sum(SumPaymentOptions::new(
        default_payment,
        left,
        cost,
        types_by_preference,
        can_afford,
    ))
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
        let model = get_sum_payment_model(
            budget,
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
