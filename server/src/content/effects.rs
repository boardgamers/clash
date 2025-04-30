use crate::content::incidents::great_diplomat::{DIPLOMAT_ID, DiplomaticRelations, Negotiations};
use crate::events::EventOrigin;
use crate::wonder::Wonder;
use serde::{Deserialize, Serialize};

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
}

impl PermanentEffect {
    #[must_use]
    pub fn event_origin(&self) -> EventOrigin {
        match self {
            PermanentEffect::Pestilence => EventOrigin::Incident(1),
            PermanentEffect::Construct(c) => match c {
                ConstructEffect::CityDevelopment => EventOrigin::CivilCard(17), // also 18
                ConstructEffect::GreatEngineer => EventOrigin::Incident(26),
            },
            PermanentEffect::Collect(c) => match c {
                CollectEffect::ProductionFocus => EventOrigin::CivilCard(19), // also 20
                CollectEffect::Overproduction => EventOrigin::Incident(29),   // also 30
            },
            PermanentEffect::LoseAction(_) => EventOrigin::Incident(38),
            PermanentEffect::PublicWonderCard(_) => EventOrigin::Incident(40),
            PermanentEffect::SolarEclipse => EventOrigin::Incident(41),
            PermanentEffect::TrojanHorse => EventOrigin::Incident(42),
            PermanentEffect::Anarchy(_) => EventOrigin::Incident(44),
            PermanentEffect::DiplomaticRelations(_) => EventOrigin::Incident(DIPLOMAT_ID),
            // can also be 16, but that doesn't matter for the help text
            PermanentEffect::CulturalTakeover => EventOrigin::CivilCard(15),
            PermanentEffect::Negotiations(_) => EventOrigin::CivilCard(23), // also 24
            PermanentEffect::Assassination(_) => EventOrigin::CivilCard(27), // also 28
        }
    }
}
