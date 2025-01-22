use crate::combat::{Combat, CombatModifier};
use crate::content::advances::SIEGECRAFT;
use crate::content::custom_actions::CustomAction;
use crate::game::{Game, GameState};
use crate::log::{format_collect_log_item, format_happiness_increase};
use crate::player::Player;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseState {
    SiegecraftPayment(Combat),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum CustomPhaseAction {
    Siegecraft(ResourcePile),
}

impl CustomPhaseAction {
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match &game.state {
            GameState::CustomPhase(state) => match state {
                CustomPhaseState::SiegecraftPayment(c) => {
                    if let CustomPhaseAction::Siegecraft(payment) = self {
                        pay_siegecraft(game, c.clone(), player_index, payment);
                    } else {
                        panic!("siegecraft actions can only be executed if the game is in a siegecraft state");
                    }
                }
            },
            _ => panic!("can only execute custom phase actions if the game is in a custom phase"),
        }
    }

    pub fn undo(self, game: &mut Game, player_index: usize) {
        match game.state {
            GameState::CustomPhase(ref state) => match state {
                CustomPhaseState::SiegecraftPayment(s) => {
                    panic!("combat actions cannot be undone");
                }
            },
            _ => panic!("can only undo custom phase actions if the game is in a custom phase"),
        }
    }

    #[must_use]
    pub fn format_log_item(&self, _game: &Game, player: &Player, player_name: &str) -> String {
        match self {
            CustomPhaseAction::Siegecraft(payment) => {
                format!("{} paid {} for siegecraft", player_name, payment)
            }
        }
    }
}

pub fn start_siegecraft_phase(game: &mut Game, attacker: usize, defender_position: Position, c: Combat) -> bool {
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

fn pay_siegecraft(game: &mut Game, mut combat: Combat, player_index: usize, mut payment: ResourcePile) {
    let player = &mut game.players[player_index];
    player.loose_resources(payment.clone());
    if payment.try_take(ResourceType::Wood, 2) {
        combat.modifiers.push(CombatModifier::CancelFortressIncreaseCombatValue)
    } 
    if payment.try_take(ResourceType::Ore, 2) {
        combat.modifiers.push(CombatModifier::CancelFortressIgnoreHit)
    }
    if !payment.is_empty() {
        panic!("payment for siegecraft was not empty after paying for siegecraft");
    }

    game.state = GameState::Combat(combat);
}
