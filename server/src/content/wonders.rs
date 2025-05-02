use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::do_gain_action_card_from_pile;
use crate::advance::Advance;
use crate::card::HandCard;
use crate::city::{City, MoodState};
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{
    AdvanceRequest, HandCardsRequest, PaymentRequest, PositionRequest,
};
use crate::game::Game;
use crate::incident::draw_and_discard_incident_card_from_pile;
use crate::log::format_mood_change;
use crate::map::Terrain;
use crate::map::Terrain::Fertile;
use crate::objective_card::{discard_objective_card, gain_objective_card_from_pile};
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player::{Player, add_unit};
use crate::position::Position;
use crate::unit::UnitType;
use crate::wonder::Wonder;
use crate::{resource_pile::ResourcePile, wonder::WonderInfo};
use itertools::Itertools;
use std::collections::HashSet;
use std::sync::Arc;

#[must_use]
pub fn get_all_uncached() -> Vec<WonderInfo> {
    vec![
        great_wall(),
        great_statue(),
        great_mausoleum(),
        colosseum(),
        library(),
        great_lighthouse(),
        pyramids(),
        great_gardens(),
    ]
}

fn great_wall() -> WonderInfo {
    // todo war ships broken - counts for both players
    // todo combat value
    // todo -2 combat value in the first round
    // todo automatically win battles if Barbarians attack any of your cities
    WonderInfo::builder(
        Wonder::GreatWall,
        "Great Well",
        "Land combat in your happy city: Attacker gets -2 combat value in the first round. \
        You automatically win battles if Barbarians attack any of your cities.",
        PaymentOptions::fixed_resources(ResourcePile::new(3, 2, 7, 0, 0, 0, 5)),
        Advance::Siegecraft,
    )
    .add_combat_round_start_listener(6, |g, c, s, role, info| {
        if info.owning_player != c.player(role)
            && c.round == 1
            && role.is_attacker()
            && c.defender_city(g)
                .is_some_and(|c| c.mood_state == MoodState::Happy)
        {
            s.extra_combat_value -= 2;
            s.roll_log
                .push("Great Wall gives -2 combat value in the first round".to_string());
        }
    })
    .build()
}

fn great_statue() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatStatue,
        "Great Statue",
        "Draw 1 objective card. \
        Once per turn, as a free action, discard an objective card from: Gain 1 action.",
        PaymentOptions::fixed_resources(ResourcePile::new(3, 4, 5, 0, 0, 0, 5)),
        Advance::Monuments,
    )
    .add_custom_action(CustomActionType::GreatStatue)
    .add_one_time_ability_initializer(|game, player_index| {
        gain_objective_card_from_pile(game, player_index);
    })
    .build()
}

pub(crate) fn use_great_statue() -> Builtin {
    Builtin::builder(
        "Great Statue",
        "Discard an objectives card from your hand: Gain 1 action.",
    )
    .add_hand_card_request(
        |event| &mut event.custom_action,
        0,
        |game, player_index, _,_| {
            let player = game.player(player_index);
            Some(HandCardsRequest::new(
                player
                    .objective_cards
                    .iter()
                    .map(|&a| HandCard::ObjectiveCard(a))
                    .collect_vec(),
                1..=1,
                "Select an objective card to discard",
            ))
        },
        |game, s, _,_| {
            let HandCard::ObjectiveCard(card) = s.choice[0] else {
                panic!("not an objective card")
            };
            game.add_info_log_item(&format!(
                "{} discarded {} to gain an action",
                s.player_name,
                game.cache.get_objective_card(card).name()
            ));
            discard_objective_card(game, s.player_index, card);
            game.actions_left += 1;
        },
    )
    .build()
}

fn great_mausoleum() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatMausoleum,
        "Great Lighthouse",
        "Whenever you draw an action or game event card, you may instead draw the \
        top card of the action or game event discard pile. \
        You discard to the bottom of the pile.",
        PaymentOptions::fixed_resources(ResourcePile::new(4, 4, 4, 0, 0, 0, 5)),
        Advance::Priesthood,
    )
    .build()
}

