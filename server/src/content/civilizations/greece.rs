use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::can_play_civil_card;
use crate::advance::Advance;
use crate::card::HandCard;
use crate::city::{activate_city, increase_mood_state};
use crate::civilization::Civilization;
use crate::combat::update_combat_strength;
use crate::combat_listeners::CombatStrength;
use crate::content::ability::AbilityBuilder;
use crate::content::advances::AdvanceGroup;
use crate::content::advances::warfare::draft_cost;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{HandCardsRequest, PositionRequest};
use crate::events::check_event_origin;
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo, leader_position};
use crate::leader_ability::{LeaderAbility, activate_leader_city, can_activate_leader_city};
use crate::map::{block_has_player_city, get_map_setup};
use crate::payment::PaymentConversion;
use crate::player::Player;
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use itertools::Itertools;

pub(crate) fn greece() -> Civilization {
    Civilization::new(
        "Greece",
        vec![study(), sparta(), hellenistic_culture(), city_states()],
        vec![alexander(), leonidas(), pericles()],
        None,
    )
}

fn study() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Study,
        SpecialAdvanceRequirement::Advance(Advance::PublicEducation),
        "Study",
        "Gain 1 idea when recruiting in a city with an Academy.",
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.recruit,
        3,
        |game, p, r| {
            if game.get_any_city(r.city_position).pieces.academy.is_some() {
                p.gain_resources(game, ResourcePile::ideas(1));
            }
        },
    )
    .build()
}

fn sparta() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::Sparta,
        SpecialAdvanceRequirement::Advance(Advance::Draft),
        "Sparta",
        "You may pay Draft with culture tokens instead of mood tokens. \
        In land battles with fewer units than your enemy: Your enemy may not play tactics cards.",
    )
    .add_transient_event_listener(
        |event| &mut event.recruit_cost,
        0,
        |cost, units, player, _| {
            if units.infantry > 0 {
                cost.info
                    .log
                    .push("Sparta allows to pay the Draft cost as culture tokes".to_string());
                cost.cost.conversions.insert(
                    0,
                    PaymentConversion::limited(
                        ResourcePile::mood_tokens(1),
                        ResourcePile::culture_tokens(1),
                        draft_cost(player),
                    ),
                );
            }
        },
    )
    .add_simple_persistent_event_listener(
        |event| &mut event.combat_round_start_allow_tactics,
        0,
        |game, player, r| {
            let opponent = r.combat.opponent(player.index);
            if r.combat.fighting_units(game, player.index) < r.combat.fighting_units(game, opponent)
            {
                update_combat_strength(
                    game,
                    opponent,
                    r,
                    |_game, _combat, s: &mut CombatStrength, _role| {
                        s.roll_log
                            .push("Sparta denies playing tactics cards".to_string());
                        s.deny_tactics_card = true;
                    },
                );
            }
        },
    )
    .build()
}

fn hellenistic_culture() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::HellenisticCulture,
        SpecialAdvanceRequirement::Advance(Advance::Arts),
        "Hellenistic Culture",
        "Cultural influence: You may use any influenced city as a starting point. \
        You may replace the cost of Arts with 2 mood tokens.",
    )
    .add_action_modifier(
        CustomActionType::HellenisticInfluenceCultureAttempt,
        |c| {
            c.once_per_turn_mutually_exclusive(CustomActionType::ArtsInfluenceCultureAttempt)
                .free_action()
                .resources(ResourcePile::mood_tokens(2))
        },
        PlayingActionType::InfluenceCultureAttempt,
    )
    .build()
}

fn city_states() -> SpecialAdvanceInfo {
    SpecialAdvanceInfo::builder(
        SpecialAdvance::CityStates,
        SpecialAdvanceRequirement::AnyGovernment,
        "City States",
        "Once per turn, when the mood of a city was decreased due to activating a city, \
        you may instead decrease the mood of another city \
        of at least the same size and mood level.",
    )
    .add_position_request(
        |event| &mut event.city_activation_mood_decreased,
        0,
        |game, p, position| {
            let player_index = p.index;
            if game
                .player_mut(player_index)
                .event_info
                .contains_key("city_states")
            {
                return None;
            }

            let p = game.player(player_index);
            let city = p.get_city(*position);

            let cities = p
                .cities
                .iter()
                .filter_map(
                    // mood was already decreased
                    |c| {
                        (c.position != *position
                            && c.mood_state > city.mood_state
                            && c.size() >= city.size())
                        .then_some(c.position)
                    },
                )
                .collect_vec();

            if cities.is_empty() {
                return None;
            }

            Some(PositionRequest::new(
                cities,
                0..=1,
                "Select a city to decrease its mood instead of the activated city",
            ))
        },
        |game, s, position| {
            if !s.choice.is_empty() {
                increase_mood_state(game, *position, 1, &s.origin);
                activate_city(s.choice[0], game, &s.origin);
                game.player_mut(s.player_index)
                    .event_info
                    .insert("city_states".to_string(), "used".to_string());
            }
        },
    )
    .build()
}

