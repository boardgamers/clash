use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::barbarians::barbarians_bonus;
use crate::combat_listeners::{
    choose_carried_units_casualties, choose_fighter_casualties, offer_retreat, place_settler,
};
use crate::content::incidents_famine::pestilence_permanent_effect;
use crate::cultural_influence::cultural_influence_resolution;
use crate::events::EventOrigin;
use crate::explore::explore_resolution;
use crate::game::{Game, GameState};
use crate::pirates::{pirates_bonus, pirates_round_bonus};
use crate::status_phase::{
    complete_objectives, determine_first_player, draw_cards, free_advance, may_change_government,
    raze_city, StatusPhaseState,
};

pub struct Builtin {
    pub name: String,
    pub description: String,
    pub listeners: AbilityListeners,
}

impl Builtin {
    #[must_use]
    pub fn builder(name: &str, description: &str) -> BuiltinBuilder {
        BuiltinBuilder::new(name, description)
    }
}

pub struct BuiltinBuilder {
    name: String,
    descriptions: String,
    builder: AbilityInitializerBuilder,
}

impl BuiltinBuilder {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            descriptions: description.to_string(),
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn build(self) -> Builtin {
        Builtin {
            name: self.name,
            description: self.descriptions,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for BuiltinBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Builtin(self.name.clone())
    }
}

#[must_use]
pub fn get_all() -> Vec<Builtin> {
    vec![
        cultural_influence_resolution(),
        explore_resolution(),
        place_settler(),
        choose_fighter_casualties(),
        choose_carried_units_casualties(),
        offer_retreat(),
        barbarians_bonus(),
        pirates_bonus(),
        pirates_round_bonus(),
        pestilence_permanent_effect(),
    ]
}

///
/// # Panics
/// Panics if builtin does not exist
#[must_use]
pub fn get_builtin(game: &Game, name: &str) -> Builtin {
    get_all()
        .into_iter()
        .find(|builtin| builtin.name == name)
        .or_else(|| {
            if let GameState::StatusPhase(p) = game.state() {
                let handler = status_phase_handler(p);
                if handler.name == name {
                    return Some(handler);
                }
            }
            None
        })
        .unwrap_or_else(|| panic!("builtin not found: {name}"))
}

pub(crate) fn status_phase_handler(phase: &StatusPhaseState) -> Builtin {
    use StatusPhaseState::*;

    match phase {
        CompleteObjectives => complete_objectives(),
        FreeAdvance => free_advance(),
        RazeSize1City => raze_city(),
        DrawCards => draw_cards(),
        ChangeGovernmentType => may_change_government(),
        DetermineFirstPlayer(_) => determine_first_player(),
    }
}
