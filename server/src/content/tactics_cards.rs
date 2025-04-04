use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{gain_action_card, gain_action_card_from_pile};
use crate::combat::{Combat, update_combat_strength};
use crate::combat_listeners::{CombatResult, CombatRoundStart, CombatStrength, kill_combat_units};
use crate::content::action_cards::get_action_card;
use crate::content::persistent_events::{PaymentRequest, PositionRequest, UnitsRequest};
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::{
    CombatLocation, CombatRole, FighterRequirement, TacticsCard, TacticsCardTarget,
};
use crate::utils::a_or_an;
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
        "Roll a die for each of your Army units. \
        If you rolled a 5 or 6, ignore 1 hit",
    )
    .fighter_requirement(FighterRequirement::Army)
    .add_reveal_listener(0, |player, game, combat, s| {
        for _ in &combat.fighting_units(game, player) {
            let roll = game.next_dice_roll().value;
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
        "If your opponent loses the same number of units as you or more: \
        Roll a die. On a 5 or 6, add 1 hit, which cannot be ignored",
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

            let roll = game.next_dice_roll().value;
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
        "Receive 1 combat value for each defending Army unit",
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
    .location_requirement(CombatLocation::Land)
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
        "Gain 1 to your combat value. \
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
        |event| &mut event.combat_round_start_tactics,
        0,
        move |game, p, s| {
            if s.is_active(p, id, TacticsCardTarget::Opponent) {
                let cost = PaymentOptions::resources(ResourcePile::food(2));
                if game.player(p).can_afford(&cost) {
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
    TacticsCard::builder(id, "For the People", "Add 1 die to your roll.")
        .role_requirement(CombatRole::Defender)
        .location_requirement(CombatLocation::City)
        .add_reveal_listener(0, |_player, _game, _c, s| {
            s.extra_dies += 1;
            s.roll_log
                .push("For The People added 1 extra die".to_string());
        })
        .build()
}

pub(crate) fn improved_defenses(id: u8) -> TacticsCard {
    TacticsCard::builder(id, "Improved Defenses", "Ignore 1 hit.")
        .role_requirement(CombatRole::Defender)
        .location_requirement(CombatLocation::City)
        .add_reveal_listener(0, |_player, _game, _c, s| {
            s.hit_cancels += 1;
            s.roll_log
                .push("Improved Defenses ignored 1 hit.".to_string());
        })
        .build()
}

pub(crate) fn tactical_retreat(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
        "Tactical Retreat",
        "The battle ends after all On Reveal effects are resolved. \
        Withdraw to an adjacent field without enemies. \
        The opponent is considered to have won the battle.",
    )
    .fighter_requirement(FighterRequirement::Army)
    .role_requirement(CombatRole::Defender)
    .checker(|_, game, c| !tactical_retreat_targets(c, game).is_empty())
    .add_position_request(
        |event| &mut event.combat_round_start_tactics,
        0,
        move |game, p, s| {
            (p == s.combat.defender).then_some(PositionRequest::new(
                tactical_retreat_targets(&s.combat, game),
                1..=1,
                "Select a position to withdraw to",
            ))
        },
        move |game, s, r| {
            r.final_result = Some(CombatResult::AttackerWins);
            let to = s.choice[0];
            game.add_info_log_item(&format!(
                "{} withdraws to {}",
                game.player_name(s.player_index),
                to
            ));
            for unit in game
                .player_mut(s.player_index)
                .get_units_mut(r.combat.defender_position)
            {
                unit.position = to;
            }
        },
    )
    .build()
}

fn tactical_retreat_targets(c: &Combat, game: &Game) -> Vec<Position> {
    let player = c.defender;
    c.defender_position
        .neighbors()
        .into_iter()
        .filter(|&p| game.map.is_land(p) && game.enemy_player(player, p).is_none())
        .collect()
}

pub(crate) fn defensive_formation(id: u8) -> TacticsCard {
    TacticsCard::builder(id, "Defensive Formation", "Roll 1 extra die.")
        .role_requirement(CombatRole::Defender)
        .add_reveal_listener(0, |_player, _game, _c, s| {
            s.extra_dies += 1;
            s.roll_log
                .push("Defensive Formation added 1 extra die.".to_string());
        })
        .build()
}

pub(crate) fn scout(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
        "Scout",
        "Ignore the enemy tactics card and take it to your hand.",
    )
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_round_start_reveal_tactics,
            0,
            move |game, p, name, s| {
                if s.is_active(p, id, TacticsCardTarget::ActivePlayer) {
                    update_combat_strength(game, s.combat.opponent(p), s, |game, _combat, st, _role| {
                        if let Some(tactics_card) = st.tactics_card.take() {
                            let card = get_action_card(tactics_card);
                            gain_action_card(game, p, &card);
                            game.add_info_log_item(&format!(
                                "{name} ignores the enemy tactics {} and takes it to their hand using Scout",
                                card.tactics_card.expect("tactics card not found").name
                            ));
                        } else {
                            game.add_info_log_item(&format!(
                                "{name} cannot use Scout - opponent didn't play a tactics card",
                            ));
                        }
                    });
                }
            },
        )
        .build()
}

pub(crate) fn martyr(id: u8) -> TacticsCard {
    TacticsCard::builder(id, "Martyr", "todo")
        .fighter_any_requirement(&[FighterRequirement::Army, FighterRequirement::Ship])
        .add_units_request(
            |event| &mut event.combat_round_start_tactics,
            0,
            move |game, p, s| {
                Some(UnitsRequest::new(p, s.combat.fighting_units(game, p), 1..=1, "Select a unit to sacrifice"))
            },
            move |game, s, r| {
                let unit = s.choice[0];
                game.add_info_log_item(&format!(
                    "{} sacrifices {} using Martyr",
                    game.player_name(s.player_index),
                    a_or_an(game.player(s.player_index).get_unit(unit).unit_type.name())
                ));
                kill_combat_units(game, &mut r.combat, s.player_index, &[unit]);
            },
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_round_start_reveal_tactics,
            0,
            move |game, p, name, s| {
                if s.is_active(p, id, TacticsCardTarget::Opponent) {
                    update_combat_strength(game, p, s, |game, _combat, st, _role| {
                        game.add_info_log_item(&format!(
                            "{name} cannot use their tactics card using Martyr (but it is still discarded)",
                        ));
                        st.tactics_card = None;
                    });
                }
            },
        )
        .build()
}

pub(crate) fn archers(id: u8) -> TacticsCard {
    TacticsCard::builder(
        id,
        "Archers",
        "Roll a die: On a 5 or 6, the opponent loses 1 unit immediately.",
    )
    .fighter_requirement(FighterRequirement::Army)
    .add_units_request(
        |event| &mut event.combat_round_start_tactics,
        0,
        move |game, p, s| {
            if !s.is_active(p, id, TacticsCardTarget::Opponent) {
                return None;
            }

            let roll = game.next_dice_roll().value;
            if roll >= 5 {
                game.add_info_log_item(&format!("Archers rolled a {roll} and scored a hit"));
            } else {
                game.add_info_log_item(&format!("Archers rolled a {roll} and did not score a hit"));
                return None;
            }

            Some(UnitsRequest::new(
                p,
                s.combat.fighting_units(game, p),
                1..=1,
                "Select a unit to sacrifice for Archers",
            ))
        },
        move |game, s, r| {
            let unit = s.choice[0];
            game.add_info_log_item(&format!(
                "{} sacrifices {} for Archers",
                game.player_name(s.player_index),
                a_or_an(game.player(s.player_index).get_unit(unit).unit_type.name())
            ));
            kill_combat_units(game, &mut r.combat, s.player_index, &[unit]);
        },
    )
    .build()
}

pub(crate) fn flanking(id: u8) -> TacticsCard {
    TacticsCard::builder(id, "Flanking", "Add 1 die to your roll.")
        .role_requirement(CombatRole::Attacker)
        .add_reveal_listener(0, |_player, _game, _c, s| {
            s.extra_dies += 1;
            s.roll_log.push("Flanking added 1 extra die".to_string());
        })
        .build()
}