fn alexander() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Alexander,
        "Alexander the Great",
        LeaderAbility::builder(
            "Idol",
            "As a free action, pay 1 culture token and activate the leader city: \
            play an action card as a free action",
        )
        .add_custom_action(
            CustomActionType::Idol,
            |c| {
                c.any_times()
                    .free_action()
                    .resources(ResourcePile::culture_tokens(1))
            },
            use_idol,
            |game, p| {
                can_activate_leader_city(game, p)
                    && !idol_cards(game, p, &ResourcePile::culture_tokens(1)).is_empty()
            },
        )
        .build(),
        LeaderAbility::builder(
            "Ruler of the World",
            "In a land battle with leader, gain 1 combat value for each region \
            with a city you control, except the starting region.",
        )
        .add_combat_strength_listener(100, |game, c, s, r| {
            if c.is_land_battle_with_leader(r, game) {
                let setup = get_map_setup(game.human_players_count());
                let player = c.player(r);
                let extra = setup
                    .free_positions
                    .into_iter()
                    .chain(
                        setup
                            .home_positions
                            .iter()
                            .enumerate()
                            .filter_map(|(i, h)| (i != player).then_some(h.position.clone())),
                    )
                    .filter(|b| block_has_player_city(game, b, player))
                    .count();
                s.extra_combat_value += extra as i8;
                s.roll_log
                    .push(format!("Ruler of the World adds {extra} combat value",));
            }
        })
        .build(),
    )
}

fn use_idol(b: AbilityBuilder) -> AbilityBuilder {
    b.add_hand_card_request(
        |event| &mut event.custom_action,
        0,
        |game, player, _| {
            activate_leader_city(game, player);

            Some(HandCardsRequest::new(
                idol_cards(game, player.get(game), &ResourcePile::empty()),
                1..=1,
                "Select an action card to play as a free action",
            ))
        },
        |game, s, _| {
            let HandCard::ActionCard(id) = s.choice[0] else {
                panic!("expected action card");
            };
            s.log(
                game,
                &format!(
                    "Decided to play {} as a free action",
                    game.cache.get_civil_card(id).name
                ),
            );

            PlayingAction::ActionCard(id)
                .execute_without_action_cost(game, s.player_index)
                .expect("playing action card with Idol");
        },
    )
}

fn idol_cards(game: &Game, p: &Player, extra_cost: &ResourcePile) -> Vec<HandCard> {
    p.action_cards
        .iter()
        .filter_map(|&a| {
            let action_cost = PlayingActionType::ActionCard(a).cost(game, p.index);
            if action_cost.free {
                // can play directly
                return None;
            }

            if can_play_civil_card(game, p, a).is_err() {
                // cannot play this card
                return None;
            }

            let mut payment_options = action_cost.payment_options(p, check_event_origin());
            payment_options.default += extra_cost.clone();
            p.can_afford(&payment_options)
                .then_some(HandCard::ActionCard(a))
        })
        .collect_vec()
}

fn leonidas() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Leonidas,
        "Leonidas I",
        LeaderAbility::builder(
            "That's Sparta",
            "Gain 1 culture token when recruiting in the leader city.",
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.recruit,
            4,
            |game, p, r| {
                if leader_position(p.get(game)) == r.city_position {
                    p.gain_resources(game, ResourcePile::culture_tokens(1));
                }
            },
        )
        .build(),
        LeaderAbility::builder(
            "Hero of Thermopylae",
            "In land battle with leader: \
            Get +2 combat value per army unit you have less than your enemy.",
        )
        .add_combat_strength_listener(101, |game, c, s, r| {
            let p = c.player(r);
            let pl = c.fighting_units(game, p).len();
            let op = c.fighting_units(game, c.opponent(p)).len();

            let extra = op.saturating_sub(pl) * 2;
            if c.has_leader(r, game) && extra > 0 {
                s.extra_combat_value += extra as i8;
                s.roll_log
                    .push(format!("Hero of Thermopylae adds {extra} combat value",));
            }
        })
        .build(),
    )
}

fn pericles() -> LeaderInfo {
    LeaderInfo::new(
        Leader::Pericles,
        "Pericles",
        LeaderAbility::advance_gain_custom_action(
            "Master",
            CustomActionType::Master,
            AdvanceGroup::Education,
        ),
        LeaderAbility::builder(
            "Admiral",
            "In Sea battles with leader: Gain +2 combat value",
        )
        .add_combat_strength_listener(102, |game, c, s, r| {
            if c.has_leader(r, game) && c.is_sea_battle(game) {
                s.extra_combat_value += 2;
                s.roll_log.push("Admiral adds +2 combat value".to_string());
            }
        })
        .build(),
    )
}
