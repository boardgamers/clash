use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::CultureToken;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building::Fortress;
use crate::combat::CombatModifier::{
    CancelFortressExtraDie, CancelFortressIgnoreHit, SteelWeaponsAttacker, SteelWeaponsDefender,
};
use crate::combat::{Combat, CombatModifier};
use crate::combat_listeners::CombatStrength;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::persistent_events::PaymentRequest;
use crate::events::EventPlayer;
use crate::game::Game;
use crate::payment::{PaymentConversion, PaymentOptions};
use crate::player::Player;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::{CombatRole, play_tactics_card};
use crate::unit::UnitType;

pub(crate) fn warfare() -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Warfare,
        "Warfare",
        vec![tactics(), siegecraft(), steel_weapons(), draft()],
    )
}

fn tactics() -> AdvanceBuilder {
    play_tactics_card(
        AdvanceInfo::builder(
            Advance::Tactics,
            "Tactics",
            "May Move Army units, May use Tactics on Action Cards",
        )
        .with_advance_bonus(CultureToken)
        .with_unlocked_building(Fortress)
        .add_combat_strength_listener(3, fortress),
    )
}

fn siegecraft() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Siegecraft,
        "Siegecraft",
        "When attacking a city with a Fortress, pay 2 wood to cancel the Fortress’ \
        ability to add +1 die and/or pay 2 ore to ignore its ability to cancel a hit.",
    )
    .add_payment_request_listener(
        |e| &mut e.combat_start,
        0,
        |game, player, c| {
            let p = player.get(game);
            let extra_die =
                player
                    .payment_options()
                    .sum(p, 2, &[ResourceType::Wood, ResourceType::Gold]);
            let ignore_hit =
                player
                    .payment_options()
                    .sum(p, 2, &[ResourceType::Ore, ResourceType::Gold]);

            let player = player.get(game);
            if game
                .try_get_any_city(c.defender_position())
                .is_some_and(|c| c.pieces.fortress.is_some())
                && (player.can_afford(&extra_die) || player.can_afford(&ignore_hit))
            {
                Some(vec![
                    PaymentRequest::optional(
                        extra_die,
                        "Cancel fortress ability to add an extra die \
                             in the first round of combat",
                    ),
                    PaymentRequest::optional(
                        ignore_hit,
                        "Cancel fortress ability to ignore the first hit \
                             in the first round of combat",
                    ),
                ])
            } else {
                None
            }
        },
        |game, s, c| {
            let mut modifiers: Vec<CombatModifier> = Vec::new();
            let payment = &s.choice;
            if !payment[0].is_empty() {
                modifiers.push(CancelFortressExtraDie);
                s.log(game, "Cancel Fortress Extra Die");
            }
            if !payment[1].is_empty() {
                modifiers.push(CancelFortressIgnoreHit);
                s.log(game, "Cancel Fortress Ignore Hit");
            }

            c.modifiers.extend(modifiers);
        },
    )
}

fn steel_weapons() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::SteelWeapons,
        "Steel Weapons",
        "Immediately before a Land battle starts, \
        you may pay 1 ore to get +2 combat value in every Combat Round against an enemy \
        that does not have the Steel Weapons advance. \
        If the enemy also has Steel Weapons, you only +1 combat value, \
        even if the enemy does not use the ability.",
    )
    .add_payment_request_listener(
        |e| &mut e.combat_start,
        1,
        |game, p, c| {
            let player_index = p.index;
            let player = p.get(game);

            let cost = steel_weapons_cost(game, c, p);
            if cost.is_free() {
                add_steel_weapons(player_index, c);
                return None;
            }

            if player.can_afford(&cost) {
                Some(vec![PaymentRequest::optional(cost, "Use steel weapons")])
            } else {
                None
            }
        },
        |_game, s, c| {
            if s.choice[0].is_empty() {
                return;
            }
            add_steel_weapons(s.player_index, c);
        },
    )
    .add_combat_strength_listener(2, use_steel_weapons)
}

fn draft() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Draft,
        "Draft",
        "When Recruiting, you may spend 1 mood token to pay for 1 Infantry Army Unit.",
    )
    .with_advance_bonus(CultureToken)
    .add_transient_event_listener(
        |event| &mut event.recruit_cost,
        2,
        |cost, units, game, p| {
            if units.infantry > 0 {
                // insert at beginning so that it's preferred over gold

                let pile = ResourcePile::mood_tokens(draft_cost(p.get(game)));
                cost.info
                    .add_log(p, "Reduce the cost of 1 Infantry to 1 mood token");
                cost.cost.conversions.insert(
                    0,
                    PaymentConversion::limited(UnitType::cost(&UnitType::Infantry), pile, 1),
                );
            }
        },
    )
}

pub(crate) fn draft_cost(player: &Player) -> u8 {
    if player.can_use_advance(Advance::CivilLiberties) {
        2
    } else {
        1
    }
}

fn add_steel_weapons(player_index: usize, c: &mut Combat) {
    if player_index == c.attacker() {
        c.modifiers.push(SteelWeaponsAttacker);
    } else {
        c.modifiers.push(SteelWeaponsDefender);
    }
}

#[must_use]
fn steel_weapons_cost(game: &Game, combat: &Combat, p: &EventPlayer) -> PaymentOptions {
    let player = p.get(game);
    let attacker = &game.player(combat.attacker());
    let defender = &game.player(combat.defender());
    let both_steel_weapons = attacker.can_use_advance(Advance::SteelWeapons)
        && defender.can_use_advance(Advance::SteelWeapons);
    let cost = u8::from(!player.can_use_advance(Advance::Metallurgy) || both_steel_weapons);
    p.payment_options()
        .sum(player, cost, &[ResourceType::Ore, ResourceType::Gold])
}

fn fortress(game: &Game, c: &Combat, s: &mut CombatStrength, role: CombatRole) {
    if role.is_attacker() || !c.defender_fortress(game) || c.stats.round != 1 {
        return;
    }

    if !c.modifiers.contains(&CancelFortressExtraDie) {
        s.roll_log.push("fortress added one extra die".to_string());
        s.extra_dies += 1;
    }

    if !c.modifiers.contains(&CancelFortressIgnoreHit) {
        s.roll_log.push("fortress cancelled one hit".to_string());
        s.hit_cancels += 1;
    }
}

fn use_steel_weapons(game: &Game, c: &Combat, s: &mut CombatStrength, role: CombatRole) {
    let steel_weapon_value = if game
        .player(c.attacker())
        .can_use_advance(Advance::SteelWeapons)
        && game
            .player(c.defender())
            .can_use_advance(Advance::SteelWeapons)
    {
        1
    } else {
        2
    };

    let add_combat_value = |s: &mut CombatStrength, value: u8| {
        s.extra_combat_value += value as i8;
        s.roll_log
            .push(format!("steel weapons added {value} combat value"));
    };

    if role.is_attacker() {
        if c.modifiers.contains(&SteelWeaponsAttacker) {
            add_combat_value(s, steel_weapon_value);
        }
    } else if c.modifiers.contains(&SteelWeaponsDefender) {
        add_combat_value(s, steel_weapon_value);
    }
}
