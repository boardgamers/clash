use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::combat::update_combat_strength;
use crate::combat_listeners::CombatStrength;
use crate::content::advances::warfare::draft_cost;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::PositionRequest;
use crate::payment::PaymentConversion;
use crate::player::gain_resources;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use itertools::Itertools;

pub(crate) fn greece() -> Civilization {
    Civilization::new(
        "Greece",
        vec![study(), sparta(), hellenistic_culture(), city_states()],
        vec![],
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
        |game, player_index, _player_name, r| {
            if game.get_any_city(r.city_position).pieces.academy.is_some() {
                gain_resources(game, player_index, ResourcePile::ideas(1), |name, pile| {
                    format!("{name} gained {pile} for Study")
                });
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
        |cost, units, player| {
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
        |game, player, _name, r| {
            let opponent = r.combat.opponent(player);
            if r.combat.fighting_units(game, player) < r.combat.fighting_units(game, opponent) {
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
    .add_custom_action(CustomActionType::HellenisticInfluenceCultureAttempt)
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
    .add_custom_action(CustomActionType::HellenisticInfluenceCultureAttempt)
    .add_position_request(
        |event| &mut event.city_activation_mood_decreased,
        0,
        |game, player_index, position| {
            if game
                .player_mut(player_index)
                .event_info
                .insert("city_states".to_string(), "used".to_string())
                .is_some()
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
            if s.choice.is_empty() {
                game.add_info_log_item(&format!(
                    "{} decided not to decrease the mood of another city using City States",
                    s.player_name
                ));
            } else {
                let choice = s.choice[0];
                game.add_info_log_item(&format!(
                    "{} decided to decrease the mood of {} instead of {} using City States",
                    s.player_name, choice, position
                ));
                let p = game.player_mut(s.player_index);
                p.get_city_mut(choice).activate();
                p.get_city_mut(*position).increase_mood_state();
            }
        },
    )
    .build()
}
