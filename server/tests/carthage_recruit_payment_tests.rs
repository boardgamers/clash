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
fn ship_only_regular_default_and_elephant_only_remain_unchanged() {
    let ship = cost(0, 1, 0, false, false, false);
    assert_eq!(ship.default, R::wood(2));
    legal(&ship, R::gold(1));
    legal(&ship, R::mood_tokens(1));
    assert_eq!(
        ship.first_valid_payment(&(R::wood(2) + R::gold(2))),
        Some(R::wood(2))
    );
    let elephant = cost(1, 0, 0, false, false, false);
    assert_eq!(elephant.default, R::food(1) + R::gold(1));
    assert!(elephant.alternatives.is_empty());
    legal(&elephant, R::food(1) + R::culture_tokens(1));
    assert!(!elephant.is_valid_payment(&R::food(2)));
}
