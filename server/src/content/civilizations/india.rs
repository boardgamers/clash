// Monumental Edition board and leaders:
// https://boardgamegeek.com/image/8436927
// https://boardgamegeek.com/image/8436926
// Rulebook p. 34 clarifies Proselytism's range.
use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, gain_advance_without_payment};
use crate::city_pieces::Building;
use crate::civilization::Civilization;
use crate::combat::CombatModifier;
use crate::content::ability::{Ability, AbilityBuilder};
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::{AdvanceRequest, PaymentRequest};
use crate::cultural_influence::{InfluenceCultureInfo, resolve_influence_roll};
use crate::game::Game;
use crate::leader::{Leader, LeaderInfo};
use crate::leader_ability::{LeaderAbility, activate_leader_city, can_activate_leader_city};
use crate::payment::PaymentConversion;
use crate::player::{CostTrigger, Player};
use crate::resource_pile::ResourcePile;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo, SpecialAdvanceRequirement};
use crate::structure::Structure;
use crate::tactics_card::CombatRole;
use crate::unit::UnitType;

const PEACE_AND_POETRY: &str = "Peace & Poetry";
const BUDDHISM: &str = "Buddhism";

pub(crate) fn india() -> Civilization {
    Civilization::new("India", vec![
        SpecialAdvanceInfo::builder(
            SpecialAdvance::IndianElephants,
            SpecialAdvanceRequirement::Advance(Advance::Husbandry),
            "Indian Elephants",
            "Recruit Elephants in Forest cities without a Market. Elephants in non-Angry cities can establish Trade Routes.",
        ).build(),
        SpecialAdvanceInfo::builder(
            SpecialAdvance::Proselytism,
            SpecialAdvanceRequirement::Advance(Advance::StateReligion),
            "Proselytism",
            "Settlers can attempt Cultural Influence with range 2. Each additional settler in that space adds 1 range. Settlers in a city each add 1 to its influence range.",
        ).build(),
        SpecialAdvanceInfo::builder(
            SpecialAdvance::Prosperity,
            SpecialAdvanceRequirement::Advance(Advance::TradeRoutes),
            "Prosperity",
            "Trade Routes may provide mood tokens. Routes to cities containing your influenced buildings may provide culture tokens instead.",
        ).build(),
        SpecialAdvanceInfo::builder(
            SpecialAdvance::PeaceAndPoetry,
            SpecialAdvanceRequirement::Advance(Advance::Theaters),
            PEACE_AND_POETRY,
            "Once per turn, when researching an advance, you may pay 1 mood token instead of 2 food.",
        )
        .add_transient_event_listener(
            |e| &mut e.advance_cost, 3,
            |cost, _, game, p| {
                if !p.get(game).event_info.contains_key(PEACE_AND_POETRY) {
                    cost.cost.conversions.push(PaymentConversion::limited(
                        ResourcePile::food(2), ResourcePile::mood_tokens(1), 1,
                    ));
                }
            },
        ).build(),
    ], vec![sri_gupta(), ashoka(), akbar()], None)
}

// Record the option only when it is actually paid, never when costs are previewed
// or when a different ability grants an advance for tokens.
pub(crate) fn record_peace_and_poetry(game: &mut Game, player: usize, payment: &ResourcePile) {
    if payment.mood_tokens > 0
        && game
            .player(player)
            .has_special_advance(SpecialAdvance::PeaceAndPoetry)
    {
        game.player_mut(player)
            .event_info
            .insert(PEACE_AND_POETRY.into(), "used".into());
    }
}

