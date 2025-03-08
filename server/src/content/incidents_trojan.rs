use crate::ability_initializer::AbilityInitializerSetup;
use crate::combat::{get_combat, take_combat, CombatModifier, CombatRetreatState};
use crate::combat_listeners::CombatResult;
use crate::content::advances::get_advance;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::PaymentRequest;
use crate::game::{Game, GameState};
use crate::incident::{Anarchy, Incident, IncidentBaseEffect, PermanentIncidentEffect};
use crate::payment::PaymentOptions;
use crate::player_events::IncidentTarget;
use crate::resource_pile::ResourcePile;
use crate::utils::remove_element_by;

pub(crate) fn trojan_incidents() -> Vec<Incident> {
    vec![trojan_horse(), solar_eclipse(), anarchy()]
}

const TROJAN_DESCRIPTION: &str = "In a land battle against a defended city (Army unit or Fortress), the attacker may pay 1 wood and 1 culture token to get 1 victory point and to deny the defender tactics cards in the first round of combat.";

fn trojan_horse() -> Incident {
    Incident::builder(
        42,
        "Trojan Horse",
        &format!("The following is available to all players: {TROJAN_DESCRIPTION}"),
        IncidentBaseEffect::BarbariansMove,
    )
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _, _| {
        game.permanent_incident_effects
            .push(PermanentIncidentEffect::TrojanHorse);
    })
    .build()
}

fn trojan_cost() -> PaymentOptions {
    PaymentOptions::resources(ResourcePile::wood(1) + ResourcePile::culture_tokens(1))
}

pub(crate) fn decide_trojan_horse() -> Builtin {
    Builtin::builder("Trojan Horse", TROJAN_DESCRIPTION)
        .add_payment_request_listener(
            |event| &mut event.on_combat_start,
            10,
            |game, player_index, ()| {
                if is_land_battle_against_defended_city(game, player_index) {
                    game.permanent_incident_effects.iter().find_map(|e| {
                        matches!(e, PermanentIncidentEffect::TrojanHorse).then_some(vec![
                            PaymentRequest::new(trojan_cost(), "Activate the Trojan Horse?", true),
                        ])
                    })
                } else {
                    None
                }
            },
            |game, s| {
                if s.choice[0].is_empty() {
                    game.add_info_log_item(&format!(
                        "{} declined to activate the Trojan Horse",
                        s.player_name
                    ));
                } else {
                    let player = game.get_player_mut(s.player_index);
                    player.event_victory_points += 1_f32;
                    game.add_info_log_item(&format!(
                        "{} activated the Trojan Horse and gained 1 victory point",
                        s.player_name
                    ));
                    game.permanent_incident_effects
                        .retain(|e| !matches!(e, PermanentIncidentEffect::TrojanHorse));
                    let Some(GameState::Combat(c)) = &mut game.state_stack.last_mut() else {
                        panic!("Invalid state")
                    };
                    c.modifiers.push(CombatModifier::TrojanHorse);
                }
            },
        )
        .build()
}

fn is_land_battle_against_defended_city(game: &Game, player_index: usize) -> bool {
    let combat = get_combat(game);
    !combat.is_sea_battle(game)
        && combat.attacker == player_index
        && combat.defender_city(game).is_some()
}

pub(crate) fn activate_trojan_horse() -> Builtin {
    Builtin::builder("Trojan Horse", "-")
        .add_player_event_listener(
            |event| &mut event.on_combat_round,
            10,
            |s, c, _game| {
                if s.attacker && c.round == 1 && c.modifiers.contains(&CombatModifier::TrojanHorse) {
                    s.deny_tactics.push(c.defender);
                    s.roll_log.push(
                        "Trojan Horse denies the defender tactics cards in the first round of combat"
                            .to_string(),
                    );
                }
            },
        )
        .build()
}

