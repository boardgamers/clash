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
    defenders: Units,
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
    data["players"][0]["objective_cards"] = serde_json::json!([24]);
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
    let stats = CombatStats::new(
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
            defenders,
            Position::from_offset("C1"),
        ),
        Some(result),
        None,
    );
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
    game.events.iter().any(|event| matches!(&event.event_type, PersistentEventType::SelectObjectives(info) if info.shown_objective.as_deref() == Some("Barbarian Conquest")))
}

#[test]
fn defending_a_city_against_barbarians_never_offers_assimilator() {
    assert!(!qualifies(
        true,
        CombatResult::DefenderWins,
        Units {
            infantry: 2,
            ..Units::empty()
        },
        Battleground::City,
        true
    ));
}

#[test]
fn assimilator_requires_winning_as_attacker_against_two_barbarian_army_units_in_a_city() {
    let army = Units {
        infantry: 1,
        cavalry: 1,
        ..Units::empty()
    };
    for battleground in [Battleground::City, Battleground::CityWithFortress] {
        assert!(qualifies(
            false,
            CombatResult::AttackerWins,
            army.clone(),
            battleground,
            true
        ));
        assert!(!qualifies(
            false,
            CombatResult::DefenderWins,
            army.clone(),
            battleground,
            true
        ));
        assert!(!qualifies(
            false,
            CombatResult::Draw,
            army.clone(),
            battleground,
            true
        ));
        assert!(!qualifies(
            false,
            CombatResult::AttackerWins,
            army.clone(),
            battleground,
            false
        ));
    }
    assert!(!qualifies(
        false,
        CombatResult::AttackerWins,
        army,
        Battleground::Land,
        true
    ));
    assert!(!qualifies(
        false,
        CombatResult::AttackerWins,
        Units {
            infantry: 1,
            settlers: 1,
            ..Units::empty()
        },
        Battleground::City,
        true
    ));
}
