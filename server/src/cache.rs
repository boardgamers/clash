use crate::action_card::{ActionCard, CivilCard};
use crate::advance::{Advance, AdvanceInfo};
use crate::city_pieces::Building;
use crate::civilization::Civilization;
use crate::content::advances::AdvanceGroup;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::custom_action_builtins;
use crate::content::{
    action_cards, advances, builtin, civilizations, incidents, objective_cards, objectives, wonders,
};
use crate::game::Game;
use crate::incident::Incident;
use crate::leader::{Leader, LeaderInfo};
use crate::objective_card::{Objective, ObjectiveCard};
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo};
use crate::status_phase::StatusPhaseState::{ChangeGovernmentType, DetermineFirstPlayer};
use crate::status_phase::{
    StatusPhaseState, complete_objectives, determine_first_player, draw_cards, free_advance,
    get_status_phase, may_change_government, raze_city,
};
use crate::tactics_card::TacticsCard;
use crate::wonder::{Wonder, WonderInfo};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Cache {
    all_builtins: Vec<Builtin>,
    builtins_by_name: HashMap<String, Builtin>,
    status_phase_handlers: HashMap<StatusPhaseState, Builtin>,

    all_advance_groups: Vec<AdvanceGroup>,
    advance_groups_by_name: HashMap<String, AdvanceGroup>,
    all_advances: Vec<AdvanceInfo>,
    all_governments: Vec<AdvanceGroup>,
    governments_by_name: HashMap<String, AdvanceGroup>,
    advances_by_building: HashMap<Building, Advance>,

    all_action_cards: Vec<ActionCard>,
    action_cards_by_id: HashMap<u8, ActionCard>,

    all_objective_cards: Vec<ObjectiveCard>,
    objective_cards_by_id: HashMap<u8, ObjectiveCard>,
    all_objectives: Vec<Objective>,
    objectives_by_name: HashMap<String, Objective>,

    all_wonders: Vec<WonderInfo>,

    all_incidents: Vec<Incident>,
    incidents_by_id: HashMap<u8, Incident>,

    civilizations_by_name: HashMap<String, Civilization>,
    all_special_advances: Vec<SpecialAdvanceInfo>,
    leaders: HashMap<Leader, LeaderInfo>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    #[must_use]
    pub fn new() -> Self {
        Cache {
            all_builtins: builtin::get_all_uncached(),
            builtins_by_name: builtin::get_all_uncached()
                .into_iter()
                .map(|builtin| (builtin.name.clone(), builtin))
                .chain(
                    custom_action_builtins()
                        .into_values()
                        .map(|builtin| (builtin.name.clone(), builtin)),
                )
                .collect(),
            status_phase_handlers: status_phase_handlers(),

            all_advances: advances::get_all_uncached(),

            all_advance_groups: advances::get_groups_uncached(),
            advance_groups_by_name: advances::get_groups_uncached()
                .into_iter()
                .map(|advance_group| (advance_group.name.clone(), advance_group))
                .collect(),

            all_governments: advances::get_governments_uncached(),
            governments_by_name: advances::get_governments_uncached()
                .into_iter()
                .map(|government| (government.name.clone(), government))
                .collect(),

            advances_by_building: advances::get_all_uncached()
                .into_iter()
                .filter_map(|advance| {
                    advance
                        .unlocked_building
                        .map(|building| (building, advance.advance))
                })
                .collect(),

            all_action_cards: action_cards::get_all_uncached(),
            action_cards_by_id: action_cards::get_all_uncached()
                .into_iter()
                .chain(
                    incidents::get_all_uncached()
                        .into_iter()
                        .filter_map(|incident| incident.action_card)
                        .collect_vec(),
                )
                .map(|action_card| (action_card.id, action_card))
                .collect(),

            all_objective_cards: objective_cards::get_all_uncached(),
            objective_cards_by_id: objective_cards::get_all_uncached()
                .into_iter()
                .map(|objective_card| (objective_card.id, objective_card))
                .collect(),

            all_objectives: objectives::get_all_uncached(),
            objectives_by_name: objectives::get_all_uncached()
                .into_iter()
                .map(|objective| (objective.name.clone(), objective))
                .collect(),

            all_wonders: wonders::get_all_uncached()
                .into_iter()
                .sorted_by_key(|w| w.wonder)
                .collect_vec(),

            all_incidents: incidents::get_all_uncached(),
            incidents_by_id: incidents::get_all_uncached()
                .into_iter()
                .map(|incident| (incident.id, incident))
                .collect(),

            civilizations_by_name: civilizations::get_all_uncached()
                .into_iter()
                .map(|c| (c.name.clone(), c))
                .collect(),
            all_special_advances: civilizations::get_all_uncached()
                .into_iter()
                .flat_map(|c| c.special_advances)
                .sorted_by_key(|s| s.advance)
                .collect(),
            leaders: civilizations::get_all_uncached()
                .into_iter()
                .flat_map(|c| c.leaders)
                .map(|l| (l.leader, l))
                .collect(),
        }
    }

    #[must_use]
    pub fn get_advances(&self) -> &Vec<AdvanceInfo> {
        &self.all_advances
    }

    #[must_use]
    pub fn get_advance(&self, a: Advance) -> &AdvanceInfo {
        &self.all_advances[a as usize]
    }

    #[must_use]
    pub fn get_special_advance(&self, a: SpecialAdvance) -> &SpecialAdvanceInfo {
        &self.all_special_advances[a as usize]
    }

    #[must_use]
    pub fn get_advance_groups(&self) -> &Vec<AdvanceGroup> {
        &self.all_advance_groups
    }

    ///
    /// # Panics
    ///
    /// Panics if advance group doesn't exist
    #[must_use]
    pub fn get_advance_group(&self, name: &str) -> &AdvanceGroup {
        self.advance_groups_by_name
            .get(name)
            .unwrap_or_else(|| panic!("Advance group {name} not found"))
    }

    #[must_use]
    pub fn get_governments(&self) -> &Vec<AdvanceGroup> {
        &self.all_governments
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if government doesn't exist
    #[must_use]
    pub fn get_government(&self, government: &str) -> &AdvanceGroup {
        self.governments_by_name
            .get(government)
            .unwrap_or_else(move || {
                panic!("Government {government} not found");
            })
    }

    #[must_use]
    pub fn get_building_advance(&self, building: Building) -> Advance {
        self.advances_by_building[&building]
    }

    #[must_use]
    pub fn get_builtins(&self) -> &Vec<Builtin> {
        &self.all_builtins
    }

    ///
    /// # Panics
    ///
    /// Panics if builtin does not exist
    #[must_use]
    pub fn get_builtin(&self, name: &str, game: &Game) -> &Builtin {
        self.builtins_by_name
            .get(name)
            .or_else(|| {
                if let Some(p) = get_status_phase(game) {
                    return Some(self.status_phase_handler(p));
                }
                None
            })
            .unwrap_or_else(|| panic!("builtin not found: {name}"))
    }

    #[must_use]
    pub fn status_phase_handler(&self, p: &StatusPhaseState) -> &Builtin {
        match p {
            DetermineFirstPlayer(_) => &self.status_phase_handlers[&DetermineFirstPlayer(0)],
            ChangeGovernmentType(_) => &self.status_phase_handlers[&ChangeGovernmentType(false)],
            _ => &self.status_phase_handlers[p],
        }
    }

    #[must_use]
    pub fn get_action_cards(&self) -> &Vec<ActionCard> {
        &self.all_action_cards
    }

    ///
    /// # Panics
    ///
    /// Panics if action card does not exist
    #[must_use]
    pub fn get_action_card(&self, id: u8) -> &ActionCard {
        self.action_cards_by_id
            .get(&id)
            .expect("incident action card not found")
    }

    ///
    /// # Panics
    /// Panics if action card does not exist
    #[must_use]
    pub fn get_civil_card(&self, id: u8) -> &CivilCard {
        &self.get_action_card(id).civil_card
    }

    ///
    /// # Panics
    /// Panics if action card does not exist
    #[must_use]
    pub fn get_tactics_card(&self, id: u8) -> &TacticsCard {
        self.get_action_card(id)
            .tactics_card
            .as_ref()
            .unwrap_or_else(|| panic!("tactics card not found for action card {id}"))
    }

    #[must_use]
    pub fn get_objective_cards(&self) -> &Vec<ObjectiveCard> {
        &self.all_objective_cards
    }

    ///
    /// # Panics
    ///
    /// Panics if objective card does not exist
    #[must_use]
    pub fn get_objective_card(&self, id: u8) -> &ObjectiveCard {
        self.objective_cards_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("objective card not found {id}"))
    }

    #[must_use]
    pub fn get_objectives(&self) -> &Vec<Objective> {
        &self.all_objectives
    }

    ///
    /// # Panics
    /// Panics if incident does not exist
    #[must_use]
    pub fn get_objective(&self, name: &str) -> &Objective {
        self.objectives_by_name
            .get(name)
            .expect("objective not found")
    }

    #[must_use]
    pub fn get_wonders(&self) -> &Vec<WonderInfo> {
        &self.all_wonders
    }

    #[must_use]
    pub fn get_wonder(&self, w: Wonder) -> &WonderInfo {
        &self.all_wonders[w as usize]
    }

    #[must_use]
    pub fn get_incidents(&self) -> &Vec<Incident> {
        &self.all_incidents
    }

    ///
    /// # Panics
    ///
    /// Panics if incident does not exist
    #[must_use]
    pub fn get_incident(&self, id: u8) -> &Incident {
        self.incidents_by_id.get(&id).expect("incident not found")
    }

    ///
    /// # Panics
    ///
    /// Panics if civilization does not exist
    #[must_use]
    pub fn get_civilization(&self, name: &str) -> Civilization {
        match name {
            // "Maya" => maya::maya(), // still needs to be implemented
            // for integration testing
            "test0" => Civilization::new("test0", vec![], vec![]),
            "test1" => Civilization::new("test1", vec![], vec![]),
            "test2" => Civilization::new("test2", vec![], vec![]),
            _ => self
                .civilizations_by_name
                .get(name)
                .cloned()
                .expect("civilization not found"),
        }
    }

    #[must_use]
    pub fn get_leader(&self, leader: &Leader) -> &LeaderInfo {
        self.leaders
            .get(leader)
            .unwrap_or_else(|| panic!("leader not found: {leader:?}"))
    }
}

fn status_phase_handlers() -> HashMap<StatusPhaseState, Builtin> {
    use StatusPhaseState::*;

    HashMap::from([
        (CompleteObjectives, complete_objectives()),
        (FreeAdvance, free_advance()),
        (DrawCards, draw_cards()),
        (RazeSize1City, raze_city()),
        (ChangeGovernmentType(false), may_change_government()),
        (DetermineFirstPlayer(0), determine_first_player()),
    ])
}
