use crate::combat::{combat_loop, Combat, CombatModifier};
use crate::content::advances::SIEGECRAFT;
use crate::game::{Game, GameState};
use crate::payment::PaymentModel;
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};
use crate::resource::ResourceType;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseState {
    SiegecraftPayment(Combat),
}

pub const SIEGECRAFT_EXTRA_DIE: PaymentModel = PaymentModel::sum(2, &[ResourceType::Wood, ResourceType::Gold]);
pub const SIEGECRAFT_IGNORE_HIT: PaymentModel = PaymentModel::sum(2, &[ResourceType::Ore, ResourceType::Gold]);

#[derive(Serialize, Deserialize, Clone)]
pub struct SiegecraftPayment {
    pub extra_die: ResourcePile,
    pub ignore_hit: ResourcePile,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum CustomPhaseAction {
    SiegecraftPaymentAction(SiegecraftPayment),
}

impl CustomPhaseAction {
    ///
    /// # Panics
    /// Panics if the action cannot be executed
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match &game.state {
            GameState::CustomPhase(state) => match state {
                CustomPhaseState::SiegecraftPayment(c) =>
                {
                    #[allow(irrefutable_let_patterns)]
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
                CustomPhaseState::SiegecraftPayment(_) => {
                    panic!("combat actions cannot be undone");
                }
            },
            _ => panic!("can only undo custom phase actions if the game is in a custom phase"),
        }
    }

    #[must_use]
    pub fn format_log_item(&self, _game: &Game, _player: &Player, player_name: &str) -> String {
        match self {
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

pub fn start_siegecraft_phase(
    game: &mut Game,
    attacker: usize,
    defender_position: Position,
    c: Combat,
) -> bool {
    let player = &game.players[attacker];
    let r = &player.resources;
    if game
        .get_any_city(defender_position)
        .is_some_and(|c| c.pieces.fortress.is_some())
        && player.has_advance(SIEGECRAFT)
        && (r.can_afford(&ResourcePile::wood(2)) || r.can_afford(&ResourcePile::ore(2)))
    {
        game.state = GameState::CustomPhase(CustomPhaseState::SiegecraftPayment(c));
        true
    } else {
        false
    }
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
            CombatModifier::CancelFortressExtraDie,
        ),
        (
            &payment.ignore_hit,
            &SIEGECRAFT_IGNORE_HIT,
            CombatModifier::CancelFortressIgnoreHit,
        ),
    ];

    for (payment, cost, gain) in options {
        if !payment.is_empty() {
            assert!(cost.is_valid_payment(payment), "Invalid payment");
            player.loose_resources(payment.clone());
            combat.modifiers.push(gain);
        }
    }
    combat_loop(game, combat);
}
