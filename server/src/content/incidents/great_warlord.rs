use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::ActionCard;
use crate::combat::CombatModifier;
use crate::content::ability::Ability;
use crate::content::advances::AdvanceGroup;
use crate::content::incidents::great_persons::{
    great_person_action_card, great_person_description,
};
use crate::game::GameState;
use crate::game::GameState::Movement;
use crate::movement::MoveState;
use std::mem;

pub(crate) fn great_warlord() -> ActionCard {
    let groups = vec![AdvanceGroup::Warfare];
    great_person_action_card(
        24,
        "Great Warlord",
        &format!(
            "{} Then, gain a Move action. On the first battle you fight, \
            gain 2 combat value in every round.",
            great_person_description(&groups)
        ),
        |c| c.action().no_resources(),
        groups,
        |_game, _player| true,
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.play_action_card,
        0,
        |game, _player_index, _| {
            game.state = GameState::Movement(MoveState {
                great_warlord_used: true,
                ..MoveState::default()
            });
        },
    )
    .build()
}

pub(crate) fn use_great_warlord() -> Ability {
    Ability::builder("great_warlord", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_start,
            9,
            |game, _player_index, c| {
                if let Movement(m) = &mut game.state {
                    if mem::replace(&mut m.great_warlord_used, false) {
                        c.modifiers.push(CombatModifier::GreatWarlord);
                    }
                }
            },
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_round_start,
            9,
            |_game, p, r| {
                if r.combat.modifiers.contains(&CombatModifier::GreatWarlord)
                    && r.combat.attacker() == p.index
                {
                    r.attacker_strength.extra_combat_value += 2;
                    r.attacker_strength
                        .roll_log
                        .push("Player gets +2 combat value for Great Warlord".to_string());
                }
            },
        )
        .build()
}
