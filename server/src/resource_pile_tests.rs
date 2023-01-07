use crate::resource_pile::ResourcePile;

fn assert_can_afford(name: &str, cost: ResourcePile) {
    let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
    assert!(player_has.can_afford(&cost), "{name}")
}

fn assert_cannot_afford(name: &str, cost: ResourcePile) {
    let player_has = ResourcePile::new(1, 2, 3, 4, 5, 6, 7);
    assert!(!player_has.can_afford(&cost), "{name}")
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