fn golden_age_advances(game: &Game, player: &Player, reserve_culture: bool) -> Vec<Advance> {
    let mut available = player.resources.clone();
    if reserve_culture {
        if available.culture_tokens == 0 {
            return vec![];
        }
        available.culture_tokens -= 1;
    }
    game.cache
        .get_advance_groups()
        .iter()
        .filter(|group| !group.advances.iter().any(|a| player.has_advance(a.advance)))
        .flat_map(|group| group.advances.iter())
        .filter(|a| {
            player.can_advance_free(a.advance, game)
                && player
                    .advance_cost(a.advance, game, CostTrigger::NoModifiers)
                    .cost
                    .first_valid_payment(&available)
                    .is_some()
        })
        .map(|a| a.advance)
        .collect()
}

fn golden_age(b: AbilityBuilder) -> AbilityBuilder {
    b.add_advance_request(
        |e| &mut e.custom_action,
        2,
        |game, p, _| {
            Some(AdvanceRequest::new(golden_age_advances(
                game,
                p.get(game),
                false,
            )))
        },
        |game, s, action| {
            let cost =
                s.player()
                    .get(game)
                    .advance_cost(s.choice, game, game.execute_cost_trigger());
            action.advance_purchase = Some((s.choice, cost));
        },
    )
    .add_payment_request_listener(
        |e| &mut e.custom_action,
        1,
        |_game, _p, action| {
            let (_, cost) = action
                .advance_purchase
                .as_ref()
                .expect("selected Golden Age advance");
            Some(vec![PaymentRequest::mandatory(
                cost.cost.clone(),
                "Golden Age · Research cost",
            )])
        },
        |game, s, action| {
            let (advance, cost) = action
                .advance_purchase
                .take()
                .expect("selected Golden Age advance");
            // The payment listener has already paid the resources.
            cost.info.execute(game);
            record_peace_and_poetry(game, s.player_index, &s.choice[0]);
            activate_leader_city(game, &s.player());
            gain_advance_without_payment(game, advance, &s.player(), s.choice[0].clone(), true);
        },
    )
}

fn sri_gupta() -> LeaderInfo {
    LeaderInfo::new(Leader::SriGupta, "Maharaja Sri Gupta",
        LeaderAbility::builder("Golden Age",
            "Free action: Activate Sri Gupta's city and pay 1 culture token plus the research cost to gain an advance in a category you have not started.")
        .add_custom_action(
            CustomActionType::GoldenAge,
            |c| c.any_times().free_action().culture_tokens(1),
            golden_age,
            |game, p| can_activate_leader_city(game, p) && !golden_age_advances(game, p, true).is_empty(),
        ).build(),
        LeaderAbility::builder("Prepared",
            "Sri Gupta adds 2 combat value in each combat round in which you play no tactics card.")
        // Tactics is chosen at priority 0; Prepared must run after that choice.
        .add_combat_strength_listener(-1, |game, combat, strength, role| {
            if combat.has_leader(role, game) && strength.tactics_card.is_none() {
                strength.extra_combat_value += 2;
                strength.roll_log.push("Prepared adds +2 combat value".into());
            }
        }).build(),
    )
}

pub(crate) fn buddhism_available(game: &Game, player: usize, info: &InfluenceCultureInfo) -> bool {
    let p = game.player(player);
    !p.event_info.contains_key(BUDDHISM)
        && p.units.iter().any(|u| {
            u.unit_type == UnitType::Leader(Leader::Ashoka)
                && u.position == info.starting_city_position
        })
        && p.try_get_city(info.starting_city_position)
            .is_some_and(|city| {
                city.pieces.temple.is_some()
                    || info.structure == Structure::Building(Building::Temple)
            })
}

