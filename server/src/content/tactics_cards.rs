use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::gain_action_card_from_pile;
use crate::combat::update_combat_strength;
use crate::combat_listeners::{CombatRoundStart, CombatStrength};
use crate::content::action_cards::get_action_card;
use crate::content::custom_phase_actions::PaymentRequest;
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::{
    CombatLocation, CombatRole, FighterRequirement, TacticsCard, TacticsCardTarget,
};
use std::vec;

///
/// # Panics
/// Panics if action card does not exist
#[must_use]
pub fn get_tactics_card(id: u8) -> TacticsCard {
    get_action_card(id)
        .tactics_card
        .expect("tactics card not found")
}
pub(crate) type TacticsCardFactory = fn(u8) -> TacticsCard;

pub(crate) fn peltasts(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
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

pub(crate) fn encircled(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
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

pub(crate) fn wedge_formation(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
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

pub(crate) fn high_morale(id: u8) -> TacticsCard {
    TacticsCard::builder(id, "High Morale", "Gain 2 combat value.")
        .add_reveal_listener(0, |_player, _game, _c, s| {
            s.extra_combat_value += 2;
            s.roll_log
                .push("High Morale added 2 combat value".to_string());
        })
        .build()
}

pub(crate) fn heavy_resistance(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
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

pub(crate) fn high_ground(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
        "High Ground",
        "Unless you attack a city: Your opponent can't use combat abilities.",
    )
    .fighter_any_requirement(&[FighterRequirement::Army, FighterRequirement::Fortress])
    .checker(|player, game, combat| {
        combat.role(player) == CombatRole::Defender || combat.defender_city(game).is_none()
    })
    .target(TacticsCardTarget::Opponent)
    .add_reveal_listener(0, |_player, _game, _combat, s| {
        s.roll_log
            .push("High Ground prevents opponent from using combat abilities".to_string());
        s.deny_combat_abilities = true;
    })
    .build()
}

pub(crate) fn surprise(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
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

pub(crate) fn siege(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
        "Siege",
        "When attacking a city: Gain 1 to your combat value. \
            Your opponent can't use combat abilities unless they pay 2 food.",
    )
    .fighter_requirement(FighterRequirement::Army)
    .role_requirement(CombatRole::Attacker)
    .location_requirement(CombatLocation::City)
    .add_reveal_listener(1, |_player, _game, _combat, s| {
        s.extra_combat_value += 1;
        s.roll_log.push("Siege added 1 to combat value".to_string());
    })
    .add_payment_request_listener(
        |event| &mut event.on_combat_round_start_tactics,
        0,
        move |game, p, s| {
            if s.is_active(p, id, TacticsCardTarget::Opponent) {
                let cost = PaymentOptions::resources(ResourcePile::food(2));
                if game.get_player(p).can_afford(&cost) {
                    return Some(vec![PaymentRequest::new(
                        cost,
                        "Pay 2 food to use combat abilities this round",
                        true,
                    )]);
                }
                apply_siege(game, s, p);
            }
            None
        },
        move |game, s, r| {
            let pile = &s.choice[0];
            if pile.is_empty() {
                apply_siege(game, r, s.player_index);
            } else {
                game.add_info_log_item(&format!(
                    "{} paid {pile} to use combat abilities",
                    s.player_name
                ));
            }
        },
    )
    .build()
}

fn apply_siege(game: &mut Game, r: &mut CombatRoundStart, player: usize) {
    update_combat_strength(
        game,
        player,
        r,
        |_game, _combat, s: &mut CombatStrength, _role| {
            s.roll_log
                .push("Siege prevents opponent from using combat abilities".to_string());
            s.deny_combat_abilities = true;
        },
    );
}

pub(crate) fn for_the_people(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
        "For the People",
        "When defending a city: Add 1 die to your roll.",
    )
    .fighter_any_requirement(&[FighterRequirement::Army, FighterRequirement::Fortress])
    .role_requirement(CombatRole::Defender)
    .location_requirement(CombatLocation::City)
    .target(TacticsCardTarget::ActivePlayer)
    .add_reveal_listener(0, |_player, _game, _c, s| {
        s.extra_dies += 1;
        s.roll_log
            .push("For The People added 1 extra die".to_string());
    })
    .build()
}

pub(crate) fn martyr(id: u8) -> TacticsCard {
    TacticsCard::builder(id, "Martyr", "todo")
        .target(TacticsCardTarget::Opponent)
        //todo add sacrifice effect, add test
        .add_veto_tactics_listener(0, move |p, game, _c, s| {
            game.add_info_log_item(&format!(
                "{} ignores the enemy tactics",
                game.player_name(p)
            ));
            s.tactics_card = None;
        })
        .build()
}
