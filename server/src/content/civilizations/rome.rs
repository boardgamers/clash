use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{discard_action_card, gain_action_card_from_pile};
use crate::advance::{Advance, base_advance_cost, gain_advance_without_payment};
use crate::card::{HandCard, all_action_hand_cards, all_objective_hand_cards};
use crate::city::MoodState;
use crate::civilization::Civilization;
use crate::content::ability::AbilityBuilder;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{HandCardsRequest, PaymentRequest, PositionRequest};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::{
    LeaderAbility, LeaderAbilityBuilder, activate_leader_city, can_activate_leader_city,
};
use crate::map::{block_for_position, block_has_player_city};
use crate::objective_card::{discard_objective_card, gain_objective_card_from_pile};
use crate::payment::{
    PaymentConversion, PaymentConversionType, PaymentOptions, PaymentReason, base_resources,
};
use crate::player::{can_add_army_unit, gain_resources, gain_unit};
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
        "Ignore Famine events. \
                Sanitation cost is reduced to 0 resources or a free action",
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
        |i, &a, _| {
            if a == Advance::Sanitation {
                i.set_zero_resources();
                i.info
                    .log
                    .push("Aqueduct reduced the cost to 0".to_string());
            }
        },
    )
    .build()
}

fn use_aqueduct(b: AbilityBuilder) -> AbilityBuilder {
    b.add_simple_persistent_event_listener(
        |event| &mut event.custom_action,
        0,
        |game, player, name, a| {
            game.add_info_log_item(&format!(
                "{name} uses Aqueduct to gain Sanitation as a free action",
            ));
            gain_advance_without_payment(
                game,
                Advance::Sanitation,
                player,
                a.payment.clone(),
                true,
            );
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
        |game, player, _, s| {
            if s.is_winner(player) && s.is_battle() {
                gain_resources(
                    game,
                    player,
                    ResourcePile::gold(1) + ResourcePile::mood_tokens(1),
                    |name, pile| format!("{name} gained {pile} for Captivi"),
                );
            }
        },
    )
    .add_transient_event_listener(
        |event| &mut event.building_cost,
        2,
        |i, _b, _| {
            i.cost.conversions.push(PaymentConversion::new(
                base_resources(),
                ResourcePile::mood_tokens(1),
                PaymentConversionType::Unlimited,
            ));
            i.info
                .log
                .push("Captivi allows to replace resources with mood tokens".to_string());
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
            s.captured_city(player)
                .is_some()
                .then_some(vec![PaymentRequest::optional(
                    PaymentOptions::resources(
                        game.player(player),
                        PaymentReason::AdvanceAbility,
                        ResourcePile::culture_tokens(1),
                    ),
                    "Pay 1 culture token to make the city happy",
                )])
        },
        |game, c, s| {
            let pile = &c.choice[0];
            if pile.is_empty() {
                game.add_info_log_item("Provinces made the city Neutral instead of Angry");
            } else {
                game.add_info_log_item(&format!(
                    "Provinces made the city Happy instead of Angry for {pile}"
                ));
            }

            game.player_mut(s.attacker.player)
                .get_city_mut(s.defender.position)
                .set_mood_state(if pile.is_empty() {
                    MoodState::Neutral
                } else {
                    MoodState::Happy
                });
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
            "Imperator",
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
                s.roll_log.push("Imperator adds 2 combat value".to_string());
            }
        })
        .build(),
    )
}

fn use_princeps(b: AbilityBuilder) -> AbilityBuilder {
    b.add_hand_card_request(
        |event| &mut event.custom_action,
        0,
        |game, player, _| {
            activate_leader_city(
                game,
                player,
                "draw 1 action and 1 objective card using Princeps",
            );
            gain_action_card_from_pile(game, player);
            gain_objective_card_from_pile(game, player);

            let p = game.player(player);
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
                        game.add_info_log_item(&format!(
                            "{} discarded action card {} for Princeps",
                            s.player_name,
                            game.cache.get_action_card(*card).name()
                        ));
                        discard_action_card(game, p, *card);
                    }
                    HandCard::ObjectiveCard(card) => {
                        game.add_info_log_item(&format!(
                            "{} discarded objective card {} for Princeps",
                            s.player_name,
                            game.cache.get_objective_card(*card).name()
                        ));
                        discard_objective_card(game, p, *card);
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
            CustomActionType::StatesmanIncreaseHappiness,
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
            if !s.player(player).survived_leader() {
                return None;
            }

            let p = game.player(player);
            if p.available_units().infantry == 0 || !can_add_army_unit(p, leader_position(p)) {
                return None;
            }
            s.captured_city(player)
                .is_some()
                .then_some(vec![PaymentRequest::optional(
                    PaymentOptions::resources(
                        p,
                        PaymentReason::LeaderAbility,
                        ResourcePile::gold(1),
                    ),
                    "Pay 1 gold to gain 1 infantry",
                )])
        },
        |game, c, _| {
            if !c.choice.is_empty() {
                let p = c.player_index;
                let position = leader_position(game.player(p));
                gain_unit(p, position, UnitType::Infantry, game);
                game.add_info_log_item(&format!(
                    "{} used Proconsul to gain 1 infantry in {position} for {}",
                    game.player_name(p),
                    c.choice[0]
                ));
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
                |r, city, game| {
                    if let Ok(info) = r {
                        if info.is_defender
                            && leader_position(game.player(city.player_index)) == city.position
                        {
                            *r = Err("Sulla prevents influence culture attempts in this city"
                                .to_string());
                        }
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
            game.add_info_log_item(&format!(
                "{} selected Barbarian Armies that may NOT move: {}",
                s.player_name,
                may_not_move
                    .iter()
                    .map(ToString::to_string)
                    .collect_vec()
                    .join(", ")
            ));
            for pos in may_not_move {
                remove_element(movable, pos);
            }
        },
    )
}
