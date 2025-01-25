use crate::combat::CombatModifier::{
    CancelFortressExtraDie, CancelFortressIgnoreHit, SteelWeaponsAttacker, SteelWeaponsDefender,
};
use crate::combat::{start_combat, Combat, CombatModifier};
use crate::content::advances::{METALLURGY, SIEGECRAFT, STEEL_WEAPONS};
use crate::game::{Game, GameState};
use crate::payment::PaymentModel;
use crate::player::Player;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseState {
    SteelWeaponsAttacker(Combat),
    SteelWeaponsDefender(Combat),
    SiegecraftPayment(Combat),
}

pub const SIEGECRAFT_EXTRA_DIE: PaymentModel =
    PaymentModel::sum(2, &[ResourceType::Wood, ResourceType::Gold]);
pub const SIEGECRAFT_IGNORE_HIT: PaymentModel =
    PaymentModel::sum(2, &[ResourceType::Ore, ResourceType::Gold]);

#[derive(Serialize, Deserialize, Clone)]
pub struct SiegecraftPayment {
    pub extra_die: ResourcePile,
    pub ignore_hit: ResourcePile,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum CustomPhaseAction {
    SteelWeaponsAttackerAction(ResourcePile),
    SteelWeaponsDefenderAction(ResourcePile),
    SiegecraftPaymentAction(SiegecraftPayment),
}

impl CustomPhaseAction {
    ///
    /// # Panics
    /// Panics if the action cannot be executed
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match &game.state {
            GameState::CustomPhase(state) => match state {
                CustomPhaseState::SteelWeaponsAttacker(c) => {
                    if let CustomPhaseAction::SteelWeaponsAttackerAction(payment) = self {
                        pay_steel_weapons(
                            game,
                            c.clone(),
                            player_index,
                            &payment,
                            SteelWeaponsAttacker,
                        );
                    } else {
                        panic!("Need to pass SteelWeaponsAttackerAction to execute");
                    }
                }
                CustomPhaseState::SteelWeaponsDefender(c) => {
                    if let CustomPhaseAction::SteelWeaponsDefenderAction(payment) = self {
                        pay_steel_weapons(
                            game,
                            c.clone(),
                            player_index,
                            &payment,
                            SteelWeaponsDefender,
                        );
                    } else {
                        panic!("Need to pass SteelWeaponsDefenderAction to execute");
                    }
                }
                CustomPhaseState::SiegecraftPayment(c) => {
                    if let CustomPhaseAction::SiegecraftPaymentAction(payment) = self {
                        pay_siegecraft(game, c.clone(), player_index, &payment);
                    } else {
                        panic!("Need to pass SiegecraftPaymentAction to execute");
                    }
                }
            },
            _ => panic!("can only execute custom phase actions if the game is in a custom phase"),
        }
    }

    ///
    /// # Panics
    /// Panics if the action cannot be undone
    pub fn undo(self, game: &mut Game, _player_index: usize) {
        match game.state {
            GameState::CustomPhase(ref state) => match state {
                CustomPhaseState::SiegecraftPayment(_)
                | CustomPhaseState::SteelWeaponsAttacker(_)
                | CustomPhaseState::SteelWeaponsDefender(_) => {
                    panic!("combat actions cannot be undone");
                }
            },
            _ => panic!("can only undo custom phase actions if the game is in a custom phase"),
        }
    }

    #[must_use]
    pub fn format_log_item(&self, _game: &Game, _player: &Player, player_name: &str) -> String {
        match self {
            CustomPhaseAction::SteelWeaponsAttackerAction(payment)
            | CustomPhaseAction::SteelWeaponsDefenderAction(payment) => {
                format!("{player_name} paid for steel weapons: {payment}",)
            }
            CustomPhaseAction::SiegecraftPaymentAction(payment) => {
                let mut effects = vec![];
                if !payment.extra_die.is_empty() {
                    effects.push(format!(
                        "{} to increase the combat value",
                        payment.extra_die
                    ));
                }
                if !payment.ignore_hit.is_empty() {
                    effects.push(format!("{} to ignore a hit", payment.ignore_hit));
                }
                if effects.is_empty() {
                    format!("{player_name} did not use siegecraft",)
                } else {
                    format!("{player_name} paid for siegecraft: {}", effects.join(", "))
                }
            }
        }
    }
}

pub fn start_steel_weapons_phase(
    game: &mut Game,
    c: Combat,
    player_index: usize,
    combat_modifier: CombatModifier,
) -> bool {
    let player = &game.players[player_index];
    let cost = steel_weapons_cost(game, &c, player_index);
    if player.has_advance(STEEL_WEAPONS) {
        if cost.is_free() {
            // auto-apply free steel weapons
            let mut new = c.clone();
            new.modifiers.push(combat_modifier);
            start_combat(game, new, Some(combat_modifier));
            return true;
        } else if cost.can_afford(&player.resources) {
            let phase = if c.attacker == player_index {
                CustomPhaseState::SteelWeaponsAttacker(c)
            } else {
                CustomPhaseState::SteelWeaponsDefender(c)
            };
            game.state = GameState::CustomPhase(phase);
            return true;
        }
    }
    false
}

pub fn start_siegecraft_phase(game: &mut Game, c: Combat) -> bool {
    let player = &game.players[c.attacker];
    let r = &player.resources;
    if game
        .get_any_city(c.defender_position)
        .is_some_and(|c| c.pieces.fortress.is_some())
        && player.has_advance(SIEGECRAFT)
        && (SIEGECRAFT_EXTRA_DIE.can_afford(r) || SIEGECRAFT_IGNORE_HIT.can_afford(r))
    {
        game.state = GameState::CustomPhase(CustomPhaseState::SiegecraftPayment(c));
        true
    } else {
        false
    }
}

#[must_use]
pub fn steel_weapons_cost(game: &Game, combat: &Combat, player_index: usize) -> PaymentModel {
    let player = &game.players[player_index];
    let attacker = &game.players[combat.attacker];
    let defender = &game.players[combat.defender];
    let both_steel_weapons =
        attacker.has_advance(STEEL_WEAPONS) && defender.has_advance(STEEL_WEAPONS);
    let cost = u32::from(!player.has_advance(METALLURGY) || both_steel_weapons);
    PaymentModel::sum(cost, &[ResourceType::Ore, ResourceType::Gold])
}

fn pay_steel_weapons(
    game: &mut Game,
    mut combat: Combat,
    player_index: usize,
    payment: &ResourcePile,
    combat_modifier: CombatModifier,
) {
    let cost = steel_weapons_cost(game, &combat, player_index);

    if !payment.is_empty() {
        assert!(cost.is_valid_payment(payment), "Invalid payment");
        game.players[player_index].loose_resources(payment.clone());
        combat.modifiers.push(combat_modifier);
    }

    start_combat(game, combat, Some(combat_modifier));
}

fn pay_siegecraft(
    game: &mut Game,
    mut combat: Combat,
    player_index: usize,
    payment: &SiegecraftPayment,
) {
    let player = &mut game.players[player_index];

    let options = [
        (
            &payment.extra_die,
            &SIEGECRAFT_EXTRA_DIE,
            CancelFortressExtraDie,
        ),
        (
            &payment.ignore_hit,
            &SIEGECRAFT_IGNORE_HIT,
            CancelFortressIgnoreHit,
        ),
    ];

    for (payment, cost, gain) in options {
        if !payment.is_empty() {
            assert!(cost.is_valid_payment(payment), "Invalid payment");
            player.loose_resources(payment.clone());
            combat.modifiers.push(gain);
        }
    }
    start_combat(game, combat, Some(CancelFortressExtraDie));
}
