use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::do_gain_action_card_from_pile;
use crate::advance::{Advance, init_great_library};
use crate::card::{HandCard, HandCardLocation, all_objective_hand_cards, log_card_transfer};
use crate::city::{City, MoodState, activate_city};
use crate::combat_listeners::CombatRoundEnd;
use crate::content::ability::{Ability, AbilityBuilder};
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{
    AdvanceRequest, HandCardsRequest, PaymentRequest, PositionRequest,
};
use crate::game::Game;
use crate::incident::draw_and_discard_incident_card_from_pile;
use crate::map::Terrain;
use crate::map::Terrain::Fertile;
use crate::objective_card::{discard_objective_card, gain_objective_card_from_pile};
use crate::player::{Player, gain_unit};
use crate::position::Position;
use crate::tactics_card::CombatRole;
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
    WonderInfo::builder(
        Wonder::GreatWall,
        "Land combat in your happy city: Attacker gets -2 combat value in the first round. \
        You automatically win battles if Barbarians attack any of your cities.",
        ResourcePile::new(3, 2, 7, 0, 0, 0, 5),
        Advance::Siegecraft,
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_round_start,
        8,
        move |game, p, s| {
            let c = &s.combat;
            if c.first_round()
                && c.role(p.index) == CombatRole::Defender
                && c.defender_city(game)
                    .is_some_and(|c| c.mood_state == MoodState::Happy)
            {
                s.defender_strength.extra_combat_value -= 2;
                s.defender_strength
                    .roll_log
                    .push("Great Wall gives -2 combat value in the first round".to_string());
            }
        },
    )
    .build()
}

fn great_statue() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatStatue,
        "Draw 1 objective card. \
        Once per turn, as a free action, discard an objective card from: Gain 1 action.",
        ResourcePile::new(3, 4, 5, 0, 0, 0, 5),
        Advance::Monuments,
    )
    .add_custom_action(
        CustomActionType::GreatStatue,
        |c| c.once_per_turn().free_action().no_resources(),
        use_great_statue,
        |_game, p| !p.objective_cards.is_empty(),
    )
    .add_once_initializer(move |game, player| {
        gain_objective_card_from_pile(game, player.index, &player.origin);
    })
    .build()
}

fn use_great_statue(b: AbilityBuilder) -> AbilityBuilder {
    b.add_hand_card_request(
        |event| &mut event.custom_action,
        0,
        |game, p, _| {
            let player = p.get(game);
            Some(HandCardsRequest::new(
                all_objective_hand_cards(player),
                1..=1,
                "Select an objective card to discard",
            ))
        },
        |game, s, _| {
            let HandCard::ObjectiveCard(card) = s.choice[0] else {
                panic!("not an objective card")
            };
            discard_objective_card(
                game,
                s.player_index,
                card,
                &s.origin,
                HandCardLocation::DiscardPile,
            );
            s.log(game, "Gain 1 action");
            game.actions_left += 1;
        },
    )
}

fn great_mausoleum() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatMausoleum,
        "Whenever you draw an action or game event card, you may instead draw the \
        top card of the action or game event discard pile. \
        You discard to the bottom of the pile.",
        ResourcePile::new(4, 4, 4, 0, 0, 0, 5),
        Advance::Priesthood,
    )
    .build()
}

