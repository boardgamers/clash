use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::civilization::Civilization;
use crate::combat::update_combat_strength;
use crate::combat_listeners::CombatStrength;
use crate::content::advances::warfare::draft_cost;
use crate::content::custom_actions::CustomActionType;
use crate::payment::PaymentConversion;
use crate::player::gain_resources;
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};

pub(crate) fn greece() -> Civilization {
    Civilization::new(
        "Greece",
        vec![study(), sparta(), hellenistic_culture()],
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
        In land battles with fewer units than your enemy: Your enemy may nut play tactics cards.",
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
