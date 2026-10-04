use crate::action::Action;
use crate::advance::Advance;
use crate::cache::Cache;
use crate::combat_listeners::{CombatResult, on_end_combat};
use crate::combat_stats::{Battleground, CombatPlayerStats, CombatStats};
use crate::content::persistent_events::PersistentEventType;
use crate::game::{Game, GameContext};
use crate::game_data::GameData;
use crate::log::{ActionLogAction, current_turn_log_mut};
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::unit::Units;

fn qualifies(result: CombatResult, own_units: u8, enemy_units: u8, fewer_warfare: bool) -> bool {
    let mut data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/test_games/tactics_cards/heavy_resistance.json"
    ))
    .unwrap();
    for p in data["players"].as_array_mut().unwrap() {
        p["objective_cards"] = serde_json::json!([]);
        p["action_cards"] = serde_json::json!([]);
        p["advances"] = serde_json::json!(["Farming", "Mining", "Tactics"]);
    }
    data["players"][0]["objective_cards"] = serde_json::json!([16]);
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.log_index = 1;
    if fewer_warfare {
        game.players[1].advances.insert(Advance::Draft);
    }
    current_turn_log_mut(&mut game)
        .actions
        .push(ActionLogAction::new(
            Action::Playing(PlayingAction::EndTurn),
            0,
            None,
            0,
        ));
    let stats = CombatStats::new(
        Battleground::Land,
        false,
        CombatPlayerStats::new(
            0,
            Units {
                infantry: own_units,
                ..Units::empty()
            },
            Position::from_offset("C2"),
        ),
        CombatPlayerStats::new(
            1,
            Units {
                infantry: enemy_units,
                ..Units::empty()
            },
            Position::from_offset("C1"),
        ),
        Some(result),
        None,
    );
    on_end_combat(&mut game, stats);
    game.events.iter().any(|event| matches!(&event.event_type, PersistentEventType::SelectObjectives(info) if info.shown_objective.as_deref() == Some("Bold")))
}

#[test]
fn losing_or_drawing_with_fewer_warfare_advances_never_offers_bold() {
    for result in [CombatResult::DefenderWins, CombatResult::Draw] {
        assert!(!qualifies(result.clone(), 2, 1, true));
        assert!(!qualifies(result, 1, 2, true));
    }
}

#[test]
fn winning_with_either_disadvantage_offers_bold() {
    assert!(qualifies(CombatResult::AttackerWins, 2, 1, true));
    assert!(qualifies(CombatResult::AttackerWins, 1, 2, false));
    assert!(!qualifies(CombatResult::AttackerWins, 2, 2, false));
}