pub(crate) fn use_great_mausoleum() -> Ability {
    Ability::builder("Great Mausoleum", "")
        .add_bool_request(
            |event| &mut event.choose_action_card,
            0,
            |game, p, ()| {
                if let Some(card) = game.action_cards_discarded.last() {
                    Some(format!(
                        "Do you want to draw {} from the discard pile?",
                        game.cache.get_action_card(*card).name()
                    ))
                } else {
                    do_gain_action_card_from_pile(game, p);
                    None
                }
            },
            |game, s, ()| {
                if s.choice {
                    let card = game
                        .action_cards_discarded
                        .pop()
                        .expect("action card not found in discard pile");
                    s.log(
                        game,
                        &format!(
                            "Draw {} from the discard pile",
                            game.cache.get_action_card(card).name()
                        ),
                    );
                    game.player_mut(s.player_index).action_cards.push(card);
                    log_card_transfer(
                        game,
                        &HandCard::ActionCard(card),
                        HandCardLocation::DiscardPile,
                        HandCardLocation::Hand(s.player_index),
                        &s.origin,
                    );
                } else {
                    do_gain_action_card_from_pile(game, s.player_index, &s.origin);
                }
            },
        )
        .add_bool_request(
            |event| &mut event.choose_incident,
            0,
            |game, p, i| {
                if let Some(card) = game.incidents_discarded.last() {
                    Some(format!(
                        "Do you want to draw {} from the discard pile?",
                        game.cache.get_incident(*card).name
                    ))
                } else {
                    i.incident_id = draw_and_discard_incident_card_from_pile(game, p.index);
                    None
                }
            },
            |game, s, i| {
                if s.choice {
                    let card = game
                        .incidents_discarded
                        .pop()
                        .expect("action card not found in discard pile");
                    s.log(
                        game,
                        &format!(
                            "Drew {} from the discard pile",
                            game.cache.get_incident(card).name
                        ),
                    );
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
        "Requires a port to build: \
        Activate the city: Place a ship on any sea space without enemy ships. \
        Decide the staring player of the next turn.",
        ResourcePile::new(3, 5, 4, 0, 0, 0, 5),
        Advance::Cartography,
    )
    .placement_requirement(Arc::new(|pos, game| {
        game.get_any_city(pos).pieces.port.is_some()
    }))
    .add_custom_action(
        CustomActionType::GreatLighthouse,
        |c| c.any_times().free_action().no_resources(),
        use_great_lighthouse,
        |game, p| {
            great_lighthouse_city(p).can_activate()
                && p.available_units().ships > 0
                && !great_lighthouse_spawns(game, p.index).is_empty()
        },
    )
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

fn use_great_lighthouse(b: AbilityBuilder) -> AbilityBuilder {
    b.add_position_request(
        |event| &mut event.custom_action,
        0,
        |game, p, _| {
            Some(PositionRequest::new(
                great_lighthouse_spawns(game, p.index),
                1..=1,
                "Select a sea space to place a ship",
            ))
        },
        |game, s, _| {
            activate_city(
                great_lighthouse_city(game.player(s.player_index)).position,
                game,
                &s.origin,
            );
            gain_unit(game, s.player_index, s.choice[0], UnitType::Ship, &s.origin);
        },
    )
}

fn library() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatLibrary,
        "Once per turn, as a free action, \
        you may choose a non-government, non-civilization advance: \
        Use the effect until the end of your turn.",
        ResourcePile::new(3, 6, 3, 0, 0, 0, 5),
        Advance::Philosophy,
    )
    .add_custom_action(
        CustomActionType::GreatLibrary,
        |c| c.once_per_turn().free_action().no_resources(),
        use_great_library,
        |_, _| true,
    )
    .build()
}

fn use_great_library(b: AbilityBuilder) -> AbilityBuilder {
    b.add_advance_request(
        |event| &mut event.custom_action,
        0,
        |game, p, _| {
            let player = game.player(p.index);
            Some(AdvanceRequest::new(
                game.cache
                    .get_advances()
                    .iter()
                    .filter_map(|a| {
                        (a.government.is_none() && !player.has_advance(a.advance))
                            .then_some(a.advance)
                    })
                    .collect_vec(),
            ))
        },
        |game, s, _| {
            let advance = s.choice;
            s.log(game, &format!("Use {} for the turn", advance.name(game)));
            game.player_mut(s.player_index).great_library_advance = Some(advance);
            init_great_library(game, s.player_index);
        },
    )
}

fn great_gardens() -> WonderInfo {
    WonderInfo::builder(
        Wonder::GreatGardens,
        "The city with this wonder may Collect any type of resource from \
            Grassland spaces including ideas and gold. \
            Enemies cannot enter the city if they have entered a Grassland space this turn.",
        ResourcePile::new(5, 5, 2, 0, 0, 0, 5),
        Advance::Irrigation,
    )
    .add_transient_event_listener(
        |events| &mut events.terrain_collect_options,
        1,
        |m, (), (), _| {
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
        "Counts as 5.1 victory points (instead of 4). \
            All victory points are awarded to the player who built the wonder \
            (owning does not grant any points).",
        ResourcePile::new(2, 3, 7, 0, 0, 0, 5),
        Advance::Rituals,
    )
    .built_victory_points(5.1) // because it breaks the tie
    .owned_victory_points(0)
    .build()
}

fn colosseum() -> WonderInfo {
    WonderInfo::builder(
        Wonder::Colosseum,
        "May pay culture tokens with mood tokens (or vice versa) - \
        except for the building wonders.\
        May increase the combat value in a land battle by 1 for 1 culture or mood token.",
        ResourcePile::new(3, 4, 5, 0, 0, 0, 5),
        Advance::Sports,
    )
    .add_payment_request_listener(
        |e| &mut e.combat_round_end,
        90,
        |game, p, e| {
            let player = &game.player(p.index);

            let cost = p.payment_options().tokens(player, 1);

            if !player.can_afford(&cost) {
                return None;
            }

            if !apply_colosseum(e, p.index, false) {
                p.log(
                    game,
                    &format!(
                        "Combat value is already at maximum, cannot increase combat value for {p}",
                    ),
                );
                return None;
            }

            Some(vec![PaymentRequest::optional(
                cost,
                "Add 1 to the combat value?",
            )])
        },
        |game, s, e| {
            let pile = &s.choice[0];
            if !pile.is_empty() {
                s.log(game, "Increase combat value by 1, scoring an extra hit");
                apply_colosseum(e, s.player_index, true);
            }
        },
    )
    .build()
}

fn apply_colosseum(e: &mut CombatRoundEnd, player: usize, do_update: bool) -> bool {
    e.update_hits(e.role(player), do_update, |h| {
        h.combat_value += 1;
    })
}
