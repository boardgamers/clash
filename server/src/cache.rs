use crate::action_card::ActionCard;
use crate::advance::{Advance, AdvanceInfo};
use crate::content::advances::AdvanceGroup;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::custom_action_builtins;
use crate::content::{
    action_cards, advances, builtin, incidents, objective_cards, objectives, wonders,
};
use crate::game::Game;
use crate::incident::Incident;
use crate::objective_card::{Objective, ObjectiveCard};
use crate::status_phase::StatusPhaseState::DetermineFirstPlayer;
use crate::status_phase::{
    StatusPhaseState, complete_objectives, determine_first_player, draw_cards, free_advance,
    get_status_phase, may_change_government, raze_city,
};
use crate::wonder::Wonder;
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::LazyLock;

static CACHE: LazyLock<Cache> = LazyLock::new(Cache::new);

#[must_use]
pub fn get() -> &'static Cache {
    &CACHE
}

pub struct Cache {
    all_builtins: Vec<Builtin>,
    builtins_by_name: HashMap<String, Builtin>,
    status_phase_handlers: HashMap<StatusPhaseState, Builtin>,

    all_advance_groups: Vec<AdvanceGroup>,
    advance_groups_by_name: HashMap<String, AdvanceGroup>,
    all_advances: Vec<AdvanceInfo>,
    all_governments: Vec<AdvanceGroup>,
    governments_by_name: HashMap<String, AdvanceGroup>,

    all_action_cards: Vec<ActionCard>,
    action_cards_by_id: HashMap<u8, ActionCard>,

    all_objective_cards: Vec<ObjectiveCard>,
    objective_cards_by_id: HashMap<u8, ObjectiveCard>,
    all_objectives: Vec<Objective>,
    objectives_by_name: HashMap<String, Objective>,

    all_wonders: Vec<Wonder>,
    wonders_by_name: HashMap<String, Wonder>,

    all_incidents: Vec<Incident>,
    incidents_by_id: HashMap<u8, Incident>,
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

            all_wonders: wonders::get_all_uncached(),
            wonders_by_name: wonders::get_all_uncached()
                .into_iter()
                .map(|wonder| (wonder.name.clone(), wonder))
                .collect(),

            all_incidents: incidents::get_all_uncached(),
            incidents_by_id: incidents::get_all_uncached()
                .into_iter()
                .map(|incident| (incident.id, incident))
                .collect(),
        }
    }

    #[must_use]
    pub fn get_advances(&'static self) -> &'static Vec<AdvanceInfo> {
        &self.all_advances
    }

    #[must_use]
    pub fn get_advance(&'static self, a: Advance) -> &AdvanceInfo {
        &self.all_advances[a as usize]
    }

    #[must_use]
    pub fn get_advance_groups(&'static self) -> &'static Vec<AdvanceGroup> {
        &self.all_advance_groups
    }

    #[must_use]
    pub fn get_advance_group(&'static self, name: &str) -> Option<&'static AdvanceGroup> {
        self.advance_groups_by_name.get(name)
    }

    #[must_use]
    pub fn get_governments(&'static self) -> &'static Vec<AdvanceGroup> {
        &self.all_governments
    }

    #[must_use]
    pub fn get_government(&'static self, name: &str) -> Option<&'static AdvanceGroup> {
        self.governments_by_name.get(name)
    }

    #[must_use]
    pub fn get_builtins(&'static self) -> &'static Vec<Builtin> {
        &self.all_builtins
    }

    #[must_use]
    pub fn get_builtin(&'static self, name: &str, game: &Game) -> Option<&'static Builtin> {
        self.builtins_by_name.get(name).or_else(|| {
            if let Some(p) = get_status_phase(game) {
                return Some(self.status_phase_handler(p));
            }
            None
        })
    }

    #[must_use]
    pub fn status_phase_handler(&'static self, p: &StatusPhaseState) -> &'static Builtin {
        if let DetermineFirstPlayer(_) = p {
            return &self.status_phase_handlers[&DetermineFirstPlayer(0)];
        }
        &self.status_phase_handlers[p]
    }

    #[must_use]
    pub fn get_action_cards(&'static self) -> &'static Vec<ActionCard> {
        &self.all_action_cards
    }

    #[must_use]
    pub fn get_action_card(&'static self, id: u8) -> Option<&'static ActionCard> {
        self.action_cards_by_id.get(&id)
    }

    #[must_use]
    pub fn get_objective_cards(&'static self) -> &'static Vec<ObjectiveCard> {
        &self.all_objective_cards
    }

    #[must_use]
    pub fn get_objective_card(&'static self, id: u8) -> Option<&'static ObjectiveCard> {
        self.objective_cards_by_id.get(&id)
    }

    #[must_use]
    pub fn get_objectives(&'static self) -> &'static Vec<Objective> {
        &self.all_objectives
    }

    #[must_use]
    pub fn get_objective(&'static self, name: &str) -> Option<&'static Objective> {
        self.objectives_by_name.get(name)
    }

    #[must_use]
    pub fn get_wonders(&'static self) -> &'static Vec<Wonder> {
        &self.all_wonders
    }

    #[must_use]
    pub fn get_wonder(&'static self, name: &str) -> Option<&'static Wonder> {
        self.wonders_by_name.get(name)
    }

    #[must_use]
    pub fn get_incidents(&'static self) -> &'static Vec<Incident> {
        &self.all_incidents
    }

    #[must_use]
    pub fn get_incident(&'static self, id: u8) -> Option<&'static Incident> {
        self.incidents_by_id.get(&id)
    }
}

fn status_phase_handlers() -> HashMap<StatusPhaseState, Builtin> {
    use StatusPhaseState::*;

    HashMap::from([
        (CompleteObjectives, complete_objectives()),
        (FreeAdvance, free_advance()),
        (DrawCards, draw_cards()),
        (RazeSize1City, raze_city()),
        (ChangeGovernmentType, may_change_government()),
        (DetermineFirstPlayer(0), determine_first_player()),
    ])
}
