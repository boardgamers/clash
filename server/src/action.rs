use crate::map::Rotation;
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::status_phase::StatusPhaseAction;
use crate::unit::MovementAction;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Action {
    Playing(PlayingAction),
    StatusPhase(StatusPhaseAction),
    Movement(MovementAction),
    CulturalInfluenceResolution(bool),
    Combat(CombatAction),
    PlaceSettler(Position),
    ExploreResolution(Rotation),
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

    #[must_use]
    pub fn place_settler(self) -> Option<Position> {
        if let Self::PlaceSettler(v) = self {
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
}

#[derive(Serialize, Deserialize, Clone)]
pub enum CombatAction {
    PlayActionCard(PlayActionCard),
    RemoveCasualties(Vec<u32>),
    Retreat(bool),
}

// Can't use Option<String> because of mongo stips null values
#[derive(Serialize, Deserialize, Clone)]
pub enum PlayActionCard {
    None,
    Card(String),
}
