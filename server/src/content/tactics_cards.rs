use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::combat::update_combat_strength;
use crate::combat_listeners::{CombatEventPhase, CombatRoundStart, CombatStrength};
use crate::content::custom_phase_actions::PaymentRequest;
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::{
    CombatRole, FighterRequirement, TacticsCard, TacticsCardBuilder, TacticsCardTarget,
};
use itertools::Itertools;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<TacticsCard> {
    let all: Vec<TacticsCard> = vec![
        peltasts(),
        encircled(),
        wedge_formation(),
        high_morale(),
        heavy_resistance(),
        elevated_position(),
        surprise(),
        siege(),
    ];
    assert_eq!(
        all.iter().unique_by(|i| &i.name).count(),
        all.len(),
        "action card ids are not unique"
    );
    all
}

///
/// # Panics
/// Panics if action card does not exist
#[must_use]
pub fn get_tactics_card(name: &str) -> TacticsCard {
    get_all()
        .into_iter()
        .find(|c| c.name == name)
        .expect("action card not found")
}

pub(crate) fn peltasts() -> TacticsCard {
    TacticsCard::builder(
        "Peltasts",
        "On reveal: Roll a die for each of your Army units. \
        If you rolled a 5 or 6, ignore 1 hit",
    )
    .fighter_requirement(FighterRequirement::Army)
    .add_reveal_listener(0, |player, game, combat, s| {
        for _ in &combat.fighting_units(game, player) {
            let roll = game.get_next_dice_roll().value;
            if roll >= 5 {
                s.roll_log
                    .push(format!("Peltasts rolled a {roll} and ignored a hit",));
                s.hit_cancels += 1;
                return;
            }
        }
        s.roll_log.push("Pelts rolled no 5 or 6".to_string());
    })
    .build()
}

pub(crate) fn encircled() -> TacticsCard {
    TacticsCard::builder(
        "Encircled",
        "Before removing casualties: If your opponent loses the same number of units \
        as you or more: Roll a die. On a 5 or 6, add 1 hit, which cannot be ignored",
    )
    .fighter_requirement(FighterRequirement::Army)
    .add_resolve_listener(0, |player, game, e| {
        let combat = &e.combat;
        let opponent = combat.opponent(player);
        let role = combat.role(player);
        let opponent_role = combat.role(opponent);

        let player_losses = e.casualties(role).fighters;
        let opponent_losses = e.casualties(opponent_role).fighters;
        if opponent_losses >= player_losses {
            if opponent_losses == combat.fighting_units(game, opponent).len() as u8 {
                game.add_info_log_item("Encircled cannot do damage - all units already die");
                return;
            }

            let roll = game.get_next_dice_roll().value;
            if roll >= 5 {
                game.add_info_log_item(
                    "Encircled rolled a 5 or 6 and added a hit that cannot be ignored",
                );
                e.casualties_mut(opponent_role).fighters += 1;
            } else {
                game.add_info_log_item("Encircled rolled no 5 or 6");
            }
        } else {
            game.add_info_log_item("Encircled cannot do damage - opponent has fewer losses");
        }
    })
    .build()
}

pub(crate) fn wedge_formation() -> TacticsCard {
    TacticsCard::builder(
        "Wedge Formation",
        "As attacker: Receive 1 combat value for each defending Army unit",
    )
    .fighter_requirement(FighterRequirement::Army)
    .role_requirement(CombatRole::Attacker)
    .add_reveal_listener(0, |_player, game, c, s| {
        let v = c.fighting_units(game, c.defender).len() as i8;
        s.extra_combat_value += v;
        s.roll_log
            .push(format!("Wedge Formation added {v} combat value",));
    })
    .build()
}

pub(crate) fn high_morale() -> TacticsCard {
    TacticsCard::builder("High Morale", "Gain 2 combat value.")
        .add_reveal_listener(0, |_player, _game, _c, s| {
            s.extra_combat_value += 2;
            s.roll_log
                .push("High Morale added 2 combat value".to_string());
        })
        .build()
}