fn ashoka() -> LeaderInfo {
    LeaderInfo::new(Leader::Ashoka, "Ashoka the Great",
        LeaderAbility::builder(BUDDHISM,
            "Once per turn, reroll Cultural Influence from Ashoka's city if it has a Temple or the target is a Temple.")
        .add_bool_request(
            |e| &mut e.influence_culture, 1,
            |game, p, info| (info.roll < crate::consts::INFLUENCE_MIN_ROLL
                && buddhism_available(game, p.index, info))
                .then(|| format!("Buddhism · Rolled {}. Reroll?", info.roll)),
            |game, s, info| {
                if s.choice {
                    let previous = info.roll;
                    info.roll = game.next_dice_roll().value + info.roll_boost;
                    info.info.info.insert(BUDDHISM.into(), "used".into());
                    game.player_mut(s.player_index).event_info.insert(BUDDHISM.into(), "used".into());
                    s.log(game, &format!("Rerolled Cultural Influence: {previous} → {}", info.roll));
                }
                info.roll_boost_cost = resolve_influence_roll(game, info, s.player_index);
            },
        ).build(),
        LeaderAbility::builder("Expansionist",
            "When Ashoka captures a city, gain 1 culture token, or 2 if it has a Temple.")
        .add_simple_persistent_event_listener(
            |e| &mut e.combat_end, 24,
            |game, p, combat| {
                if combat.player(p.index).survived_leader()
                    && combat.captured_city(p.index).is_some()
                {
                    let amount = if game.get_any_city(combat.defender.position).pieces.temple.is_some() { 2 } else { 1 };
                    p.gain_resources(game, ResourcePile::culture_tokens(amount));
                }
            },
        ).build(),
    )
}

fn tusks_modifier(role: CombatRole) -> CombatModifier {
    if role == CombatRole::Attacker {
        CombatModifier::BladedTusksAttacker
    } else {
        CombatModifier::BladedTusksDefender
    }
}

fn akbar() -> LeaderInfo {
    LeaderInfo::new(Leader::Akbar, "Akbar the Great",
        LeaderAbility::builder("Bladed Tusks",
            "Before battle, pay 1 ore. Each surviving Elephant in Akbar's army adds 1 combat value each round, even if Akbar dies.")
        .add_payment_request_listener(
            |e| &mut e.combat_start, 3,
            |game, p, combat| {
                let role = combat.role(p.index);
                let cost = p.payment_options().resources(p.get(game), ResourcePile::ore(1));
                (combat.has_leader(role, game)
                    && combat.fighting_units(game, p.index).iter().any(|id| p.get(game).get_unit(*id).unit_type == UnitType::Elephant)
                    && p.get(game).can_afford(&cost))
                    .then(|| vec![PaymentRequest::optional(cost, "Bladed Tusks · +1 combat value per Elephant each round")])
            },
            |_game, s, combat| {
                if !s.choice[0].is_empty() { combat.modifiers.push(tusks_modifier(combat.role(s.player_index))); }
            },
        ).build(),
        LeaderAbility::builder("Prosperous",
            "Gain 1 gold when you move Akbar into one of your cities.")
        .add_transient_event_listener(
            |e| &mut e.before_move, 10,
            |game, movement, (), p| {
                if movement.from != movement.to && p.get(game).try_get_city(movement.to).is_some()
                    && movement.units.iter().any(|id| p.get(game).get_unit(*id).unit_type == UnitType::Leader(Leader::Akbar))
                { p.gain_resources(game, ResourcePile::gold(1)); }
            },
        ).build(),
    )
}

// This listener belongs to the game rather than the leader: the paid effect
// remains in force after Akbar is removed as a casualty.
pub(crate) fn use_bladed_tusks() -> Ability {
    Ability::builder("Bladed Tusks", "")
        .add_combat_strength_listener(109, |game, combat, strength, role| {
            if combat.modifiers.contains(&tusks_modifier(role)) {
                let player = if role.is_attacker() {
                    combat.attacker()
                } else {
                    combat.defender()
                };
                let elephants = combat
                    .fighting_units(game, player)
                    .iter()
                    .filter(|id| game.player(player).get_unit(**id).unit_type == UnitType::Elephant)
                    .count() as i8;
                strength.extra_combat_value += elephants;
                strength
                    .roll_log
                    .push(format!("Bladed Tusks adds +{elephants} combat value"));
            }
        })
        .build()
}
