//! Sparta substitutes Draft's tokens, never a leader's normal mood cost.
use server::{
    cache::Cache,
    game::{Game, GameContext},
    game_data::GameData,
    leader::Leader,
    player::CostTrigger,
    position::Position,
    recruit::recruit_cost_without_replaced,
    resource_pile::ResourcePile,
    unit::Units,
};

fn fixture(civil_liberties: bool) -> Game {
    fixture_with_colosseum(civil_liberties, false)
}

fn fixture_with_colosseum(civil_liberties: bool, colosseum: bool) -> Game {
    let mut data: serde_json::Value = serde_json::from_str(include_str!(
        "test_games/civilizations/greece/sparta_draft.json"
    ))
    .unwrap();
    data["players"][0]["available_leaders"] = serde_json::json!(["Alexander"]);
    if civil_liberties {
        data["players"][0]["advances"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!("CivilLiberties"));
    }
    if colosseum {
        data["players"][0]["cities"][0]["city_pieces"]["wonders"] =
            serde_json::json!(["Colosseum"]);
    }
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

#[test]
fn sparta_cannot_replace_a_leaders_mood_even_with_infantry() {
    for civil_liberties in [false, true] {
        let game = fixture(civil_liberties);
        for infantry in [0, 1] {
            let cost = recruit_cost_without_replaced(
                &game,
                game.player(0),
                &Units::new(0, infantry, 0, 0, 0, Some(Leader::Alexander)),
                Position::from_offset("A1"),
                CostTrigger::NoModifiers,
            )
            .unwrap()
            .cost;
            let resources = ResourcePile::food(infantry) + ResourcePile::ore(infantry);
            let normal =
                resources.clone() + ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1);
            assert!(cost.is_valid_payment(&normal));
            let illegal = resources + ResourcePile::culture_tokens(2);
            assert!(
                !cost.is_valid_payment(&illegal),
                "leader mood converted: infantry={infantry}, civil liberties={civil_liberties}"
            );
            if infantry > 0 {
                let draft_tokens = if civil_liberties { 2 } else { 1 };
                let legal =
                    ResourcePile::culture_tokens(1 + draft_tokens) + ResourcePile::mood_tokens(1);
                assert!(cost.is_valid_payment(&legal));
                assert_eq!(cost.first_valid_payment(&legal), Some(legal));
                let mood_draft =
                    ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1 + draft_tokens);
                assert!(cost.is_valid_payment(&mood_draft));
                let no_mood = ResourcePile::culture_tokens(2 + draft_tokens);
                assert!(!cost.is_valid_payment(&no_mood));
                assert_eq!(cost.first_valid_payment(&no_mood), None);
            }
        }
    }
}

#[test]
fn sparta_still_pays_for_one_infantry_without_a_leader() {
    for civil_liberties in [false, true] {
        let game = fixture(civil_liberties);
        let draft_tokens = if civil_liberties { 2 } else { 1 };
        for infantry in [1, 2] {
            let cost = recruit_cost_without_replaced(
                &game,
                game.player(0),
                &Units::new(0, infantry, 0, 0, 0, None),
                Position::from_offset("A1"),
                CostTrigger::NoModifiers,
            )
            .unwrap()
            .cost;
            let remainder = ResourcePile::food(infantry - 1) + ResourcePile::ore(infantry - 1);
            assert!(cost.is_valid_payment(
                &(remainder.clone() + ResourcePile::culture_tokens(draft_tokens))
            ));
            assert!(cost.is_valid_payment(&(remainder + ResourcePile::mood_tokens(draft_tokens))));
            if infantry == 2 {
                assert!(!cost.is_valid_payment(&ResourcePile::culture_tokens(2 * draft_tokens)));
            }
        }
    }
}

#[test]
fn colosseum_can_convert_the_leader_fee_without_expanding_draft() {
    for civil_liberties in [false, true] {
        let game = fixture_with_colosseum(civil_liberties, true);
        let draft_tokens = if civil_liberties { 2 } else { 1 };
        for infantry in [0, 1, 2] {
            let cost = recruit_cost_without_replaced(
                &game,
                game.player(0),
                &Units::new(0, infantry, 0, 0, 0, Some(Leader::Alexander)),
                Position::from_offset("A1"),
                CostTrigger::NoModifiers,
            )
            .unwrap()
            .cost;
            for leader_tokens in [
                ResourcePile::culture_tokens(2),
                ResourcePile::mood_tokens(2),
                ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1),
            ] {
                let normal = ResourcePile::food(infantry)
                    + ResourcePile::ore(infantry)
                    + leader_tokens.clone();
                assert!(cost.is_valid_payment(&normal), "normal: {normal:?}");
                assert_eq!(cost.first_valid_payment(&normal), Some(normal));
                if infantry > 0 {
                    for draft in [
                        ResourcePile::culture_tokens(draft_tokens),
                        ResourcePile::mood_tokens(draft_tokens),
                    ] {
                        let payment = ResourcePile::food(infantry - 1)
                            + ResourcePile::ore(infantry - 1)
                            + leader_tokens.clone()
                            + draft;
                        assert!(cost.is_valid_payment(&payment), "Draft: {payment:?}");
                        assert_eq!(cost.first_valid_payment(&payment), Some(payment));
                    }
                }
            }
            if infantry == 2 {
                let illegal = ResourcePile::culture_tokens(2 + 2 * draft_tokens);
                assert!(!cost.is_valid_payment(&illegal));
                assert_eq!(cost.first_valid_payment(&illegal), None);
            }
        }
    }
}
