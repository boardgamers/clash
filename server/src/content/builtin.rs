use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::barbarians::barbarians_bonus;
use crate::combat_listeners::{
    choose_fighter_casualties, combat_stats, offer_retreat, place_settler,
};
use crate::content::action_cards::cultural_takeover::use_cultural_takeover;
use crate::content::action_cards::development::collect_only;
use crate::content::action_cards::negotiation::{use_assassination, use_negotiations};
use crate::content::action_cards::synergies::use_teach_us;
use crate::content::incidents::famine::pestilence_permanent_effect;
use crate::content::incidents::great_builders::construct_only;
use crate::content::incidents::great_diplomat::use_diplomatic_relations;
use crate::content::incidents::great_warlord::use_great_warlord;
use crate::content::incidents::trojan::{
    anarchy_advance, decide_trojan_horse, solar_eclipse_end_combat,
};
use crate::cultural_influence::use_cultural_influence;
use crate::events::EventOrigin;
use crate::explore::explore_resolution;
use crate::game::Game;
use crate::objective_card::select_objectives;
use crate::pirates::{pirates_bonus, pirates_round_bonus};
use crate::unit::choose_carried_units_to_remove;
use crate::wonder::{build_wonder, on_draw_wonder_card};

#[derive(Clone)]
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
pub fn get_all_uncached() -> Vec<Builtin> {
    vec![
        use_cultural_influence(),
        explore_resolution(),
        on_draw_wonder_card(),
        build_wonder(),
        choose_carried_units_to_remove(),
        select_objectives(),
        // combat related
        place_settler(),
        choose_fighter_casualties(),
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
        construct_only(),
        use_diplomatic_relations(),
        // action card related
        collect_only(),
        use_cultural_takeover(),
        use_negotiations(),
        use_assassination(),
        use_teach_us(),
    ]
}

pub(crate) fn init_player(game: &mut Game, player_index: usize, all: &[Builtin]) {
    for b in all {
        b.listeners.init(game, player_index);
    }
}
