use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{discard_action_card, gain_action_card_from_pile};
use crate::advance::{Advance, base_advance_cost, gain_advance_without_payment};
use crate::card::{HandCard, HandCardLocation, all_action_hand_cards, all_objective_hand_cards};
use crate::city::{MoodState, set_city_mood};
use crate::civilization::Civilization;
use crate::content::ability::AbilityBuilder;
use crate::content::custom_actions::{CustomActionType, PlayingActionModifier};
use crate::content::persistent_events::{HandCardsRequest, PaymentRequest, PositionRequest};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::{
    LeaderAbility, LeaderAbilityBuilder, activate_leader_city, can_activate_leader_city,
};
use crate::map::{block_for_position, block_has_player_city};
use crate::objective_card::{discard_objective_card, gain_objective_card_from_pile};
use crate::payment::{PaymentConversion, PaymentConversionType, base_resources};
use crate::player::{can_add_army_unit, gain_unit};
use crate::playing_actions::PlayingActionType;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::unit::UnitType;
use crate::utils::remove_element;
use itertools::Itertools;

pub(crate) fn rome() -> Civilization {
    Civilization::new(
        "Rome",
        vec![aqueduct(), roman_roads(), captivi(), provinces()],
        vec![augustus(), caesar(), sulla()],
        None,
    )
}

fn aqueduct() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Aqueduct,
        SpecialAdvanceRequirement::Advance(Advance::Engineering),
        "Aqueduct",
        "Ignore Famine events. Sanitation cost is reduced to 0 resources or a free action",
    )
    .add_custom_action(
        CustomActionType::Aqueduct,
        |c| c.any_times().free_action().advance_cost_without_discounts(),
        use_aqueduct,
        |_game, p| !p.has_advance(Advance::Sanitation) && p.can_afford(&base_advance_cost(p)),
    )
    .add_transient_event_listener(
        |event| &mut event.advance_cost,
        3,
        |i, &a, _, p| {
            if a == Advance::Sanitation {
                i.set_zero_resources(p);
            }
        },
    )
    .build()
}

fn use_aqueduct(b: AbilityBuilder) -> AbilityBuilder {
    b.add_simple_persistent_event_listener(
        |event| &mut event.custom_action,
        0,
        |game, p, a| {
            gain_advance_without_payment(game, Advance::Sanitation, p, a.payment.clone(), true);
        },
    )
}

fn roman_roads() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::RomanRoads,
        SpecialAdvanceRequirement::Advance(Advance::Roads),
        "Roman Roads",
        "Roads distance is increased to 4 if travelling between your cities",
    )
    // is checked explicitly
    .build()
}

fn captivi() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Captivi,
        SpecialAdvanceRequirement::Advance(Advance::Bartering),
        "Captivi",
        "Gain 1 gold and 1 mood token when you win a battle. \
        You may replace any resources with mood tokens when paying for buildings.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_end,
        20,
        |game, p, s| {
            let player = p.index;
            if s.is_winner(player) && s.is_battle() {
                p.gain_resources(game, ResourcePile::gold(1) + ResourcePile::mood_tokens(1));
            }
        },
    )
    .add_transient_event_listener(
        |event| &mut event.building_cost,
        2,
        |i, _b, _, p| {
            i.cost.conversions.push(PaymentConversion::resource_options(
                base_resources(),
                ResourcePile::mood_tokens(1),
                PaymentConversionType::Unlimited,
            ));
            i.info.add_log(p, "May replace resources with mood tokens");
        },
    )
    .build()
}

fn provinces() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Provinces,
        SpecialAdvanceRequirement::AnyGovernment,
        "Provinces",
        "You can recruit Cavalry units in any city \
        that is at least 3 spaces away from your capital. \
        Captured cities become Neutral instead of Angry - or Happy if you pay 1 culture token.",
    )
    .add_payment_request_listener(
        |event| &mut event.combat_end,
        21,
        |game, player, s| {
            s.captured_city(player.index)
                .is_some()
                .then_some(vec![PaymentRequest::optional(
                    player
                        .payment_options()
                        .resources(player.get(game), ResourcePile::culture_tokens(1)),
                    "Pay 1 culture token to make the city happy",
                )])
        },
        |game, c, s| {
            set_city_mood(
                game,
                s.defender.position,
                &c.origin,
                if c.choice[0].is_empty() {
                    MoodState::Neutral
                } else {
                    MoodState::Happy
                },
            );
        },
    )
    .build()
}

fn augustus() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Augustus,
        "Augustus",
        LeaderAbility::builder(
            "Princeps",
            "As an action, pay 1 culture token and activate the leader city: \
            Draw 1 action and 1 objective card. \
            Then discard 1 action and 1 objective card.",
        )
        .add_custom_action(
            CustomActionType::Princeps,
            |c| {
                c.any_times()
                    .action()
                    .resources(ResourcePile::culture_tokens(1))
            },
            use_princeps,
            can_activate_leader_city,
        )
        .build(),
        LeaderAbility::builder(
            "Emperor",
            "Land battle with leader: If you don't own a city in the region: \
            Gain 2 combat value in every combat round",
        )
        .add_combat_strength_listener(103, |game, c, s, r| {
            if c.is_land_battle_with_leader(r, game)
                && !block_has_player_city(
                    game,
                    &block_for_position(game, c.defender_position()).1,
                    c.player(r),
                )
            {
                s.extra_combat_value += 2;
                s.roll_log.push("Emperor adds 2 combat value".to_string());
            }
        })
        .build(),
    )
}

