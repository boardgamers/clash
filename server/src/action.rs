use serde::{Deserialize, Serialize};

use crate::status_phase::StatusPhaseAction;
use crate::playing_actions::PlayingAction;

#[derive(Serialize, Deserialize)]
pub enum Action {
    Playing(PlayingAction),
    StatusPhase(StatusPhaseAction),
    CulturalInfluenceResolution(bool),
    Undo,
    Redo,
}

impl Action {
    pub fn playing(self) -> Option<PlayingAction> {
        if let Self::Playing(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn status_phase(self) -> Option<StatusPhaseAction> {
        if let Self::StatusPhase(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn cultural_influence_resolution(self) -> Option<bool> {
        if let Self::CulturalInfluenceResolution(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
