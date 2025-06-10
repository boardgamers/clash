use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::barbarians::barbarians_bonus;
use crate::combat_listeners::{choose_fighter_casualties, offer_retreat, place_settler};
use crate::content::action_cards::cultural_takeover::use_cultural_takeover;
use crate::content::action_cards::development::collect_only;
use crate::content::action_cards::negotiation::{use_assassination, use_negotiations};
use crate::content::action_cards::synergies::use_teach_us;
use crate::content::advances::education::use_academy;
use crate::content::advances::science::use_observatory;
use crate::content::advances::seafaring::fishing_collect;
use crate::content::advances::spirituality::use_temple;
use crate::content::civilizations::vikings::lose_raid_resource;
use crate::content::incidents::famine::pestilence_permanent_effect;
use crate::content::incidents::great_builders::construct_only;
use crate::content::incidents::great_diplomat::use_diplomatic_relations;
use crate::content::incidents::great_warlord::use_great_warlord;
use crate::content::incidents::trojan::{
    anarchy_advance, decide_trojan_horse, solar_eclipse_end_combat,
};
use crate::content::wonders::use_great_mausoleum;
use crate::cultural_influence::use_cultural_influence;
use crate::events::EventOrigin;
use crate::explore::explore_resolution;
use crate::game::Game;
use crate::objective_card::select_objectives;
use crate::pirates::{pirates_bonus, pirates_round_bonus};
use crate::playing_actions::pay_for_action;
use crate::unit::choose_carried_units_to_remove;
use crate::wonder::{build_wonder_handler, draw_wonder_card_handler, use_draw_replacement_wonder};

#[derive(Clone)]
pub struct Ability {
    pub name: String,
    pub description: String,
    pub listeners: AbilityListeners,
}

impl Ability {
    #[must_use]
    pub fn builder(name: &str, description: &str) -> AbilityBuilder {
        AbilityBuilder::new(name, description)
    }
}

pub struct AbilityBuilder {
    name: String,
    description: String,
    builder: AbilityInitializerBuilder,
}

impl AbilityBuilder {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn build(self) -> Ability {
        Ability {
            name: self.name,
            description: self.description,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for AbilityBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Ability(self.name.clone())
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

#[must_use]
pub fn get_all_uncached() -> Vec<Ability> {
    vec![
        fishing_collect(),
        pay_for_action(),
        use_cultural_influence(),
        explore_resolution(),
        draw_wonder_card_handler(),
        build_wonder_handler(),
        choose_carried_units_to_remove(),
        select_objectives(),
        // building related
        use_academy(),
        use_observatory(),
        use_temple(),
        // combat related
        place_settler(),
        choose_fighter_casualties(),
        offer_retreat(),
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
        // wonder related
        use_great_mausoleum(),
        use_draw_replacement_wonder(),
        // civilization related
        lose_raid_resource(),
    ]
}

pub(crate) fn init_player(game: &mut Game, player_index: usize, all: &[Ability]) {
    for b in all {
        b.listeners.init(game, player_index);
    }
}
