use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::barbarians::barbarians_bonus;
use crate::combat_listeners::{choose_carried_units_casualties, choose_fighter_casualties, combat_stats, offer_retreat, place_settler};
use crate::content::incidents::famine::pestilence_permanent_effect;
use crate::content::incidents::great_builders::use_great_engineer;
use crate::content::incidents::great_diplomat::use_diplomatic_relations;
use crate::content::incidents::great_warlord::use_great_warlord;
use crate::content::incidents::trojan::{
    anarchy_advance, decide_trojan_horse, solar_eclipse_end_combat,
};
use crate::cultural_influence::cultural_influence_resolution;
use crate::events::EventOrigin;
use crate::explore::explore_resolution;
use crate::game::Game;
use crate::pirates::{pirates_bonus, pirates_round_bonus};
use crate::status_phase::{
    complete_objectives, determine_first_player, draw_cards, free_advance, get_status_phase,
    may_change_government, raze_city, StatusPhaseState,
};
use crate::wonder::{build_wonder, on_draw_wonder_card};

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
        on_draw_wonder_card(),
        build_wonder(),
        // combat related
        place_settler(),
        choose_fighter_casualties(),
        choose_carried_units_casualties(),
        offer_retreat(),
        combat_stats(),
        // incident related
        barbarians_bonus(),
        pirates_bonus(),
        pirates_round_bonus(),
        pestilence_permanent_effect(),
        decide_trojan_horse(),
        solar_eclipse_end_combat(),
        anarchy_advance(),
        use_great_warlord(),
        use_great_engineer(),
        use_diplomatic_relations(),
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
            if let Some(p) = get_status_phase(game) {
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
        DrawCards => draw_cards(),
        RazeSize1City => raze_city(),
        ChangeGovernmentType => may_change_government(),
        DetermineFirstPlayer(_) => determine_first_player(),
    }
}

pub(crate) fn init_player(game: &mut Game, player_index: usize) {
    for b in get_all() {
        (b.listeners.initializer)(game, player_index);
    }
}
