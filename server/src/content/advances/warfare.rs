use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::CultureToken;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city_pieces::Building::Fortress;
use crate::combat::CombatModifier::{
    CancelFortressExtraDie, CancelFortressIgnoreHit, SteelWeaponsAttacker, SteelWeaponsDefender,
};
use crate::combat::{Combat, CombatModifier};
use crate::combat_listeners::CombatStrength;
use crate::content::advances::{AdvanceGroup, advance_group_builder};
use crate::content::persistent_events::PaymentRequest;
use crate::game::Game;
use crate::payment::{PaymentConversion, PaymentOptions};
use crate::player::Player;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::{CombatRole, play_tactics_card};
use crate::unit::UnitType;

pub(crate) fn warfare() -> AdvanceGroup {
    advance_group_builder(
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
        .add_combat_round_start_listener(3, fortress),
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
                let extra_die = PaymentOptions::sum(2, &[ResourceType::Wood, ResourceType::Gold]);
                let ignore_hit = PaymentOptions::sum(2, &[ResourceType::Ore, ResourceType::Gold]);

                let player = &game.players[player];
                if game
                    .try_get_any_city(c.defender_position)
                    .is_some_and(|c| c.pieces.fortress.is_some())
                    && (player.can_afford(&extra_die) || player.can_afford(&ignore_hit))
                {
                    Some(vec![
                        PaymentRequest {
                            cost: extra_die,
                            name: "Cancel fortress ability to add an extra die in the first round of combat".to_string(),
                            optional: true,
                        },
                        PaymentRequest {
                            cost: ignore_hit,
                            name: "Cancel fortress ability to ignore the first hit in the first round of combat".to_string(),
                            optional: true,
                        },
                    ])
                } else {
                    None
                }
            },
            |game, s, c| {
                game.add_info_log_item(
                    &format!("{} paid for siegecraft: ", s.player_name));
                let mut paid = false;
                let mut modifiers: Vec<CombatModifier> = Vec::new();
                let payment = &s.choice;
                if !payment[0].is_empty() {
                    modifiers.push(CancelFortressExtraDie);
                    game.add_to_last_log_item(&format!("{} to cancel the fortress ability to add an extra die", payment[0]));
                    paid = true;
                }
                if !payment[1].is_empty() {
                    modifiers.push(CancelFortressIgnoreHit);
                    if paid {
                        game.add_to_last_log_item(" and ");
                    }
                    game.add_to_last_log_item(&format!("{} to cancel the fortress ability to ignore a hit", payment[1]));
                    paid = true;
                }
                if !paid {
                    game.add_to_last_log_item("nothing");
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
        |game, player_index, c| {
            let player = &game.players[player_index];

            let cost = steel_weapons_cost(game, c, player_index);
            if cost.is_free() {
                add_steel_weapons(player_index, c);
                return None;
            }

            if player.can_afford(&cost) {
                Some(vec![PaymentRequest {
                    cost,
                    name: "Use steel weapons".to_string(),
                    optional: true,
                }])
            } else {
                None
            }
        },
        |game, s, c| {
            let pile = &s.choice[0];
            game.add_info_log_item(&format!(
                "{} paid for steel weapons: {}",
                s.player_name, pile
            ));
            if pile.is_empty() {
                return;
            }
            add_steel_weapons(s.player_index, c);
        },
    )
    .add_combat_round_start_listener(2, use_steel_weapons)
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
        0,
        |cost, units, player| {
            if units.infantry > 0 {
                // insert at beginning so that it's preferred over gold

                let pile = ResourcePile::mood_tokens(draft_cost(player) as u8);
                cost.info
                    .log
                    .push(format!("Draft reduced the cost of 1 Infantry to {pile}"));
                cost.cost.conversions.insert(
                    0,
                    PaymentConversion::limited(UnitType::cost(&UnitType::Infantry), pile, 1),
                );
            }
        },
    )
}

pub(crate) fn draft_cost(player: &Player) -> u8 {
    if player.has_advance(Advance::CivilLiberties) {
        2
    } else {
        1
    }
}

fn add_steel_weapons(player_index: usize, c: &mut Combat) {
    if player_index == c.attacker {
        c.modifiers.push(SteelWeaponsAttacker);
    } else {
        c.modifiers.push(SteelWeaponsDefender);
    }
}

#[must_use]
fn steel_weapons_cost(game: &Game, combat: &Combat, player_index: usize) -> PaymentOptions {
    let player = &game.players[player_index];
    let attacker = &game.players[combat.attacker];
    let defender = &game.players[combat.defender];
    let both_steel_weapons =
        attacker.has_advance(Advance::SteelWeapons) && defender.has_advance(Advance::SteelWeapons);
    let cost = u8::from(!player.has_advance(Advance::Metallurgy) || both_steel_weapons);
    PaymentOptions::sum(cost, &[ResourceType::Ore, ResourceType::Gold])
}

fn fortress(game: &Game, c: &Combat, s: &mut CombatStrength, role: CombatRole) {
    if role.is_attacker() || !c.defender_fortress(game) || c.round != 1 {
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
    let steel_weapon_value = if game.player(c.attacker).has_advance(Advance::SteelWeapons)
        && game.player(c.defender).has_advance(Advance::SteelWeapons)
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
