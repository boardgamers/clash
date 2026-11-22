//! Warbeasts must transform every earlier Draft/Mercenaries price branch.
use server::{
    cache::Cache,
    game::{Game, GameContext},
    game_data::GameData,
    leader::Leader,
    payment::PaymentOptions,
    player::CostTrigger,
    position::Position,
    recruit::recruit_cost_without_replaced,
    resource_pile::ResourcePile as R,
    unit::Units,
};

fn cost(
    elephants: u8,
    ships: u8,
    infantry: u8,
    market: bool,
    liberties: bool,
    leader: bool,
) -> PaymentOptions {
    cost_without_advances(elephants, ships, infantry, market, liberties, leader, &[])
}

fn cost_without_advances(
    elephants: u8,
    ships: u8,
    infantry: u8,
    market: bool,
    liberties: bool,
    leader: bool,
    omitted: &[&str],
) -> PaymentOptions {
    let mut data: serde_json::Value = serde_json::from_str(include_str!(
        "test_games/civilizations/greece/sparta_draft.json"
    ))
    .unwrap();
    let p = &mut data["players"][0];
    p["civilization"] = "Carthage".into();
    p["available_leaders"] = serde_json::json!(["Hanno"]);
    p["advances"] = serde_json::json!(["Draft", "Husbandry", "Fishing", "Tactics"]);
    if liberties {
        p["advances"]
            .as_array_mut()
            .unwrap()
            .push("CivilLiberties".into());
    }
    p["cities"][0]["city_pieces"] = serde_json::json!({"port":0,"temple":0,"fortress":0});
    p["cities"][0]["port_position"] = "C3".into();
    if market {
        p["cities"][0]["city_pieces"]["market"] = 0.into();
    }
    p["advances"]
        .as_array_mut()
        .unwrap()
        .retain(|advance| !omitted.contains(&advance.as_str().unwrap()));
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let game = Game::from_data(data, cache, GameContext::Play);
    recruit_cost_without_replaced(
        &game,
        game.player(0),
        &Units::new(
            0,
            infantry,
            ships,
            0,
            elephants,
            leader.then_some(Leader::Hanno),
        ),
        Position::from_offset("A1"),
        CostTrigger::NoModifiers,
    )
    .unwrap()
    .cost
}

fn legal(cost: &PaymentOptions, payment: R) {
    assert!(
        cost.is_valid_payment(&payment),
        "not legal: {payment:?}, cost {cost:?}"
    );
    assert_eq!(cost.first_valid_payment(&payment), Some(payment));
}

#[test]
fn warbeasts_preserves_all_mercenaries_branches_with_and_without_market() {
    for market in [false, true] {
        for infantry in [false, true] {
            let cost = cost(
                1,
                u8::from(!infantry),
                u8::from(infantry),
                market,
                false,
                false,
            );
            // One regular branch and exactly two Draft branches; no cross-product
            // of payments, duplicate alternatives, or untransformed leftovers.
            assert_eq!(cost.alternatives.len(), 2);
            let branches = std::iter::once(&cost.default)
                .chain(&cost.alternatives)
                .collect::<std::collections::HashSet<_>>();
            assert_eq!(branches.len(), 3);
            if !market {
                assert!(branches.iter().all(|price| price.gold >= 1));
            }
            legal(&cost, R::food(1) + R::gold(2));
            legal(&cost, R::food(1) + R::gold(1) + R::mood_tokens(1));
            legal(&cost, R::food(1) + R::culture_tokens(1) + R::gold(1));
            let undiscounted = R::food(2)
                + if infantry {
                    R::food(1) + R::ore(1)
                } else {
                    R::wood(2)
                };
            assert_eq!(cost.is_valid_payment(&undiscounted), market);
            assert_eq!(
                cost.is_valid_payment(&(R::food(2) + R::mood_tokens(1))),
                market
            );
        }
    }
}

#[test]
fn warbeasts_requires_each_elephant_and_mercenaries_drafts_only_one_unit() {
    let two_ships = cost(1, 2, 0, false, false, false);
    legal(&two_ships, R::food(1) + R::gold(2) + R::wood(2));
    assert!(!two_ships.is_valid_payment(&(R::food(1) + R::gold(3))));
    let two_elephants = cost(2, 1, 0, false, false, false);
    legal(&two_elephants, R::food(2) + R::gold(3));
    legal(
        &two_elephants,
        R::food(2) + R::culture_tokens(2) + R::mood_tokens(1),
    );
    assert!(!two_elephants.is_valid_payment(&(R::food(3) + R::gold(1) + R::mood_tokens(1))));
    assert!(!two_elephants.is_valid_payment(&(R::food(4) + R::mood_tokens(1))));
}

#[test]
fn warbeasts_keeps_civil_liberties_mixed_tokens_and_reserved_leader_mood() {
    for infantry in [false, true] {
        let cost = cost(
            1,
            u8::from(!infantry),
            u8::from(infantry),
            false,
            true,
            true,
        );
        legal(
            &cost,
            R::food(1) + R::gold(2) + R::mood_tokens(2) + R::culture_tokens(1),
        );
        legal(
            &cost,
            R::food(1) + R::gold(3) + R::mood_tokens(1) + R::culture_tokens(1),
        );
        assert!(!cost.is_valid_payment(&(R::food(1) + R::gold(4) + R::culture_tokens(1))));
    }
}

