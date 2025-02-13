use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Bonus::CultureToken;
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Fortress;
use crate::combat::CombatModifier::{
    CancelFortressExtraDie, CancelFortressIgnoreHit, SteelWeaponsAttacker, SteelWeaponsDefender,
};
use crate::combat::{Combat, CombatModifier, CombatStrength};
use crate::content::advances::{
    advance_group_builder, AdvanceGroup, METALLURGY, STEEL_WEAPONS, TACTICS,
};
use crate::content::custom_phase_actions::CustomPhasePaymentRequest;
use crate::game::{Game, GameState};
use crate::payment::{PaymentConversion, PaymentOptions};
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;

pub(crate) fn warfare() -> AdvanceGroup {
    advance_group_builder(
        "Warfare",
        vec![
            Advance::builder(
                TACTICS,
                "May Move Army units, May use Tactics on Action Cards",
            )
            .with_advance_bonus(CultureToken)
            .with_unlocked_building(Fortress)
            .add_player_event_listener(|event| &mut event.on_combat_round, fortress, 1),
            siegecraft(),
            steel_weapons().add_player_event_listener(
                |event| &mut event.on_combat_round,
                use_steel_weapons,
                0,
            ),
            draft(),
        ],
    )
}

fn draft() -> AdvanceBuilder {
    Advance::builder(
        "Draft",
        "When Recruiting, you may spend 1 mood token to pay for 1 Infantry Army Unit.",
    )
    .with_advance_bonus(CultureToken)
    .add_player_event_listener(
        |event| &mut event.recruit_cost,
        |cost, units, ()| {
            if units.infantry > 0 {
                // insert at beginning so that it's preferred over gold
                cost.info
                    .log
                    .push("Draft reduced the cost of 1 Infantry to 1 mood token".to_string());

                cost.cost.conversions.insert(
                    0,
                    PaymentConversion::limited(
                        UnitType::cost(&UnitType::Infantry),
                        ResourcePile::mood_tokens(1),
                        1,
                    ),
                );
            }
        },
        0,
    )
}

fn steel_weapons() -> AdvanceBuilder {
    Advance::builder(
        STEEL_WEAPONS,
        "Immediately before a Land battle starts, you may pay 1 ore to get +2 combat value in every Combat Round against an enemy that does not have the Steel Weapons advance, but only +1 combat value against an enemy that does have it (regardless if they use it or not this battle).",
    )
        .add_payment_request_listener(
            |e| &mut e.on_combat_start,
            1,
            |game, player_index, ()| {
                let GameState::Combat(c) = &game.state else { panic!("Invalid state") };
                let player = &game.players[player_index];

                let cost = steel_weapons_cost(game, c, player_index);
                if cost.is_free() {
                    let GameState::Combat(c) = &mut game.state else { panic!("Invalid state") };
                    add_steel_weapons(player_index, c);
                    return None;
                }

                if player.can_afford(&cost) {
                    Some(vec![CustomPhasePaymentRequest {
                        cost,
                        name: "Use steel weapons".to_string(),
                        optional: true,
                    }])
                } else {
                    None
                }
            },
            |game, player_index, player_name, payment| {
                let GameState::Combat(c) = &mut game.state else { panic!("Invalid state") };
                add_steel_weapons(player_index, c);
                game.add_info_log_item(
                    &format!("{player_name} paid for steel weapons: {}", payment[0]));
            },
        )
}

fn siegecraft() -> AdvanceBuilder {
    Advance::builder(
        "Siegecraft",
        "When attacking a city with a Fortress, pay 2 wood to cancel the Fortress’ ability to add +1 die and/or pay 2 ore to ignore its ability to cancel a hit.",
    )
        .add_payment_request_listener(
            |e| &mut e.on_combat_start,
            0,
            |game, player, ()| {
                let GameState::Combat(c) = &game.state else { panic!("Invalid state") };

                let extra_die = PaymentOptions::sum(2, &[ResourceType::Wood, ResourceType::Gold]);
                let ignore_hit = PaymentOptions::sum(2, &[ResourceType::Ore, ResourceType::Gold]);

                let player = &game.players[player];
                if game
                    .get_any_city(c.defender_position)
                    .is_some_and(|c| c.pieces.fortress.is_some())
                    && (player.can_afford(&extra_die) || player.can_afford(&ignore_hit))
                {
                    Some(vec![
                        CustomPhasePaymentRequest {
                            cost: extra_die,
                            name: "Cancel fortress ability to add an extra die in the first round of combat".to_string(),
                            optional: true,
                        },
                        CustomPhasePaymentRequest {
                            cost: ignore_hit,
                            name: "Cancel fortress ability to ignore the first hit in the first round of combat".to_string(),
                            optional: true,
                        },
                    ])
                } else {
                    None
                }
            },
            |game, _player_index, player_name, payment| {
                game.add_info_log_item(
                    &format!("{player_name} paid for siegecraft: "));
                let mut paid = false;
                let mut modifiers: Vec<CombatModifier> = Vec::new();
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
                let GameState::Combat(c) = &mut game.state else { panic!("Invalid state") };
                c.modifiers.extend(modifiers);
            },
        )
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
        attacker.has_advance(STEEL_WEAPONS) && defender.has_advance(STEEL_WEAPONS);
    let cost = u32::from(!player.has_advance(METALLURGY) || both_steel_weapons);
    PaymentOptions::sum(cost, &[ResourceType::Ore, ResourceType::Gold])
}

fn fortress(s: &mut CombatStrength, c: &Combat, game: &Game) {
    if s.attacker || !c.defender_fortress(game) || c.round != 1 {
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

fn use_steel_weapons(s: &mut CombatStrength, c: &Combat, game: &Game) {
    let steel_weapon_value = if game.get_player(c.attacker).has_advance(STEEL_WEAPONS)
        && game.get_player(c.defender).has_advance(STEEL_WEAPONS)
    {
        1
    } else {
        2
    };

    let add_combat_value = |s: &mut CombatStrength, value: u8| {
        s.extra_combat_value += value;
        s.roll_log
            .push(format!("steel weapons added {value} combat value"));
    };

    if s.attacker {
        if c.modifiers.contains(&SteelWeaponsAttacker) {
            add_combat_value(s, steel_weapon_value);
        }
    } else if c.modifiers.contains(&SteelWeaponsDefender) {
        add_combat_value(s, steel_weapon_value);
    }
}
