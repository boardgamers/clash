use crate::content::incidents::great_diplomat::{DIPLOMAT_ID, DiplomaticRelations, Negotiations};
use crate::events::EventOrigin;
use crate::wonder::Wonder;
use serde::{Deserialize, Serialize};
use crate::game::Game;
use crate::player::Player;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Anarchy {
    pub player: usize,
    pub advances_lost: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum ConstructEffect {
    CityDevelopment,
    GreatEngineer,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CollectEffect {
    ProductionFocus,
    Overproduction,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct GreatSeerObjective {
    pub player: usize,
    pub objective_card: u8,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct GreatSeerEffect {
    pub player: usize,
    pub assigned_objectives: Vec<GreatSeerObjective>,
}

impl GreatSeerEffect {
    pub(crate) fn strip_secret(&mut self) {
        for o in &mut self.assigned_objectives {
            o.objective_card = 1_u8;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum PermanentEffect {
    Pestilence,
    LoseAction(usize),
    PublicWonderCard(Wonder),
    TrojanHorse,
    SolarEclipse,
    Anarchy(Anarchy),
    Construct(ConstructEffect),
    Collect(CollectEffect),
    CulturalTakeover,
    DiplomaticRelations(DiplomaticRelations),
    Negotiations(Negotiations),
    Assassination(usize),
    GreatSeer(GreatSeerEffect),
}

impl PermanentEffect {
    #[must_use]
    pub fn description(&self, game: &Game, player: &Player) -> Vec<String> {
        let cache = &game.cache;
        match self {
            PermanentEffect::Pestilence => cache.get_incident(1).description(game),
            PermanentEffect::Construct(c) => match c {
                ConstructEffect::CityDevelopment => vec![cache.get_civil_card(17).description],
                ConstructEffect::GreatEngineer => cache.get_incident(26).description(game),
            },
            PermanentEffect::Collect(c) => match c {
                CollectEffect::ProductionFocus => vec![cache.get_civil_card(19).description], 
                CollectEffect::Overproduction => cache.get_incident(29).description(game),   
            },
            PermanentEffect::LoseAction(_) => EventOrigin::Incident(38),
            PermanentEffect::PublicWonderCard(_) => EventOrigin::Incident(40),
            PermanentEffect::SolarEclipse => cache.get_incident(41).description(game),
            PermanentEffect::TrojanHorse => cache.get_incident(42).description(game),
            PermanentEffect::Anarchy(_) => EventOrigin::Incident(44),
            PermanentEffect::DiplomaticRelations(_) => EventOrigin::Incident(DIPLOMAT_ID),
            // can also be 16, but that doesn't matter for the help text
            PermanentEffect::CulturalTakeover => vec![cache.get_civil_card(15).description],
            PermanentEffect::Negotiations(_) => EventOrigin::CivilCard(23), // also 24
            PermanentEffect::Assassination(_) => EventOrigin::CivilCard(27), // also 28
            PermanentEffect::GreatSeer(s) => EventOrigin::CivilCard(158),
        }
    }
}
