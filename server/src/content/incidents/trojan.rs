use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::undo_unlock_special_advance;
use crate::advance::{find_government_special_advance, remove_advance};
use crate::combat::{Combat, CombatModifier, CombatRetreatState};
use crate::combat_listeners::CombatResult;
use crate::content::ability::Ability;
use crate::content::effects::{Anarchy, PermanentEffect};
use crate::content::persistent_events::{PaymentRequest, PositionRequest, UnitTypeRequest};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::leader::Leader;
use crate::player::{Player, can_add_army_unit, gain_unit};
use crate::player_events::{IncidentInfo, IncidentTarget};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::{UnitType, kill_units};
use crate::utils::remove_and_map_element_by;
use itertools::Itertools;

pub(crate) fn trojan_incidents() -> Vec<Incident> {
    vec![trojan_horse(), solar_eclipse(), anarchy(), guillotine()]
}

const TROJAN_DESCRIPTION: &str = "In a land battle against a defended city \
    (Army unit or Fortress), \
    the attacker may pay 1 wood and 1 culture token to get 1 victory point and to \
    deny the defender tactics cards in the first round of combat.";

fn trojan_horse() -> Incident {
    Incident::builder(
        42,
        "Trojan Horse",
        &format!("The following is available to all players: {TROJAN_DESCRIPTION}"),
        IncidentBaseEffect::BarbariansMove,
    )
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _, _| {
        game.permanent_effects.push(PermanentEffect::TrojanHorse);
    })
    .build()
}