pub(crate) fn heavy_resistance() -> TacticsCard {
    TacticsCard::builder(
        "Heavy Resistance",
        "Attacker gets -1 combat value for each fighting unit.",
    )
    .target(TacticsCardTarget::Opponent)
    .role_requirement(CombatRole::Defender)
    .add_reveal_listener(0, |player, game, c, s| {
        let v = c.fighting_units(game, player).len() as i8;
        s.extra_combat_value -= v;
        s.roll_log.push(format!(
            "Heavy resistance added -{v} to combat value for each unit"
        ));
    })
    .build()
}

pub(crate) fn elevated_position() -> TacticsCard {
    add_deny_tactics(
        TacticsCard::builder(
            "Elevated Position",
            "Unless you attack a city: Your opponent can't use tactics cards.",
        )
        .checker(|player, game, combat| {
            combat.role(player) == CombatRole::Defender || combat.defender_city(game).is_none()
        })
        .fighter_any_requirement(&[FighterRequirement::Army, FighterRequirement::Fortress]),
        ResourcePile::empty(),
        "Elevated Position",
    )
    .build()
}

fn add_deny_tactics(b: TacticsCardBuilder, prevent: ResourcePile, cause: &str) -> TacticsCardBuilder {
    let cause2 = cause.to_string();
    let cause3 = cause.to_string();
    b.add_payment_request_listener(
        |event| &mut event.on_combat_round_start_reveal_tactics,
        0,
        move |game, p, s| {
            if prevent.is_empty() {
                deny_tactics(game, p, s, &cause2);
                return None;
            }
            Some(vec![PaymentRequest::new(
                PaymentOptions::resources(prevent.clone()),
                "Pay 2 food to prevent opponent using combat abilities",
                true,
            )])
        },
        move |game, s, r| {
            let pile = &s.choice[0];
            if pile.is_empty() {
                deny_tactics(game, s.player_index, r, &cause3);
            } else {
                game.add_info_log_item(&format!(
                    "{} paid {pile} to use combat abilities",
                    s.player_name
                ));
            }
        },
    )
}

fn deny_tactics(game: &mut Game, p: usize, s: &mut CombatRoundStart, cause: &str) {
    let role = match s.phase {
        CombatEventPhase::RevealTacticsCardAttacker => CombatRole::Defender,
        CombatEventPhase::RevealTacticsCardDefender => CombatRole::Attacker,
        _ => panic!("unexpected phase"),
    };
    let opponent = s.combat.opponent(p);
    if s.combat.role(opponent) != role {
        return;
    }

    let opponent_name = game.player_name(opponent);
    game.add_info_log_item(&format!("{opponent_name} can't play tactics cards"));

    update_combat_strength(
        game,
        opponent,
        s,
        |_game, _combat, s: &mut CombatStrength, _role| {
            s.extra_combat_value = 0;
            s.extra_dies = 0;
            s.hit_cancels = 0;
            s.roll_log.clear();
            s.roll_log.push(format!("All combat abilities are cancelled by {cause}"));
            s.tactics_card = None
        },
    );
}

pub(crate) fn surprise() -> TacticsCard {
    TacticsCard::builder(
        "Surprise",
        "Add 1 to combat value. Draw 1 action card if you killed at least 1 unit.",
    )
    .target(TacticsCardTarget::ActivePlayer)
    .add_reveal_listener(0, |_player, _game, _c, s| {
        s.extra_combat_value += 1;
        s.roll_log
            .push("Surprise added 1 to combat value".to_string());
    })
    .add_resolve_listener(0, |player, game, e| {
        let c = &e.combat;
        if e.casualties(c.role(c.opponent(player))).fighters > 0 {
            game.add_info_log_item(&format!(
                "{} draws 1 action card for Surprise tactics",
                game.player_name(player)
            ));
            gain_action_card_from_pile(game, player);
        }
    })
    .build()
}

fn siege() -> TacticsCard {
    add_deny_tactics(
        TacticsCard::builder(
            "Siege",
            "When attacking a city: Gain 1 to your combat value. \
            Your opponent can't use tactics cards unless they pay 2 food.",
        )
        .fighter_requirement(FighterRequirement::Army)
        .role_requirement(CombatRole::Attacker)
        .checker(|_player, game, combat| combat.defender_city(game).is_some())
        .add_reveal_listener(0, |_player, _game, _combat, s| {
            s.extra_combat_value += 1;
            s.roll_log.push("Siege added 1 to combat value".to_string());
        }),
        ResourcePile::food(2),
        "Siege",
    )
    .build()
}
