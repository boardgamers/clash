//! Voting's activation token is separate from Rituals' city happiness payment.
use serde_json::{Value, json};
use server::{
    action::Action,
    cache::Cache,
    city::MoodState,
    content::custom_actions::{PlayingActionModifier, SpecialAction},
    events::EventOrigin,
    game::{Game, GameContext},
    game_api::execute,
    game_data::GameData,
    happiness::{IncreaseHappiness, happiness_cost},
    player::CostTrigger,
    playing_actions::{PlayingAction, PlayingActionType},
    position::Position,
    resource_pile::ResourcePile,
    web_view,
};

fn fixture(mood: u8) -> Game {
    let data: GameData = serde_json::from_str(include_str!(
        "test_games/advances/increase_happiness_voting_rituals.json"
    ))
    .unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.players[0].resources.mood_tokens = mood;
    game
}

fn voting() -> PlayingActionType {
    PlayingActionType::Special(SpecialAction::Modifier(
        PlayingActionModifier::VotingIncreaseHappiness,
    ))
}

fn cities() -> Vec<(Position, u8)> {
    vec![
        (Position::from_offset("C2"), 1),
        (Position::from_offset("B3"), 2),
    ]
}

fn preview(game: &Game, variant: PlayingActionType, payment: Option<ResourcePile>) -> Value {
    web_view::query(
        game,
        0,
        json!({
            "kind": "happiness", "variant": variant, "cities": cities(),
            "includePayments": true, "payment": payment,
        }),
    )
    .unwrap()
}

#[test]
fn voting_requires_actual_mood_in_generated_and_selected_payments() {
    let game = fixture(1);
    let cost = happiness_cost(
        0,
        5,
        CostTrigger::NoModifiers,
        &voting(),
        &game,
        &EventOrigin::Advance(server::advance::Advance::Voting),
    )
    .cost;
    assert_eq!(cost.default, ResourcePile::mood_tokens(6));
    assert_eq!(cost.separate_fee, vec![ResourcePile::mood_tokens(1)]);
    for resource in [
        ResourcePile::food(6),
        ResourcePile::wood(6),
        ResourcePile::ore(6),
        ResourcePile::ideas(6),
        ResourcePile::gold(6),
        ResourcePile::culture_tokens(6),
    ] {
        assert!(!cost.is_valid_payment(&resource));
        assert_eq!(cost.first_valid_payment(&resource), None);
    }
    let payment = ResourcePile::wood(5) + ResourcePile::mood_tokens(1);
    assert!(cost.is_valid_payment(&payment));
    assert_eq!(cost.first_valid_payment(&payment), Some(payment.clone()));
    assert_eq!(cost.first_valid_payment(&ResourcePile::wood(6)), None);
    // Mixed resources may still pay all five city-size/step tokens.
    assert!(cost.is_valid_payment(&ResourcePile::new(1, 1, 1, 1, 1, 1, 0)));
    let rejected = preview(&game, voting(), Some(ResourcePile::wood(6)));
    assert!(rejected["action"].is_null());
    let accepted = preview(&game, voting(), Some(payment.clone()));
    assert_eq!(accepted["payment"], json!(payment));
    assert_eq!(
        accepted["action"]["Playing"]["IncreaseHappiness"]["payment"],
        json!(payment)
    );
}

#[test]
fn voting_without_mood_is_unaffordable_despite_sufficient_resources() {
    let game = fixture(0);
    assert!(!server::happiness::available_happiness_actions(&game, 0).contains(&voting()));
    assert_eq!(
        web_view::query(
            &game,
            0,
            json!({
                "kind": "happiness", "variant": voting(), "cities": cities(),
            })
        )
        .unwrap_err(),
        "Happiness action unavailable"
    );
    let cost = happiness_cost(
        0,
        5,
        CostTrigger::NoModifiers,
        &voting(),
        &game,
        &EventOrigin::Advance(server::advance::Advance::Voting),
    )
    .cost;
    assert_eq!(cost.first_valid_payment(&game.player(0).resources), None);
}

#[test]
fn viewer_choices_and_generated_action_preserve_activation_token() {
    let game = fixture(1);
    let result = preview(&game, voting(), None);
    let payment: ResourcePile = serde_json::from_value(result["payment"].clone()).unwrap();
    assert_eq!(payment.mood_tokens, 1);
    assert_eq!(result["field"]["cost"], json!(ResourcePile::mood_tokens(6)));
    assert_eq!(result["field"]["initial"], json!(payment));
    // A small stock keeps the choices list below the viewer enumeration cap.
    let mut small = fixture(1);
    small.players[0].resources = ResourcePile::wood(5) + ResourcePile::mood_tokens(1);
    let choices = preview(&small, voting(), None);
    assert_eq!(
        choices["field"]["choices"],
        json!([small.player(0).resources])
    );
    let action: Action = serde_json::from_value(result["action"].clone()).unwrap();
    let actions = game.actions_left;
    let mut resources = game.player(0).resources.clone();
    let game = execute(game, action, 0);
    assert_eq!(game.actions_left, actions);
    resources -= payment;
    assert_eq!(game.player(0).resources, resources);
    assert_eq!(
        game.player(0)
            .get_city(Position::from_offset("C2"))
            .mood_state,
        MoodState::Neutral
    );
    assert_eq!(
        game.player(0)
            .get_city(Position::from_offset("B3"))
            .mood_state,
        MoodState::Happy
    );
}

