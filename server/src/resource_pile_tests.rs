use crate::resource_pile::{PaymentOptions, ResourcePile};

fn assert_can_afford(name: &str, cost: ResourcePile) {
    let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
    assert!(player_has.can_afford(&cost), "{name}");
}

fn assert_cannot_afford(name: &str, cost: ResourcePile) {
    let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
    assert!(!player_has.can_afford(&cost), "{name}");
}

fn assert_payment_options(name: &str, cost: ResourcePile, options: PaymentOptions) {
    let budget = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
    assert_eq!(options, cost.get_payment_options(&budget), "{name}");
}

fn assert_to_string(resource_pile: ResourcePile, expected: &str) {
    assert_eq!(
        expected.to_string(),
        resource_pile.to_string(),
        "expected {expected} but found {resource_pile}"
    );
}

#[test]
fn can_afford_test() {
    assert_can_afford("use 6 gold as wood", ResourcePile::wood(7));
    assert_cannot_afford("6 gold is not enough", ResourcePile::wood(8));

    assert_cannot_afford(
        "gold cannot be converted to mood",
        ResourcePile::mood_tokens(7),
    );
    assert_cannot_afford(
        "gold cannot be converted to culture",
        ResourcePile::culture_tokens(8),
    );

    assert_can_afford(
        "negative gold means rebate",
        ResourcePile::gold(-2) + ResourcePile::wood(9),
    );
    assert_cannot_afford(
        "negative gold cannot rebate mood",
        ResourcePile::gold(-2) + ResourcePile::mood_tokens(9),
    );
    assert_cannot_afford(
        "negative gold cannot rebate culture",
        ResourcePile::gold(-2) + ResourcePile::mood_tokens(8),
    );

    assert_can_afford("payment costs gold", ResourcePile::wood(5));
    assert_cannot_afford(
        "gold cannot be converted, because it's already used for payment",
        ResourcePile::wood(7) + ResourcePile::gold(1),
    );
}

#[test]
fn resource_limit_test() {
    let mut resources = ResourcePile::new(3, 6, 9, 9, 0, 10, 6);
    resources.apply_resource_limit(&ResourcePile::new(7, 5, 7, 10, 3, 7, 6));
    assert_eq!(ResourcePile::new(3, 5, 7, 9, 0, 7, 6), resources);
}

#[test]
fn payment_options_test() {
    assert_payment_options(
        "no gold use",
        ResourcePile::new(1, 1, 3, 2, 0, 2, 4),
        PaymentOptions::new(ResourcePile::new(1, 1, 3, 2, 0, 2, 4), 5, 0),
    );
    assert_payment_options(
        "use some gold",
        ResourcePile::new(2, 2, 3, 5, 2, 0, 0),
        PaymentOptions::new(ResourcePile::new(1, 2, 3, 4, 4, 0, 0), 1, 0),
    );
    assert_payment_options(
        "jokers",
        ResourcePile::ore(4) + ResourcePile::ideas(4) + ResourcePile::gold(-3),
        PaymentOptions::new(ResourcePile::ore(3) + ResourcePile::ideas(4), 5, 2),
    )
}

#[test]
fn resource_pile_display_test() {
    assert_to_string(ResourcePile::default(), "nothing");
    assert_to_string(ResourcePile::ore(1), "1 ore");
    assert_to_string(ResourcePile::mood_tokens(2), "2 mood tokens");
    assert_to_string(
        ResourcePile::food(3) + ResourcePile::culture_tokens(1),
        "3 food and 1 culture token",
    );
    assert_to_string(
        ResourcePile::ideas(5) + ResourcePile::wood(1) + ResourcePile::gold(10),
        "1 wood, 5 ideas and 10 gold",
    );
}
