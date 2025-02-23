use crate::content::custom_phase_actions::CustomPhaseEventAction;
use crate::map::Rotation;
use crate::playing_actions::PlayingAction;
use crate::status_phase::StatusPhaseAction;
use crate::unit::MovementAction;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Action {
    Playing(PlayingAction),
    StatusPhase(StatusPhaseAction),
    Movement(MovementAction),
    CulturalInfluenceResolution(bool),
    ExploreResolution(Rotation),
    CustomPhaseEvent(CustomPhaseEventAction),
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
    pub fn playing_ref(&self) -> Option<&PlayingAction> {
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
    pub fn explore_resolution(self) -> Option<Rotation> {
        if let Self::ExploreResolution(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn custom_phase_event(self) -> Option<CustomPhaseEventAction> {
        if let Self::CustomPhaseEvent(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