pub(crate) fn use_great_mausoleum() -> Builtin {
    Builtin::builder("Great Mausoleum", "")
        .add_bool_request(
            |event| &mut event.choose_action_card,
            0,
            |game, player_index, ()| {
                if let Some(card) = game.action_cards_discarded.last() {
                    Some(format!(
                        "Do you want to draw {} from the discard pile?",
                        game.cache.get_action_card(*card).name()
                    ))
                } else {
                    do_gain_action_card_from_pile(game, player_index);
                    None
                }
            },
            |game, s, ()| {
                if s.choice {
                    let card = game
                        .action_cards_discarded
                        .pop()
                        .expect("action card not found in discard pile");
                    game.add_info_log_item(&format!(
                        "{} drew {} from the discard pile",
                        s.player_name,
                        game.cache.get_action_card(card).name()
                    ));
                    game.player_mut(s.player_index).action_cards.push(card);
                } else {
                    do_gain_action_card_from_pile(game, s.player_index);
                }
            },
        )
        .add_bool_request(
            |event| &mut event.choose_incident,
            0,
            |game, player_index, i| {
                if let Some(card) = game.incidents_discarded.last() {
                    Some(format!(
                        "Do you want to draw {} from the discard pile?",
                        game.cache.get_incident(*card).name
                    ))
                } else {
                    i.incident_id = draw_and_discard_incident_card_from_pile(game, player_index);
                    None
                }
            },
            |game, s, i| {
                if s.choice {
                    let card = game
                        .incidents_discarded
                        .pop()
                        .expect("action card not found in discard pile");
                    game.add_info_log_item(&format!(
                        "{} drew {} from the discard pile",
                        s.player_name,
                        game.cache.get_incident(card).name
                    ));
                    i.incident_id = card;
                } else {
                    i.incident_id = draw_and_discard_incident_card_from_pile(game, s.player_index);
                }
            },
        )
        .build()
}

fn great_lighthouse() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatLighthouse,
        "Great Lighthouse",
        "Requires a port to build: \
        Activate the city: Place a ship on any sea space without enemy ships. \
        Decide the staring player of the next turn.",
        PaymentOptions::fixed_resources(ResourcePile::new(3, 5, 4, 0, 0, 0, 5)),
        Advance::Cartography,
    )
    .placement_requirement(Arc::new(|pos, game| {
        game.get_any_city(pos).pieces.port.is_some()
    }))
    .add_custom_action(CustomActionType::GreatLighthouse)
    .build()
}

pub(crate) fn great_lighthouse_city(p: &Player) -> &City {
    p.cities
        .iter()
        .find(|c| c.pieces.wonders.contains(&Wonder::GreatLighthouse))
        .expect("city not found")
}

pub(crate) fn great_lighthouse_spawns(game: &Game, player: usize) -> Vec<Position> {
    game.map
        .tiles
        .iter()
        .filter_map(|(&pos, t)| {
            (*t == Terrain::Water && game.enemy_player(player, pos).is_none()).then_some(pos)
        })
        .collect_vec()
}

pub(crate) fn use_great_lighthouse() -> Builtin {
    Builtin::builder(
        "Great Lighthouse",
        "Activate the city: Place a ship on any sea space without enemy ships.",
    )
    .add_position_request(
        |event| &mut event.custom_action,
        0,
        |game, player_index, _,_| {
            Some(PositionRequest::new(
                great_lighthouse_spawns(game, player_index),
                1..=1,
                "Select a sea space to place a ship",
            ))
        },
        |game, s, _,_| {
            let spawn = &s.choice[0];
            let city_pos = great_lighthouse_city(game.player(s.player_index)).position;
            add_unit(s.player_index, *spawn, UnitType::Ship, game);
            game.add_info_log_item(&format!(
                "{} activated the city at {city_pos} used the Great Lighthouse \
                to place a ship on {spawn} for free{}",
                s.player_name,
                format_mood_change(game.player(s.player_index), city_pos)
            ));
            game.player_mut(s.player_index)
                .get_city_mut(city_pos)
                .activate();
        },
    )
    .build()
}

