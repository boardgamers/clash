use server::action::Action;
use server::advance::Advance;
use server::cache::Cache;
use server::city_pieces::Building::Temple;
use server::construct::{Construct, available_buildings};
use server::content::effects::{ConstructEffect, PermanentEffect};
use server::content::persistent_events::EventResponse;
use server::events::EventOrigin;
use server::game::{Game, GameContext};
use server::game_api::execute;
use server::game_data::GameData;
use server::player::CostTrigger;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

fn fixture(civilization: &str, architecture: bool, used: bool, card: u8) -> Game {
    let mut data: serde_json::Value = serde_json::from_str(include_str!(
        "test_games/action_cards/city_development.json"
    ))
    .unwrap();
    let p = &mut data["players"][0];
    p["civilization"] = serde_json::json!(civilization);
    p["advances"] = serde_json::json!(["Farming", "Mining", "Myths", "StateReligion", "Dogma"]);
    if architecture {
        p["advances"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!("Engineering"));
    }
    p["action_cards"] = serde_json::json!([card]);
    p["incident_tokens"] = serde_json::json!(3);
    p["event_info"] = if used {
        serde_json::json!({"StateReligion": "used"})
    } else {
        serde_json::json!({})
    };
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    Game::from_data(data, cache, GameContext::Play)
}

#[test]
fn city_development_temple_keeps_state_religion_and_followups() {
    for (civilization, architecture) in [("Greece", false), ("Egypt", false), ("Egypt", true)] {
        for used in [false, true] {
            for card in [17, 18] {
                let mut game = fixture(civilization, architecture, used, card);
                let city = Position::from_offset("C1");
                let actions = game.actions_left;
                game = execute(game, Action::Playing(PlayingAction::ActionCard(card)), 0);
                game = execute(
                    game,
                    Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                        1,
                    )])),
                    0,
                );
                let before = game.player(0).resources.clone();
                let cost = available_buildings(&game, 0, city, &[])
                    .into_iter()
                    .find(|(b, _)| *b == Temple)
                    .expect("free Temple is available")
                    .1;
                assert_eq!(
                    cost.cost.first_valid_payment(&before),
                    Some(ResourcePile::empty())
                );
                assert!(cost.cost.is_valid_payment(&ResourcePile::empty()));
                assert!(cost.activate_city);
                game = execute(
                    game,
                    Action::Playing(PlayingAction::Construct(Construct::new(
                        city,
                        Temple,
                        ResourcePile::empty(),
                    ))),
                    0,
                );
                assert_eq!(game.actions_left, actions - 1);
                assert_eq!(game.player(0).resources, before);
                assert_eq!(game.player(0).get_city(city).activations, 1);
                assert_eq!(
                    game.player(0).event_info.contains_key("StateReligion"),
                    used
                );
                assert!(
                    !serde_json::to_string(&game.log)
                        .unwrap()
                        .contains("Reduce the food cost to 0")
                );
                assert!(
                    !game.permanent_effects.contains(&PermanentEffect::Construct(
                        ConstructEffect::CityDevelopment
                    ))
                );
                assert_eq!(
                    game.current_event_handler().unwrap().origin,
                    EventOrigin::Ability("Temple".to_string())
                );
                game = execute(
                    game,
                    Action::Response(EventResponse::ResourceReward(ResourcePile::mood_tokens(1))),
                    0,
                );
                assert_eq!(
                    game.current_event_handler().unwrap().origin,
                    EventOrigin::Advance(Advance::Dogma)
                );
                game = execute(
                    game,
                    Action::Response(EventResponse::SelectAdvance(Advance::Devotion)),
                    0,
                );
                assert!(game.player(0).advances.contains(Advance::Devotion));
                assert!(game.current_event_handler().is_none());
                // The next, ordinary Temple still gets the discount if it was unused.
                let cost = game
                    .player(0)
                    .building_cost(&game, Temple, CostTrigger::NoModifiers)
                    .cost;
                let payment = ResourcePile::wood(1) + ResourcePile::ore(1);
                assert_eq!(cost.is_valid_payment(&payment), !used);
                assert!(cost.is_valid_payment(&(payment + ResourcePile::food(u8::from(used)))));
            }
        }
    }
}

#[test]
fn ordinary_and_great_engineer_temples_still_use_state_religion() {
    for effect in [None, Some(ConstructEffect::GreatEngineer)] {
        for architecture in [false, true] {
            let mut game = fixture("Egypt", architecture, false, 17);
            if let Some(effect) = effect.clone() {
                game.permanent_effects
                    .push(PermanentEffect::Construct(effect));
            }
            game.player_mut(0).resources = ResourcePile::wood(1) + ResourcePile::ore(1);
            let city = Position::from_offset("C1");
            let cost = game
                .player(0)
                .building_cost(&game, Temple, CostTrigger::NoModifiers);
            assert!(!cost.cost.is_valid_payment(&ResourcePile::empty()));
            assert!(cost.cost.is_valid_payment(&game.player(0).resources));
            if architecture {
                assert!(cost.cost.is_valid_payment(&ResourcePile::mood_tokens(2)));
            }
            let actions = game.actions_left;
            let payment = game.player(0).resources.clone();
            game = execute(
                game,
                Action::Playing(PlayingAction::Construct(Construct::new(
                    city, Temple, payment,
                ))),
                0,
            );
            assert_eq!(game.actions_left, actions - 1);
            assert_eq!(
                game.player(0).get_city(city).activations,
                u32::from(effect.is_none())
            );
            assert_eq!(game.player(0).resources, ResourcePile::empty());
            assert!(game.player(0).event_info.contains_key("StateReligion"));
        }
    }
}
