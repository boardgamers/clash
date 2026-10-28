use crate::action::Action;
use crate::cache::Cache;
use crate::combat_listeners::{CombatResult, on_end_combat};
use crate::combat_stats::{Battleground, CombatPlayerStats, CombatStats};
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use crate::game::{Game, GameContext};
use crate::game_data::GameData;
use crate::log::{ActionLogAction, current_turn_log_mut};
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::unit::Units;

fn qualifies(
    defending: bool,
    result: CombatResult,
    losses: u8,
    battleground: Battleground,
    barbarians: bool,
) -> bool {
    let mut data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/test_games/tactics_cards/heavy_resistance.json"
    ))
    .unwrap();
    for p in data["players"].as_array_mut().unwrap() {
        p["objective_cards"] = serde_json::json!([]);
        p["action_cards"] = serde_json::json!([]);
        p["advances"] = serde_json::json!(["Farming", "Mining", "Tactics"]);
    }
    data["players"][0]["objective_cards"] = serde_json::json!([28]);
    if barbarians {
        data["players"][1]["civilization"] = serde_json::json!("Barbarians");
    }
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.log_index = 1;
    current_turn_log_mut(&mut game)
        .actions
        .push(ActionLogAction::new(
            Action::Playing(PlayingAction::EndTurn),
            0,
            None,
            0,
        ));
    let mut stats = CombatStats::new(
        battleground,
        false,
        CombatPlayerStats::new(
            usize::from(defending),
            Units {
                infantry: 2,
                ..Units::empty()
            },
            Position::from_offset("C2"),
        ),
        CombatPlayerStats::new(
            usize::from(!defending),
            Units {
                infantry: 3,
                ..Units::empty()
            },
            Position::from_offset("C1"),
        ),
        Some(result),
        None,
    );
    if defending {
        stats.attacker.losses.infantry = losses;
    } else {
        stats.defender.losses.infantry = losses;
    }
    on_end_combat(&mut game, stats);
    // Barbarian rewards resolve before combat objectives are checked.
    while let Some(handler) = game.current_event_handler() {
        let PersistentEventRequest::ResourceReward(request) = &handler.request else {
            break;
        };
        let response = Action::Response(EventResponse::ResourceReward(
            request.reward.payment_options.default.clone(),
        ));
        let player = game.active_player();
        game = crate::game_api::execute(game, response, player);
    }
    game.events.iter().any(|event| matches!(&event.event_type, PersistentEventType::SelectObjectives(info) if info.shown_objective.as_deref() == Some("Resistance")))
}

#[test]
fn defiance_accepts_human_and_barbarian_attackers() {
    for barbarians in [false, true] {
        assert!(qualifies(
            true,
            CombatResult::DefenderWins,
            2,
            Battleground::Land,
            barbarians
        ));
        assert!(qualifies(
            true,
            CombatResult::DefenderWins,
            2,
            Battleground::City,
            barbarians
        ));
        assert!(!qualifies(
            true,
            CombatResult::DefenderWins,
            1,
            Battleground::City,
            barbarians
        ));
        assert!(!qualifies(
            false,
            CombatResult::AttackerWins,
            2,
            Battleground::City,
            barbarians
        ));
        assert!(!qualifies(
            true,
            CombatResult::AttackerWins,
            2,
            Battleground::City,
            barbarians
        ));
    }
}
