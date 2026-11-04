//! Deterministic public bonuses, not a simulated roll. The registered pure
//! listeners are shared with combat resolution; no payment or tactics is executed.
use crate::advance::Advance;
use crate::combat::{Combat, CombatModifier};
use crate::combat_listeners::CombatStrength;
use crate::content::persistent_events::PersistentEventType;
use crate::game::Game;
use serde_json::{Value, json};

fn strength(game: &Game, combat: &Combat, player: usize) -> CombatStrength {
    let mut result = CombatStrength::new();
    // Exclude tactics-dependent bonuses (e.g. Prepared) rather than assuming
    // that an unchosen or hidden card means no card will be played.
    result.tactics_card = Some(0);
    game.player(player)
        .trigger_event(|e| &e.combat_strength_preview, &mut result, combat, game);
    game.player(combat.opponent(player)).trigger_event(
        |e| &e.combat_opponent_strength_preview,
        &mut result,
        combat,
        game,
    );
    result
}

pub(super) fn describe(game: &Game) -> Value {
    let combat = game
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.event_type {
            PersistentEventType::CombatEnd(_) | PersistentEventType::CombatRoundEnd(_) => {
                Some(None)
            }
            PersistentEventType::CombatStart(c) => Some(Some(c)),
            PersistentEventType::CombatRoundStart(r) => Some(Some(&r.combat)),
            _ => None,
        })
        .flatten();
    let Some(c) = combat else {
        return Value::Null;
    };
    let side = |player, modifier| {
        let mut s = strength(game, c, player);
        let mut inactive = Vec::new();
        if c.is_land_battle(game)
            && game.player(player).can_use_advance(Advance::SteelWeapons)
            && !c.modifiers.contains(&modifier)
        {
            let mut activated = c.clone();
            activated.modifiers.push(modifier);
            let bonus =
                strength(game, &activated, player).extra_combat_value - s.extra_combat_value;
            inactive.push(format!(
                "Steel Weapons not activated: +{bonus} value if activated"
            ));
        }
        if crate::content::civilizations::persia::caps_opponent_bonus(game, c, player) {
            s.extra_combat_value = s.extra_combat_value.min(2);
            s.roll_log
                .push("Round 1: total bonus capped at +2, including dice-symbol bonuses".into());
        }
        json!({"dice": c.fighting_units(game, player).len() + usize::from(s.extra_dies),
            "extraDice":s.extra_dies,"valueBonus":s.extra_combat_value,
            "hitCancels":s.hit_cancels,"effects":s.roll_log,"inactive":inactive})
    };
    json!({"round":c.stats.round,
        "attacker":side(c.attacker(), CombatModifier::SteelWeaponsAttacker),
        "defender":side(c.defender(), CombatModifier::SteelWeaponsDefender)})
}
