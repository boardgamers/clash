use serde::{Deserialize, Serialize};

use crate::game::CombatPhase;
use crate::playing_actions::PlayingAction;
use crate::status_phase::StatusPhaseAction;
use crate::unit::MovementAction;

#[derive(Serialize, Deserialize)]
pub enum Action {
    Playing(PlayingAction),
    StatusPhase(StatusPhaseAction),
    Movement(MovementAction),
    CulturalInfluenceResolution(bool),
    Combat(CombatAction),
    Undo,
    Redo,
}

impl Action {
    #[must_use]
    pub fn playing(self) -> Option<PlayingAction> {
        if let Self::Playing(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn status_phase(self) -> Option<StatusPhaseAction> {
        if let Self::StatusPhase(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn movement(self) -> Option<MovementAction> {
        if let Self::Movement(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn cultural_influence_resolution(self) -> Option<bool> {
        if let Self::CulturalInfluenceResolution(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn combat(self) -> Option<CombatAction> {
        if let Self::Combat(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum CombatAction {
    PlayActionCard(String),
    Retreat(bool)
}

impl CombatAction {
    pub fn phase(&self) -> CombatPhase {
        match self {
            Self::PlayActionCard(_) => CombatPhase::PlayActionCard,
            Self::Retreat(_) => CombatPhase::Retreat,
        }
    }
}