pub(crate) fn decide_trojan_horse() -> Ability {
    Ability::builder("Trojan Horse", TROJAN_DESCRIPTION)
        .add_payment_request_listener(
            |event| &mut event.combat_start,
            10,
            |game, p, c| {
                if is_land_battle_against_defended_city(game, p.index, c) {
                    game.permanent_effects.iter().find_map(|e| {
                        matches!(e, PermanentEffect::TrojanHorse).then_some(vec![
                            PaymentRequest::optional(
                                p.payment_options().resources(
                                    p.get(game),
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
                    player.gain_event_victory_points(1_f32, &EventOrigin::Incident(42));
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
    combat.is_land_battle(game)
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
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, _, _| {
        game.permanent_effects.push(PermanentEffect::SolarEclipse);
    })
    .build()
}

pub(crate) fn solar_eclipse_end_combat() -> Ability {
    Ability::builder("Solar Eclipse", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_round_end,
            10,
            |game, _, r| {
                if let Some(p) = game
                    .permanent_effects
                    .iter()
                    .position(|e| matches!(e, PermanentEffect::SolarEclipse))
                {
                    if r.combat.first_round() && r.combat.is_land_battle(game) {
                        game.permanent_effects.remove(p);
                        r.combat.retreat = CombatRetreatState::EndAfterCurrentRound;

                        let p = match &r.final_result {
                            Some(CombatResult::AttackerWins) => r.combat.attacker(),
                            _ => r.combat.defender(),
                        };
                        game.player_mut(p)
                            .gain_event_victory_points(1_f32, &EventOrigin::Incident(41));
                        game.add_info_log_item(&format!(
                            "{} gained 1 victory point for the Solar Eclipse",
                            game.player_name(p)
                        ));
                    }
                }
            },
        )
        .build()
}

fn guillotine() -> Incident {
    Incident::builder(
        43,
        "Guillotine",
        "Kill your leader if you have one. \
        Then, choose one of the following: Choose a new leader in one of your cities or armies. \
        Alternatively, Gain 2 victory points. \
        You cannot play leaders for the remainder of the game.",
        IncidentBaseEffect::BarbariansSpawn,
    )
    .add_bool_request(
        |e| &mut e.incident,
        3,
        |game, p, i| {
            i.is_active(IncidentTarget::ActivePlayer, p.index)
                .then(|| should_choose_new_leader(game, p.index, &p.origin))
                .flatten()
        },
        |game, s, i| {
            if s.choice {
                game.add_info_log_item(&format!("{} chose to select a new leader", s.player_name));
                i.selected_player = Some(s.player_index);
            } else {
                game.add_info_log_item(&format!(
                    "{} gained 2 victory points instead of choosing a new leader",
                    s.player_name
                ));
                game.player_mut(s.player_index)
                    .gain_event_victory_points(2_f32, &s.origin);
            }
        },
    )
    .add_incident_position_request(
        IncidentTarget::ActivePlayer,
        2,
        |game, p, i| {
            new_leader_chosen(p.index, i).then(|| {
                PositionRequest::new(
                    new_leader_positions(p.get(game)),
                    1..=1,
                    "Select a city to choose a new leader in",
                )
            })
        },
        |game, s, i| {
            let pos = s.choice[0];
            game.add_info_log_item(&format!("{} chose a new leader in {pos}", s.player_name));
            i.selected_position = Some(pos);
        },
    )
    .add_unit_type_request(
        |e| &mut e.incident,
        1,
        |game, p, i| {
            let player_index = p.index;
            new_leader_chosen(player_index, i).then(|| {
                UnitTypeRequest::new(
                    game.player(player_index)
                        .available_leaders
                        .iter()
                        .map(Leader::unit_type)
                        .collect_vec(),
                    player_index,
                    "Select a new leader to replace the killed one",
                )
            })
        },
        |game, s, i| {
            let pos = i.selected_position.expect("position should be set");
            game.add_info_log_item(&format!(
                "{} gained {} in {pos}",
                s.player_name,
                s.choice.name(game)
            ));
            gain_unit(s.player_index, pos, s.choice, game);
        },
    )
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, p, _i| {
        let available_leaders = &p.get(game).available_leaders;
        if !available_leaders.is_empty() {
            game.add_info_log_item(&format!(
                "{p} has lost leaders due to the Guillotine: {}",
                available_leaders.iter().map(|l| l.name(game)).join(", ")
            ));
            p.get_mut(game).available_leaders = vec![];
        }
    })
    .build()
}

fn should_choose_new_leader(
    game: &mut Game,
    player_index: usize,
    origin: &EventOrigin,
) -> Option<String> {
    kill_leader(game, player_index);

    let p = game.player(player_index);
    if p.available_leaders.is_empty() || new_leader_positions(p).is_empty() {
        game.add_info_log_item(&format!(
            "{p} has no leaders left to choose from after the Guillotine - \
                                gained 2 victory points",
        ));
        game.player_mut(player_index)
            .gain_event_victory_points(2_f32, origin);
        None
    } else {
        Some("Do you want to choose a new leader instead of 2 victory points?".to_string())
    }
}

fn new_leader_chosen(player_index: usize, i: &mut IncidentInfo) -> bool {
    i.selected_player == Some(player_index)
}

fn kill_leader(game: &mut Game, player_index: usize) {
    let p = game.player(player_index);
    let leader = p.units.iter().find_map(|u| {
        if let UnitType::Leader(l) = u.unit_type {
            Some((u.id, l))
        } else {
            None
        }
    });
    if let Some((id, leader)) = leader {
        game.add_info_log_item(&format!(
            "{} was killed due to the Guillotine",
            leader.name(game)
        ));
        kill_units(game, &[id], player_index, None);
    }
}

fn new_leader_positions(player: &Player) -> Vec<Position> {
    player
        .cities
        .iter()
        .filter_map(|c| can_add_army_unit(player, c.position).then_some(c.position))
        .collect()
}

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
    .add_simple_incident_listener(IncidentTarget::ActivePlayer, 0, |game, p, _| {
        let old = p.get(game).advances.len();

        let remove = p
            .get(game)
            .advances
            .iter()
            .filter(|a| a.info(game).government.is_some())
            .collect_vec();
        let player_index = p.index;
        for a in remove {
            remove_advance(game, a, player_index);
        }

        if game.player(player_index).government(game).is_some() {
            if let Some(special_advance) = find_government_special_advance(game, player_index) {
                undo_unlock_special_advance(game, special_advance, player_index);
            }
        }

        let player = game.player_mut(player_index);
        let lost = old - player.advances.len();
        player.gain_event_victory_points(lost as f32, &p.origin);
        if lost > 0 {
            game.add_info_log_item(&format!(
                "{p} lost {lost} government advances due to Anarchy - \
                     adding {lost} victory points",
            ));

            game.permanent_effects
                .push(PermanentEffect::Anarchy(Anarchy {
                    player: player_index,
                    advances_lost: lost,
                }));
        }
    })
    .build()
}

pub(crate) fn anarchy_advance() -> Ability {
    Ability::builder("Anarchy", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.advance,
            10,
            |game, p, i| {
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
                    if p.index == a.player {
                        game.add_info_log_item(&format!(
                            "{p} gained a government advance, taking a game event token \
                            instead of triggering a game event (and losing 1 victory point)",
                        ));
                        let p = p.get_mut(game);
                        p.incident_tokens += 1;
                        p.gain_event_victory_points(-1_f32, &EventOrigin::Incident(44));
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
