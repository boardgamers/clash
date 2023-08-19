use serde::{Deserialize, Serialize};

use crate::playing_actions::PlayingAction;
use crate::status_phase::StatusPhaseAction;

#[derive(Serialize, Deserialize)]
pub enum Action {
    Playing(PlayingAction),
    StatusPhase(StatusPhaseAction),
    CulturalInfluenceResolution(bool),
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
    pub fn cultural_influence_resolution(self) -> Option<bool> {
        if let Self::CulturalInfluenceResolution(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