fn solar_eclipse() -> Incident {
    Incident::builder(
        41,
        "Solar Eclipse",
        "The next land battle will end after the first round (retreat if not finished). The winner gains 1 victory point (defender if draw).",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
        .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _, _| {
            game.permanent_incident_effects
                .push(PermanentIncidentEffect::SolarEclipse);
        })
        .build()
}

pub(crate) fn solar_eclipse_end_combat() -> Builtin {
    Builtin::builder("Solar Eclipse", "-")
        .add_player_event_listener(
            |event| &mut event.on_combat_round_end,
            10,
            |game, _c, r| {
                if let Some(p) = game
                    .permanent_incident_effects
                    .iter()
                    .position(|e| matches!(e, PermanentIncidentEffect::SolarEclipse))
                {
                    let mut c = take_combat(game);
                    if c.round == 1 && !c.is_sea_battle(game) {
                        game.permanent_incident_effects.remove(p);
                        c.retreat = CombatRetreatState::EndAfterCurrentRound;

                        let p = match &r.final_result {
                            Some(CombatResult::AttackerWins) => c.attacker,
                            _ => c.defender,
                        };
                        let p = game.get_player_mut(p);
                        p.event_victory_points += 1_f32;
                        let name = p.get_name().clone();
                        game.add_info_log_item(&format!(
                            "{name} gained 1 victory point for the Solar Eclipse",
                        ));
                    }
                    game.push_state(GameState::Combat(c));
                }
            },
        )
        .build()
}

// fn guillotine() -> Incident {

// Incident::builder(
// todo implement when leaders are implemented
//
//     43,
//     "Guillotine",
//     "Kill your leader if you have one. Then, choose one of the following: A) Choose a new leader in one of your cities or armies. B) Gain 2 victory points. You cannot play leaders for the remainder of the game."
// IncidentBaseEffect::BarbariansSpawn,
// )
// .add_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _player_index| {
//     game.permanent_incident_effects
//         .push(PermanentIncidentEffect::Guillotine);
// })
// .build()
// }

fn anarchy() -> Incident {
    Incident::builder(
        44,
        "Anarchy",
        "Set aside all government advances. Whenever you research a new government advance, take a game event token from there instead of the supply (thereby not triggering game events). Each advance left in the government advances area at the end of the game is worth 1 victory point.",
        IncidentBaseEffect::None,
    )
        .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, player_index, player_name| {
            let p = game.get_player_mut(player_index);
            let old = p.advances.len();
            p.advances.retain(|a| a.government.is_none());
            let lost = old - p.advances.len();
            p.event_victory_points += lost as f32;
            if lost > 0 {
                game.add_info_log_item(&format!(
                    "{player_name} lost {lost} government advances due to Anarchy - adding {lost} victory points",
                ));
            }

            game.permanent_incident_effects
                .push(PermanentIncidentEffect::Anarchy(Anarchy {
                    player: player_index,
                    advances_lost: lost,
                }));
        })
        .build()
}

pub(crate) fn anarchy_advance() -> Builtin {
    Builtin::builder("Anarchy", "-")
        .add_simple_current_event_listener(
            |event| &mut event.on_advance,
            10,
            |game, player_index, player_name, i| {
                if get_advance(&i.name).government.is_none() {
                    return;
                }

                if let Some(PermanentIncidentEffect::Anarchy(mut a)) = remove_element_by(&mut game
                    .permanent_incident_effects,
                    |e| matches!(e, PermanentIncidentEffect::Anarchy(_)))
                {
                    if player_index == a.player {
                        game.lock_undo(); // until generic undo is implemented
                        game.add_info_log_item(&format!(
                            "{player_name} gained a government advance, taking a game event token instead of triggering a game event (and losing 1 victory point)",
                        ));
                        let p = game.get_player_mut(player_index);
                        p.incident_tokens += 1;
                        p.event_victory_points -= 1_f32;
                        a.advances_lost -= 1;
                        if a.advances_lost > 0 {
                            game.permanent_incident_effects.push(PermanentIncidentEffect::Anarchy(a));
                        }
                    }
                }
            },
        )
        .build()
}