#[test]
fn ship_recommends_discount_but_regular_default_and_elephant_only_remain_unchanged() {
    let ship = cost(0, 1, 0, false, false, false);
    assert_eq!(ship.default, R::wood(2));
    legal(&ship, R::gold(1));
    legal(&ship, R::mood_tokens(1));
    assert_eq!(
        ship.first_valid_payment(&(R::wood(2) + R::gold(2))),
        Some(R::gold(1))
    );
    let elephant = cost(1, 0, 0, false, false, false);
    assert_eq!(elephant.default, R::food(1) + R::gold(1));
    assert!(elephant.alternatives.is_empty());
    legal(&elephant, R::food(1) + R::culture_tokens(1));
    assert!(!elephant.is_valid_payment(&R::food(2)));
}

#[test]
fn rich_stocks_recommend_fewer_items_then_gold_then_native_order() {
    for market in [false, true] {
        let cost = cost(1, 1, 0, market, false, false);
        let regular = cost.default.clone();
        for (stock, expected) in [
            (
                R::food(3) + R::wood(2) + R::gold(3),
                if market {
                    R::food(2) + R::gold(1)
                } else {
                    R::food(1) + R::gold(2)
                },
            ),
            (
                R::food(3) + R::wood(2) + R::gold(3) + R::mood_tokens(2),
                if market {
                    R::food(2) + R::mood_tokens(1)
                } else {
                    R::food(1) + R::gold(1) + R::mood_tokens(1)
                },
            ),
            (
                R::food(3) + R::wood(2) + R::gold(3) + R::mood_tokens(2) + R::culture_tokens(2),
                if market {
                    R::food(2) + R::mood_tokens(1)
                } else {
                    R::culture_tokens(2) + R::mood_tokens(1)
                },
            ),
        ] {
            assert_eq!(cost.first_valid_payment(&stock), Some(expected.clone()));
            assert!(cost.is_valid_payment(&expected));
            assert!(
                cost.is_valid_payment(&regular),
                "manual regular remains legal"
            );
            assert_eq!(cost.default, regular);
            let mut choices = vec![R::empty(), regular.clone(), expected.clone(), R::empty()];
            cost.order_paid_choices(&mut choices, &stock);
            assert_eq!(
                choices,
                vec![R::empty(), expected, regular.clone(), R::empty()]
            );
            let once = choices.clone();
            cost.order_paid_choices(&mut choices, &stock);
            assert_eq!(choices, once);
        }
    }
}

#[test]
fn liberties_equal_quantity_keeps_native_resources_and_leader_mood_reserved() {
    let ship = cost(0, 1, 0, false, true, false);
    let stock = R::wood(2) + R::gold(3) + R::mood_tokens(3) + R::culture_tokens(3);
    assert_eq!(ship.first_valid_payment(&stock), Some(R::wood(2)));
    let with_leader = cost(1, 1, 0, false, false, true);
    let stock = R::food(2) + R::wood(2) + R::gold(4) + R::mood_tokens(3) + R::culture_tokens(3);
    let chosen = with_leader.first_valid_payment(&stock).unwrap();
    assert!(with_leader.is_valid_payment(&chosen));
    assert_eq!(chosen, R::culture_tokens(3) + R::mood_tokens(2));
    assert_eq!(ship.first_valid_payment(&R::wood(2)), Some(R::wood(2)));
    assert_eq!(ship.first_valid_payment(&R::food(1)), None);
}

#[test]
fn recommendation_metadata_round_trips_and_legacy_defaults_to_native() {
    use server::payment::PaymentRecommendation;
    let scoped = cost(0, 1, 0, false, false, false);
    assert_eq!(scoped.recommendation, PaymentRecommendation::Mercenaries);
    let mut json = serde_json::to_value(&scoped).unwrap();
    assert_eq!(
        serde_json::from_value::<PaymentOptions>(json.clone()).unwrap(),
        scoped
    );
    json.as_object_mut().unwrap().remove("recommendation");
    let legacy: PaymentOptions = serde_json::from_value(json).unwrap();
    assert_eq!(legacy.recommendation, PaymentRecommendation::Native);
    assert!(
        serde_json::to_value(&legacy)
            .unwrap()
            .get("recommendation")
            .is_none()
    );
    assert_eq!(
        legacy.first_valid_payment(&(R::wood(2) + R::gold(2))),
        Some(R::wood(2))
    );
    assert_eq!(
        cost(1, 0, 0, false, false, false).recommendation,
        PaymentRecommendation::Native
    );
}

#[test]
fn absent_mercenaries_keeps_regular_ship_and_tactics_does_not_change_draft_pricing() {
    use server::payment::PaymentRecommendation;
    let ship = cost_without_advances(0, 1, 0, false, false, false, &["Draft"]);
    assert_eq!(ship.recommendation, PaymentRecommendation::Native);
    assert_eq!(
        ship.first_valid_payment(&(R::wood(2) + R::gold(3))),
        Some(R::wood(2))
    );
    let infantry = cost_without_advances(0, 0, 1, false, false, false, &["Tactics"]);
    assert_eq!(infantry, cost(0, 0, 1, false, false, false));
    assert_eq!(
        infantry.first_valid_payment(&(R::food(1) + R::ore(1) + R::gold(2))),
        Some(R::gold(1))
    );
}
