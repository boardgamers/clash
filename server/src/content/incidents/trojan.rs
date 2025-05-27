use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::undo_unlock_special_advance;
use crate::advance::{find_government_special_advance, remove_advance};
use crate::combat::{Combat, CombatModifier, CombatRetreatState};
use crate::combat_listeners::CombatResult;
use crate::content::builtin::Builtin;
use crate::content::effects::{Anarchy, PermanentEffect};
use crate::content::persistent_events::PaymentRequest;
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player_events::IncidentTarget;
use crate::resource_pile::ResourcePile;
use crate::utils::remove_and_map_element_by;
use itertools::Itertools;

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
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _, _, _| {
        game.permanent_effects.push(PermanentEffect::TrojanHorse);
    })
    .build()
}

pub(crate) fn decide_trojan_horse() -> Builtin {
    Builtin::builder("Trojan Horse", TROJAN_DESCRIPTION)
        .add_payment_request_listener(
            |event| &mut event.combat_start,
            10,
            |game, player_index, c| {
                if is_land_battle_against_defended_city(game, player_index, c) {
                    game.permanent_effects.iter().find_map(|e| {
                        matches!(e, PermanentEffect::TrojanHorse).then_some(vec![
                            PaymentRequest::optional(
                                PaymentOptions::resources(
                                    game.player(player_index),
                                    PaymentReason::AdvanceAbility,
                                    ResourcePile::wood(1) + ResourcePile::culture_tokens(1),
                                ),
                                "Activate the Trojan Horse?",
                            ),
                        ])
                    })
                } else {
                    None
                }
            },
            |game, s, c| {
                if s.choice[0].is_empty() {
                    game.add_info_log_item(&format!(
                        "{} declined to activate the Trojan Horse",
                        s.player_name
                    ));
                } else {
                    let player = game.player_mut(s.player_index);
                    player.event_victory_points += 1_f32;
                    game.add_info_log_item(&format!(
                        "{} activated the Trojan Horse and gained 1 victory point",
                        s.player_name
                    ));
                    game.permanent_effects
                        .retain(|e| !matches!(e, PermanentEffect::TrojanHorse));
                    c.modifiers.push(CombatModifier::TrojanHorse);
                }
            },
        )
        .build()
}

fn is_land_battle_against_defended_city(game: &Game, player_index: usize, combat: &Combat) -> bool {
    !combat.is_sea_battle(game)
        && combat.attacker() == player_index
        && combat.defender_city(game).is_some()
}

fn solar_eclipse() -> Incident {
    Incident::builder(
        41,
        "Solar Eclipse",
        "The next land battle will end after the first round (retreat if not finished). \
        The winner gains 1 victory point (defender if draw).",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _, _, _| {
        game.permanent_effects.push(PermanentEffect::SolarEclipse);
    })
    .build()
}

pub(crate) fn solar_eclipse_end_combat() -> Builtin {
    Builtin::builder("Solar Eclipse", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_round_end,
            10,
            |game, _player, name, r| {
                if let Some(p) = game
                    .permanent_effects
                    .iter()
                    .position(|e| matches!(e, PermanentEffect::SolarEclipse))
                {
                    if r.combat.first_round() && !r.combat.is_sea_battle(game) {
                        game.permanent_effects.remove(p);
                        r.combat.retreat = CombatRetreatState::EndAfterCurrentRound;

                        let p = match &r.final_result {
                            Some(CombatResult::AttackerWins) => r.combat.attacker(),
                            _ => r.combat.defender(),
                        };
                        let p = game.player_mut(p);
                        p.event_victory_points += 1_f32;
                        game.add_info_log_item(&format!(
                            "{name} gained 1 victory point for the Solar Eclipse",
                        ));
                    }
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
//     game.permanent_effects
//         .push(PermanentIncidentEffect::Guillotine);
// })
// .build()
// }

fn anarchy() -> Incident {
    Incident::builder(
        44,
        "Anarchy",
        "Set aside all government advances. \
        Whenever you research a new government advance, \
        take a game event token from there instead of the supply \
        (thereby not triggering game events). \
        Each advance left in the government advances area at \
        the end of the game is worth 1 victory point.",
        IncidentBaseEffect::None,
    )
    .add_simple_incident_listener(
        IncidentTarget::ActivePlayer,
        0,
        |game, player_index, player_name, _| {
            let old = game.player(player_index).advances.len();

            let remove = game
                .player(player_index)
                .advances
                .iter()
                .filter(|a| a.info(game).government.is_some())
                .collect_vec();
            for a in remove {
                remove_advance(game, a, player_index);
            }

            if game.player(player_index).government(game).is_some() {
                if let Some(special_advance) = find_government_special_advance(game, player_index) {
                    undo_unlock_special_advance(game, special_advance, player_index);
                }
            }

            let p = game.player_mut(player_index);
            let lost = old - p.advances.len();
            p.event_victory_points += lost as f32;
            if lost > 0 {
                game.add_info_log_item(&format!(
                    "{player_name} lost {lost} government advances due to Anarchy - \
                     adding {lost} victory points",
                ));

                game.permanent_effects
                    .push(PermanentEffect::Anarchy(Anarchy {
                        player: player_index,
                        advances_lost: lost,
                    }));
            }
        },
    )
    .build()
}

pub(crate) fn anarchy_advance() -> Builtin {
    Builtin::builder("Anarchy", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.advance,
            10,
            |game, player_index, player_name, i| {
                if i.advance.info(game).government.is_none() {
                    return;
                }

                if let Some(mut a) = remove_and_map_element_by(&mut game.permanent_effects, |e| {
                    if let PermanentEffect::Anarchy(a) = e {
                        Some(a.clone())
                    } else {
                        None
                    }
                }) {
                    if player_index == a.player {
                        game.add_info_log_item(&format!(
                            "{player_name} gained a government advance, taking a game event token \
                            instead of triggering a game event (and losing 1 victory point)",
                        ));
                        let p = game.player_mut(player_index);
                        p.incident_tokens += 1;
                        p.event_victory_points -= 1_f32;
                        a.advances_lost -= 1;
                        if a.advances_lost > 0 {
                            game.permanent_effects.push(PermanentEffect::Anarchy(a));
                        }
                    }
                }
            },
        )
        .build()
}
