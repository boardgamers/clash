use common::{JsonTest, move_action};
use serde_json::Value;
use server::action::Action;
use server::city::MoodState;
use server::content::persistent_events::{EventResponse, PersistentEventType};
use server::game::Game;
use server::game_api::execute;
use server::position::Position;
use server::unit::UnitType;
use server::wonder::Wonder;

mod common;

const WALL_LOG: &str = "Great Wall gives -2 combat value in the first round";

fn setup() -> Game {
    let mut game = JsonTest::new("wonders").load_game("great_wall");
    // Two infantry on each side; 3/Elephant faces have no matching ability.
    // The unmodified rolls are 6 CV each, so -2 changes the number of hits.
    for player in &mut game.players[..2] {
        player.action_cards.clear();
        for unit in &mut player.units {
            unit.unit_type = UnitType::Infantry;
        }
    }
    game.dice_roll_outcomes = vec![4; 4];
    game
}

fn attack(game: Game) -> Game {
    execute(
        game,
        move_action(vec![0, 1], Position::from_offset("A1")),
        1,
    )
}

fn rolls(game: &Game) -> Vec<(usize, Value)> {
    fn collect(value: &Value, result: &mut Vec<(usize, Value)>) {
        if let Some(roll) = value.get("CombatRoll") {
            result.push((value["player"].as_u64().unwrap() as usize, roll.clone()));
        }
        match value {
            Value::Array(items) => items.iter().for_each(|item| collect(item, result)),
            Value::Object(items) => items.values().for_each(|item| collect(item, result)),
            _ => (),
        }
    }
    let mut result = vec![];
    collect(&serde_json::to_value(&game.log).unwrap(), &mut result);
    result
}

fn assert_first_rolls(game: &Game, penalty: bool) {
    let rolls = rolls(game);
    assert_eq!(rolls.len(), 2);
    for (player, roll) in rolls {
        let penalized = penalty && player == 1;
        assert_eq!(roll["combat_value"], if penalized { 4 } else { 6 });
        assert_eq!(
            roll.to_string().contains(WALL_LOG),
            penalized,
            "the penalty explanation belongs to the attacking player's roll"
        );
    }
}

#[test]
fn great_wall_penalizes_only_attacker_in_first_round_and_preserves_defender_hits() {
    let game = attack(setup());
    assert_first_rolls(&game, true);
    assert_eq!(
        game.player(1).units.len(),
        1,
        "defender still scores one hit"
    );
    assert_eq!(
        game.player(0).get_units(Position::from_offset("A1")).len(),
        2,
        "attacker's reduced roll scores no hits"
    );
    let PersistentEventType::CombatRoundEnd(end) = &game.current_event().event_type else {
        panic!("expected retreat decision");
    };
    assert_eq!(end.combat.stats.round, 1);

    // Continue with low dice, ensuring the next round has no Great Wall penalty.
    let mut game = game;
    game.dice_roll_outcomes = vec![0; 3];
    let game = execute(game, Action::Response(EventResponse::Bool(false)), 1);
    let rolls = rolls(&game);
    assert_eq!(rolls.len(), 4);
    for (player, roll) in &rolls[2..] {
        assert_eq!(roll["combat_value"], if *player == 1 { 1 } else { 2 });
        assert!(!roll.to_string().contains(WALL_LOG));
    }
}

#[test]
fn great_wall_does_not_apply_in_neutral_or_angry_cities() {
    for mood in [MoodState::Neutral, MoodState::Angry] {
        let mut game = setup();
        game.player_mut(0)
            .get_city_mut(Position::from_offset("A1"))
            .mood_state = mood;
        assert_first_rolls(&attack(game), false);
    }
}

#[test]
fn great_wall_protects_other_happy_cities_not_just_its_own_city() {
    let mut game = setup();
    let wall_city = Position::from_offset("A1");
    let other_city = Position::from_offset("C2");
    // Keep the wonder owned and its listener installed, but put its model elsewhere.
    game.player_mut(0)
        .get_city_mut(wall_city)
        .pieces
        .wonders
        .clear();
    game.player_mut(0)
        .get_city_mut(other_city)
        .pieces
        .wonders
        .push(Wonder::GreatWall);
    assert_first_rolls(&attack(game), true);
}

#[test]
fn great_wall_does_not_protect_an_army_outside_a_city() {
    let mut game = setup();
    game.player_mut(0)
        .get_city_mut(Position::from_offset("A1"))
        .position = Position::from_offset("B2");
    assert_first_rolls(&attack(game), false);
}

#[test]
fn great_wall_has_no_effect_when_its_owner_attacks() {
    let mut game = setup();
    game.player_mut(0).cities[0].pieces.wonders.clear();
    game.player_mut(1).cities[0]
        .pieces
        .wonders
        .push(Wonder::GreatWall);
    let data = game.cloned_data();
    let cache = server::cache::Cache::new(&data.options);
    let game = Game::from_data(data, cache, server::game::GameContext::Play);
    assert_first_rolls(&attack(game), false);
}