fn library() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatLibrary,
        "Great Library",
        "Once per turn, as a free action, \
        you may choose a non-government, non-civilization advance: \
        Use the effect until the end of your turn.",
        PaymentOptions::fixed_resources(ResourcePile::new(3, 6, 3, 0, 0, 0, 5)),
        Advance::Philosophy,
    )
    .add_custom_action(CustomActionType::GreatLibrary)
    .build()
}
pub(crate) fn use_great_library() -> Builtin {
    Builtin::builder(
        "Great Library",
        "Use the effect of a non-government, non-civilization advance",
    )
    .add_advance_request(
        |event| &mut event.custom_action,
        0,
        |game, player_index, _,_| {
            let player = game.player(player_index);
            Some(AdvanceRequest::new(
                game.cache
                    .get_advances()
                    .iter()
                    .filter_map(
                        // todo special advances
                        |a| {
                            (a.government.is_none() && !player.has_advance(a.advance))
                                .then_some(a.advance)
                        },
                    )
                    .collect_vec(),
            ))
        },
        |game, s, _,_| {
            let advance = s.choice;
            game.add_info_log_item(&format!(
                "{} used the Great Library to use {} for the turn",
                s.player_name,
                advance.name(game)
            ));
            advance
                .info(game)
                .listeners
                .clone()
                .init(game, s.player_index);
            game.player_mut(s.player_index).great_library_advance = Some(advance);
        },
    )
    .build()
}

fn great_gardens() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatGardens,
        "Great Gardens",
        "The city with this wonder may Collect any type of resource from \
            Grassland spaces including ideas and gold. \
            Enemies cannot enter the city if they have entered a Grassland space this turn.",
        PaymentOptions::fixed_resources(ResourcePile::new(5, 5, 2, 0, 0, 0, 5)),
        Advance::Irrigation,
    )
    .add_transient_event_listener(
        |events| &mut events.terrain_collect_options,
        1,
        |m, (), ()| {
            m.insert(
                Fertile,
                HashSet::from([
                    ResourcePile::food(1),
                    ResourcePile::wood(1),
                    ResourcePile::ore(1),
                    ResourcePile::ideas(1),
                    ResourcePile::gold(1),
                ]),
            );
        },
    )
    .build()
}

fn pyramids() -> WonderInfo {
    WonderInfo::builder(
        Wonder::Pyramids,
        "Pyramids",
        "Counts as 5.1 victory points (instead of 4). \
            All victory points are awarded to the player who built the wonder \
            (owning does not grant any points).",
        PaymentOptions::fixed_resources(ResourcePile::new(2, 3, 7, 0, 0, 0, 5)),
        Advance::Rituals,
    )
    .built_victory_points(5.1) // because it breaks the tie
    .owned_victory_points(0)
    .build()
}

fn colosseum() -> WonderInfo {
    WonderInfo::builder(
        Wonder::Colosseum,
        "Colosseum",
        "May pay culture tokens with mood tokens (or vice versa) - \
        except for the building wonders.\
        May increase the combat value in a land battle by 1 for 1 culture or mood token.",
        PaymentOptions::fixed_resources(ResourcePile::new(3, 4, 5, 0, 0, 0, 5)),
        Advance::Sports,
    )
    .add_payment_request_listener(
        |e| &mut e.combat_round_end,
        90,
        |game, player_index, e| {
            // todo , &ListenerInfo
            let player = &game.player(player_index);

            let cost = PaymentOptions::tokens(player, PaymentReason::WonderAbility, 1);

            if !player.can_afford(&cost) {
                return None;
            }

            let h = e.hits_mut(e.role(player_index));
            let mut with_increase = h.clone();
            with_increase.combat_value += 1;
            if h.hits() == with_increase.hits() {
                game.add_info_log_item(&format!(
                    "Combat value is already at maximum, cannot increase combat value for {}",
                    game.player_name(player_index)
                ));
                return None;
            }

            Some(vec![PaymentRequest::optional(
                cost,
                "Add 1 to the combat value?",
            )])
        },
        |game, s, e| {
            let pile = &s.choice[0];
            let name = &s.player_name;
            if pile.is_empty() {
                game.add_info_log_item(&format!("{name} declined to pay for the combat value",));
            } else {
                game.add_info_log_item(&format!(
                    "{name} paid {pile} to increase the combat value by 1, scoring an extra hit",
                ));
                e.hits_mut(e.role(s.player_index)).combat_value += 1;
                e.set_final_result();
            }
        },
    )
    .build()
}
