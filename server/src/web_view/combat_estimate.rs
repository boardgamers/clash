//! Run the real combat rules on disposable copies, with a separate random stream.
//! Unknown mandatory choices fail closed rather than producing misleading odds.
use crate::action::{Action, execute_without_undo};
use crate::combat_listeners::CombatResult;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest as Request, PersistentEventType,
};
use crate::game::{Game, GameContext};
use crate::log::current_turn_log;
use crate::movement::MovementAction;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use crate::utils::Rng;
use serde_json::{Value, json};

pub(super) fn estimate(game: &Game, seat: usize, input: &Value) -> Result<Value, String> {
    let mut visible = crate::game_api::strip_secret(game.clone(), Some(seat));
    // Replay frames have no rules effect and can dominate late-game copy costs.
    visible.board_history = Default::default();
    let game = &visible;
    let action: Action =
        serde_json::from_value(input["action"].clone()).map_err(|e| e.to_string())?;
    if seat != game.active_player() {
        return Err("Not your decision".into());
    }
    match &action {
        Action::Movement(MovementAction::Move(m)) => {
            let legal = super::actions::movement(game, seat, m.units.clone())?;
            if !legal["destinations"].as_array().is_some_and(|ds| {
                ds.iter()
                    .any(|d| d["attack"] == true && d["action"] == input["action"])
            }) {
                return Err("Choose a legal attack".into());
            }
        }
        Action::Response(_)
            if game.events.iter().any(|e| {
                matches!(
                    e.event_type,
                    PersistentEventType::CombatStart(_) | PersistentEventType::CombatRoundStart(_)
                )
            }) =>
        {
            ()
        }
        _ => return Err("Choose an attack or combat option".into()),
    }
    let count = input["samples"].as_u64().unwrap_or(32).clamp(1, 64);
    let offset = input["offset"].as_u64().unwrap_or(0).min(100_000);
    let siege = input["siege"].as_u64().unwrap_or(0).min(3);
    let steel = input["steel"].as_bool().unwrap_or(true);
    let mut wins = 0;
    let mut draws = 0;
    let mut losses = 0;
    let mut own_losses = 0;
    let mut enemy_losses = 0;
    let mut cost = ResourcePile::empty();
    for sample in offset..offset + count {
        let mut sim = game.clone();
        sim.context = GameContext::CombatPreview;
        // Never use the live seed, queued test dice, or future dice log.
        sim.rng = Rng::from_seed(0x636c6173685f6f646473_u128.wrapping_mul(u128::from(sample) + 1));
        for _ in 0..8 {
            sim.rng.next_seed();
        }
        sim.dice_roll_outcomes.clear();
        sim.dice_roll_log.clear();
        // Hidden hand slots are sentinel IDs, not playable cards. Never inspect them.
        for player in &mut sim.players {
            player.action_cards.retain(|id| *id != 0);
        }
        for event in &mut sim.events {
            if let PersistentEventType::CombatRoundStart(round) = &mut event.event_type {
                if round.attacker_strength.tactics_card == Some(0) {
                    round.attacker_strength.tactics_card = None;
                }
                if round.defender_strength.tactics_card == Some(0) {
                    round.defender_strength.tactics_card = None;
                }
            }
        }
        let start = sim.log_index;
        execute_without_undo(&mut sim, action.clone(), seat)?;
        let mut result = None;
        let mut paid = ResourcePile::empty();
        let mut used_siege = 0;
        for _ in 0..256 {
            if let Some(stats) = current_turn_log(&sim)
                .actions
                .iter()
                .skip(start)
                .find_map(|a| a.combat_stats.as_ref())
            {
                result = Some(stats.clone());
                break;
            }
            let event = sim.events.last().ok_or("No battle to estimate")?;
            if matches!(event.event_type, PersistentEventType::CombatEnd(_)) {
                return Err("Battle already ended".into());
            }
            let who = sim.active_player();
            let handler = event
                .player
                .handler
                .as_ref()
                .ok_or("Unsupported combat choice")?;
            let name = handler.origin.name(&sim);
            let response = match &handler.request {
                Request::Payment(fields) => {
                    let mut available = sim.player(who).resources.clone();
                    let mut payments = vec![];
                    for (i, field) in fields.iter().enumerate() {
                        let use_it = !field.optional
                            || (name == "Steel Weapons" && (who != seat || steel))
                            || (name == "Siegecraft" && who == seat && siege & (1 << i) != 0);
                        let payment = if use_it {
                            match field.cost.first_valid_payment(&available) {
                                Some(p) => p,
                                None if name == "Siegecraft" && who == seat => {
                                    return Err("Cannot afford this Siegecraft combination".into());
                                }
                                None if field.optional => ResourcePile::empty(),
                                None => return Err("Cannot pay combat cost".into()),
                            }
                        } else {
                            ResourcePile::empty()
                        };
                        if name == "Siegecraft" && who == seat && !payment.is_empty() {
                            used_siege |= 1 << i;
                        }
                        available -= payment.clone();
                        if who == seat && (name == "Steel Weapons" || name == "Siegecraft") {
                            paid += payment.clone();
                        }
                        payments.push(payment);
                    }
                    EventResponse::Payment(payments)
                }
                Request::SelectHandCards(r) if *r.needed.start() == 0 => {
                    EventResponse::SelectHandCards(vec![])
                }
                Request::BoolRequest(_) => EventResponse::Bool(false),
                Request::SelectUnits(r)
                    if name == "Choose Casualties"
                        || name == "Choose Casualties (carried units)" =>
                {
                    let mut units = r.request.choices.clone();
                    // Preserve leaders and expensive units. This is a policy, not optimal play.
                    units.sort_by_key(|id| match sim.player(r.player).get_unit(*id).unit_type {
                        UnitType::Settler => 0,
                        UnitType::Infantry | UnitType::Ship => 1,
                        UnitType::Cavalry => 2,
                        UnitType::Elephant => 3,
                        UnitType::Leader(_) => 4,
                    });
                    units.truncate(*r.request.needed.start() as usize);
                    EventResponse::SelectUnits(units)
                }
                _ => return Err(format!("Estimate unavailable for {name}")),
            };
            execute_without_undo(&mut sim, Action::Response(response), who)?;
        }
        let stats = result.ok_or("Combat simulation limit reached")?;
        if used_siege != siege {
            return Err("Cannot afford this Siegecraft combination".into());
        }
        let own_attacks = stats.attacker.player == seat;
        match stats.result {
            Some(CombatResult::Draw) => draws += 1,
            Some(CombatResult::AttackerWins) if own_attacks => wins += 1,
            Some(CombatResult::DefenderWins) if !own_attacks => wins += 1,
            Some(_) => losses += 1,
            None => return Err("Battle did not finish".into()),
        }
        // Stats omit some automatic passenger losses when the last ship sinks.
        // Count actual removed unit IDs instead, starting at this decision.
        let enemy = if own_attacks {
            stats.defender.player
        } else {
            stats.attacker.player
        };
        let removed = |player: usize| -> u64 {
            game.player(player)
                .units
                .iter()
                .filter(|unit| {
                    !sim.player(player)
                        .units
                        .iter()
                        .any(|remaining| remaining.id == unit.id)
                })
                .count() as u64
        };
        own_losses += removed(seat);
        enemy_losses += removed(enemy);
        cost = paid;
    }
    Ok(
        json!({"samples":count,"wins":wins,"draws":draws,"losses":losses,"ownLosses":own_losses,"enemyLosses":enemy_losses,"cost":cost}),
    )
}
