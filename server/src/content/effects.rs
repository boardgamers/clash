use crate::content::incidents::great_diplomat::{DiplomaticRelations, Negotiations};
use crate::game::Game;
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
            o.objective_card = 0_u8;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum PermanentEffect {
    Pestilence,
    CivilWarLoseAction(usize),
    PublicWonderCard(Wonder),
    TrojanHorse,
    SolarEclipse,
    Anarchy(Anarchy),
    Construct(ConstructEffect),
    Collect(CollectEffect),
    CulturalTakeover,
    DiplomaticRelations(DiplomaticRelations),
    Negotiations(Negotiations),
    AssassinationLoseAction(usize),
    GreatSeer(GreatSeerEffect),
}

impl PermanentEffect {
    #[must_use]
    pub fn description(&self, game: &Game) -> Vec<String> {
        let cache = &game.cache;
        match self {
            PermanentEffect::Pestilence => cache.get_incident(1).description(game),
            PermanentEffect::Construct(c) => match c {
                ConstructEffect::CityDevelopment => {
                    vec![cache.get_civil_card(17).description.clone()]
                }
                ConstructEffect::GreatEngineer => cache.get_incident(26).description(game),
            },
            PermanentEffect::Collect(c) => match c {
                CollectEffect::ProductionFocus => {
                    vec![cache.get_civil_card(19).description.clone()]
                }
                CollectEffect::Overproduction => cache.get_incident(29).description(game),
            },
            PermanentEffect::CivilWarLoseAction(p) => {
                vec![format!(
                    "{} loses an action due to Civil War",
                    game.player_name(*p)
                )]
            }
            PermanentEffect::PublicWonderCard(w) => {
                vec![format!(
                    "Public wonder for anyone to draw: {}",
                    cache.get_wonder(*w).name()
                )]
            }
            PermanentEffect::SolarEclipse => cache.get_incident(41).description(game),
            PermanentEffect::TrojanHorse => cache.get_incident(42).description(game),
            PermanentEffect::Anarchy(a) => {
                vec![format!(
                    "{} has lost {} advances (each worth 1 victory point) due to Anarchy",
                    game.player_name(a.player),
                    a.advances_lost
                )]
            }
            PermanentEffect::DiplomaticRelations(r) => {
                vec![format!(
                    "{} (active player) and {} are in diplomatic relations \
                    and cannot attack each other (unless they pay 2 culture tokens).",
                    game.player_name(r.active_player),
                    game.player_name(r.passive_player)
                )]
            }
            // can also be 16, but that doesn't matter for the help text
            PermanentEffect::CulturalTakeover => vec![cache.get_civil_card(15).description.clone()],
            PermanentEffect::Negotiations(n) => {
                vec![format!(
                    "{} and {} are in negotiations. ({} turns left).",
                    game.player_name(n.relations.active_player),
                    game.player_name(n.relations.passive_player),
                    n.remaining_turns
                )]
            }
            PermanentEffect::AssassinationLoseAction(p) => {
                vec![format!(
                    "{} loses an action due to Assassination",
                    game.player_name(*p)
                )]
            }
            PermanentEffect::GreatSeer(s) => {
                let mut desc = vec![format!(
                    "{} has been assigned the following objectives: ",
                    game.player_name(s.player)
                )];
                for o in &s.assigned_objectives {
                    let card = o.objective_card;
                    let name = if card == 0 {
                        "Hidden objective".to_string()
                    } else {
                        cache.get_objective_card(card).name()
                    };

                    desc.push(format!("{}: {name}", game.player_name(o.player)));
                }
                desc
            }
        }
    }
}