fn use_princeps(b: AbilityBuilder) -> AbilityBuilder {
    b.add_hand_card_request(
        |event| &mut event.custom_action,
        0,
        |game, p, _| {
            activate_leader_city(game, p);
            gain_action_card_from_pile(game, p);
            gain_objective_card_from_pile(game, p);

            let p = p.get(game);
            Some(HandCardsRequest::new(
                all_action_hand_cards(p)
                    .into_iter()
                    .chain(all_objective_hand_cards(p))
                    .collect_vec(),
                2..=2,
                "Select 1 action and 1 objective card from your hand",
            ))
        },
        |game, s, _| {
            let p = s.player_index;
            for c in &s.choice {
                match c {
                    HandCard::ActionCard(card) => {
                        discard_action_card(
                            game,
                            p,
                            *card,
                            &s.origin,
                            HandCardLocation::DiscardPile,
                        );
                    }
                    HandCard::ObjectiveCard(card) => {
                        discard_objective_card(
                            game,
                            p,
                            *card,
                            &s.origin,
                            HandCardLocation::DiscardPile,
                        );
                    }
                    HandCard::Wonder(_) => panic!("Invalid hand card type"),
                }
            }
        },
    )
}

pub(crate) fn validate_princeps_cards(cards: &[HandCard]) -> Result<(), String> {
    match cards.len() {
        2 => {
            if cards
                .iter()
                .filter(|c| matches!(c, HandCard::ObjectiveCard(_)))
                .count()
                == 1
            {
                Ok(())
            } else {
                Err("must select 1 action and 1 objective card from your hand".to_string())
            }
        }
        _ => Err("must select 2 cards".to_string()),
    }
}

fn caesar() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Caesar,
        "Gaius Julius Caesar",
        LeaderAbility::builder(
            "Statesman",
            "As a free action, you may use 'Increase happiness' \
                to increase happiness in the leader city.",
        )
        .add_action_modifier(
            PlayingActionModifier::StatesmanIncreaseHappiness,
            |c| c.any_times().free_action().no_resources(),
            PlayingActionType::IncreaseHappiness,
        )
        .build(),
        proconsul(),
    )
}

fn proconsul() -> LeaderAbility {
    LeaderAbility::builder(
        "Proconsul",
        "When capturing a city with leader, you may spend 1 gold to gain 1 infantry",
    )
    .add_payment_request_listener(
        |event| &mut event.combat_end,
        22,
        |game, player, s| {
            if !s.player(player.index).survived_leader() {
                return None;
            }

            let p = game.player(player.index);
            if p.available_units().infantry == 0 || !can_add_army_unit(p, leader_position(p)) {
                return None;
            }
            s.captured_city(player.index)
                .is_some()
                .then_some(vec![PaymentRequest::optional(
                    player.payment_options().resources(p, ResourcePile::gold(1)),
                    "Pay 1 gold to gain 1 infantry",
                )])
        },
        |game, s, _| {
            if !s.choice.is_empty() {
                let p = s.player_index;
                gain_unit(
                    game,
                    &s.player(),
                    leader_position(game.player(p)),
                    UnitType::Infantry,
                );
            }
        },
    )
    .build()
}

fn sulla() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Sulla,
        "Sulla",
        add_barbarian_control(
            LeaderAbility::builder(
                "Dictator",
                "The leader city may not be the target of influence culture attempts.\
                Barbarians within 2 spaces of Sulla may only move if you agree to it.",
            )
            .add_transient_event_listener(
                |event| &mut event.on_influence_culture_attempt,
                6,
                |r, city, game, p| {
                    if let Ok(info) = r
                        && info.is_defender(p.index)
                        && leader_position(game.player(city.player_index)) == city.position
                    {
                        *r =
                            Err("Sulla prevents influence culture attempts in this city"
                                .to_string());
                    }
                },
            ),
        )
        .build(),
        LeaderAbility::builder(
            "Civilizer",
            "Land battle with leader against barbarians: Gain 2 combat value",
        )
        .add_combat_strength_listener(104, |game, c, s, r| {
            if c.has_leader(r, game) && c.is_barbarian_battle(r, game) {
                s.extra_combat_value += 2;
                s.roll_log.push("Sulla adds 2 combat value".to_string());
            }
        })
        .build(),
    )
}

pub(crate) fn owner_of_sulla_in_range(position: Position, game: &Game) -> Option<usize> {
    // Check if Sulla is within 2 spaces of the given position
    game.players.iter().find_map(|p| {
        p.active_leader()
            .is_some_and(|l| l == Leader::Sulla && leader_position(p).distance(position) <= 2)
            .then_some(p.index)
    })
}

fn add_barbarian_control(builder: LeaderAbilityBuilder) -> LeaderAbilityBuilder {
    builder.add_position_request(
        |event| &mut event.stop_barbarian_movement,
        0,
        |game, _player_index, movable| {
            let units = movable
                .clone()
                .into_iter()
                .filter(|&pos| owner_of_sulla_in_range(pos, game).is_some())
                .collect_vec();
            if units.is_empty() {
                return None;
            }

            Some(PositionRequest::new(
                units.clone(),
                0..=units.len() as u8,
                "Select Barbarian Armies that may NOT move",
            ))
        },
        |game, s, movable| {
            let may_not_move = &s.choice;
            s.log(
                game,
                &format!(
                    "Selected Barbarian Armies that may NOT move: {}",
                    may_not_move
                        .iter()
                        .map(ToString::to_string)
                        .collect_vec()
                        .join(", ")
                ),
            );
            for pos in may_not_move {
                remove_element(movable, pos);
            }
        },
    )
}
