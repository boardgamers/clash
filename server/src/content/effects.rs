use crate::content::incidents::great_diplomat::{DiplomaticRelations, Negotiations};
use crate::events::EventOrigin;
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
    MassProduction,
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
    RevolutionLoseAction(usize),
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
            PermanentEffect::Pestilence => incident_effect(game, 1),
            PermanentEffect::Construct(c) => match c {
                ConstructEffect::CityDevelopment => civil_effect(game, 17),
                ConstructEffect::GreatEngineer => incident_effect(game, 26),
            },
            PermanentEffect::Collect(c) => match c {
                CollectEffect::ProductionFocus => civil_effect(game, 19),
                CollectEffect::MassProduction => incident_effect(game, 29),
            },
            PermanentEffect::RevolutionLoseAction(p) => {
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
            PermanentEffect::SolarEclipse => incident_effect(game, 41),
            PermanentEffect::TrojanHorse => incident_effect(game, 42),
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
            PermanentEffect::CulturalTakeover => civil_effect(game, 15),
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

fn incident_effect(game: &Game, id: u8) -> Vec<String> {
    event_help(game, &EventOrigin::Incident(id))
}

fn civil_effect(game: &Game, id: u8) -> Vec<String> {
    event_help(game, &EventOrigin::CivilCard(id))
}

#[must_use]
pub fn event_help(game: &Game, origin: &EventOrigin) -> Vec<String> {
    let mut h = vec![origin.name(game)];
    let cache = &game.cache;
    let d = match origin {
        EventOrigin::Advance(a) => vec![a.info(game).description.clone()],
        EventOrigin::Wonder(w) => vec![game.cache.get_wonder(*w).description.clone()],
        EventOrigin::Ability(b) => vec![cache.ability_description(b, game)],
        EventOrigin::CivilCard(id) => vec![cache.get_civil_card(*id).description.clone()],
        EventOrigin::TacticsCard(id) => vec![cache.get_tactics_card(*id).description.clone()],
        EventOrigin::Incident(id) => cache.get_incident(*id).description(game),
        EventOrigin::Objective(name) => vec![cache.get_objective(name).description.clone()],
        EventOrigin::LeaderAbility(l) => vec![
            game.player(game.active_player())
                .get_leader_ability(l)
                .description
                .clone(),
        ],
        EventOrigin::SpecialAdvance(s) => vec![s.info(game).description.clone()],
    };
    h.extend(d);
    h
}