#[test]
fn forged_resource_only_voting_action_is_rejected() {
    let game = fixture(1);
    let action = Action::Playing(PlayingAction::IncreaseHappiness(IncreaseHappiness::new(
        cities(),
        ResourcePile::wood(5) + ResourcePile::food(1),
        voting(),
    )));
    // Invalid resource payments use the engine's assertion-based validation.
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(game, action, 0)))
            .is_err()
    );
}

#[test]
fn payment_serialization_preserves_reserved_fee_and_legacy_defaults() {
    let game = fixture(1);
    let cost = happiness_cost(
        0,
        5,
        CostTrigger::NoModifiers,
        &voting(),
        &game,
        &EventOrigin::Advance(server::advance::Advance::Voting),
    )
    .cost;
    let value = serde_json::to_value(&cost).unwrap();
    assert_eq!(value["separate_fee"], json!([ResourcePile::mood_tokens(1)]));
    let roundtrip: server::payment::PaymentOptions = serde_json::from_value(value).unwrap();
    assert_eq!(roundtrip, cost);
    assert!(!roundtrip.is_valid_payment(&ResourcePile::wood(6)));

    let normal = happiness_cost(
        0,
        5,
        CostTrigger::NoModifiers,
        &PlayingActionType::IncreaseHappiness,
        &game,
        &EventOrigin::Ability("Increase Happiness".into()),
    )
    .cost;
    let value = serde_json::to_value(&normal).unwrap();
    assert!(value.get("separate_fee").is_none());
    let legacy: server::payment::PaymentOptions = serde_json::from_value(value).unwrap();
    assert!(legacy.separate_fee.is_empty());
    assert!(legacy.is_valid_payment(&ResourcePile::wood(5)));
}

#[test]
fn normal_happiness_with_rituals_needs_no_mood() {
    let game = fixture(0);
    let payment = ResourcePile::wood(5);
    let result = preview(
        &game,
        PlayingActionType::IncreaseHappiness,
        Some(payment.clone()),
    );
    assert_eq!(result["payment"], json!(payment));
    assert_eq!(result["field"]["cost"], json!(ResourcePile::mood_tokens(5)));
    let action: Action = serde_json::from_value(result["action"].clone()).unwrap();
    let actions = game.actions_left;
    let mut resources = game.player(0).resources.clone();
    let game = execute(game, action, 0);
    assert_eq!(game.actions_left, actions - 1);
    resources -= payment;
    assert_eq!(game.player(0).resources, resources);
    assert_eq!(game.player(0).resources.mood_tokens, 0);
}

#[test]
fn colosseum_can_convert_voting_fee_but_rituals_still_cannot() {
    let mut data: Value = serde_json::from_str(include_str!(
        "test_games/advances/increase_happiness_voting_rituals.json"
    ))
    .unwrap();
    data["players"][0]["cities"][0]["city_pieces"]["wonders"] = json!(["Colosseum"]);
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    let payment = ResourcePile::wood(5) + ResourcePile::culture_tokens(1);
    game.players[0].resources = payment.clone();
    let cost = happiness_cost(
        0,
        5,
        CostTrigger::NoModifiers,
        &voting(),
        &game,
        &EventOrigin::Advance(server::advance::Advance::Voting),
    )
    .cost;
    assert_eq!(
        cost.separate_fee,
        vec![
            ResourcePile::mood_tokens(1),
            ResourcePile::culture_tokens(1)
        ]
    );
    assert!(cost.is_valid_payment(&payment));
    assert_eq!(cost.first_valid_payment(&payment), Some(payment.clone()));
    assert!(cost.is_valid_payment(&ResourcePile::culture_tokens(6)));
    assert!(!cost.is_valid_payment(&ResourcePile::wood(6)));
    assert_eq!(cost.first_valid_payment(&ResourcePile::wood(6)), None);
    assert!(!cost.is_valid_payment(&(payment.clone() + ResourcePile::wood(1))));
    let roundtrip: server::payment::PaymentOptions = serde_json::from_value(json!(cost)).unwrap();
    assert!(roundtrip.is_valid_payment(&payment));
    assert!(!roundtrip.is_valid_payment(&ResourcePile::wood(6)));
    assert!(server::happiness::available_happiness_actions(&game, 0).contains(&voting()));
    let result = preview(&game, voting(), Some(payment.clone()));
    assert_eq!(result["payment"], json!(payment));
    assert_eq!(result["field"]["choices"], json!([payment]));
    let generated = preview(&game, voting(), None);
    assert_eq!(generated["payment"], json!(payment));
    let action: Action = serde_json::from_value(result["action"].clone()).unwrap();
    let actions = game.actions_left;
    let game = execute(game, action, 0);
    assert_eq!(game.actions_left, actions);
    assert_eq!(game.player(0).resources, ResourcePile::empty());
}
