use crate::ability_initializer::{AbilityInitializerBuilder, AbilityInitializerSetup};
use crate::action_card::ActionCard;
use crate::content::incidents::get_all;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::player::Player;

type StatusPhaseCheck = Box<dyn Fn(&Game, &Player) -> bool>;

pub struct Objective {
    pub name: String,
    pub description: String,
    status_phase_check: Option<StatusPhaseCheck>,
}

pub struct ObjectiveCard {
    pub id: u8,
    pub objectives: [Objective; 2],
}

pub struct ObjectiveBuilder {
    name: String,
    description: String,
    status_phase_check: Option<StatusPhaseCheck>,
    builder: AbilityInitializerBuilder,
}

impl ObjectiveBuilder {
    #[must_use]
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            status_phase_check: None,
            builder: AbilityInitializerBuilder::new(),
        }
    }
    
    #[must_use]
    pub fn status_phase_check<F>(mut self, f: F) -> Self
    where
        F: Fn(&Game, &Player) -> bool + 'static,
    {
        self.status_phase_check = Some(Box::new(f));
        self
    }
    
    #[must_use]
    pub fn build(self) -> Objective {
        Objective {
            name: self.name,
            description: self.description,
            status_phase_check: self.status_phase_check,
        }
    }
}

impl AbilityInitializerSetup for ObjectiveBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::ObjectiveCard(self.name.clone())
    }
}

///
/// # Panics
/// Panics if incident does not exist
#[must_use]
pub fn get_objective(id: u8) -> Incident {
    get_all()
        .into_iter()
        .find(|incident| incident.id == id)
        .expect("incident not found")
}