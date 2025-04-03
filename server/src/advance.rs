use crate::{ability_initializer::AbilityInitializerSetup, resource_pile::ResourcePile, utils};

use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::city_pieces::Building;
use crate::content::advances::get_advance;
use crate::content::persistent_events::PersistentEventType;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::incident::on_trigger_incident;
use crate::player_events::{AdvanceInfo, IncidentInfo};
use crate::special_advance::SpecialAdvance;
use Bonus::*;

pub struct Advance {
    pub name: String,
    pub description: String,
    pub bonus: Option<Bonus>,
    pub required: Option<String>,
    pub contradicting: Vec<String>,
    pub unlocked_building: Option<Building>,
    pub government: Option<String>,
    pub listeners: AbilityListeners,
}

impl Advance {
    #[must_use]
    pub(crate) fn builder(name: &str, description: &str) -> AdvanceBuilder {
        AdvanceBuilder::new(name.to_string(), description.to_string())
    }
}

impl PartialEq for Advance {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub(crate) struct AdvanceBuilder {
    pub name: String,
    description: String,
    advance_bonus: Option<Bonus>,
    pub required_advance: Option<String>,
    contradicting_advance: Vec<String>,
    unlocked_building: Option<Building>,
    government: Option<String>,
    builder: AbilityInitializerBuilder,
}

impl AdvanceBuilder {
    fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            advance_bonus: None,
            required_advance: None,
            contradicting_advance: vec![],
            unlocked_building: None,
            government: None,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn with_advance_bonus(mut self, advance_bonus: Bonus) -> Self {
        self.advance_bonus = Some(advance_bonus);
        self
    }

    #[must_use]
    pub fn with_required_advance(mut self, required_advance: &str) -> Self {
        self.required_advance = Some(required_advance.to_string());
        self
    }

    #[must_use]
    pub fn with_contradicting_advance(mut self, contradicting_advance: &[&str]) -> Self {
        self.contradicting_advance = contradicting_advance
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        self
    }

    #[must_use]
    pub fn with_unlocked_building(mut self, unlocked_building: Building) -> Self {
        self.unlocked_building = Some(unlocked_building);
        self
    }

    #[must_use]
    pub fn with_government(mut self, government: &str) -> Self {
        self.government = Some(government.to_string());
        self
    }

    #[must_use]
    pub fn build(self) -> Advance {
        Advance {
            name: self.name,
            description: self.description,
            bonus: self.advance_bonus,
            required: self.required_advance,
            contradicting: self.contradicting_advance,
            unlocked_building: self.unlocked_building,
            government: self.government,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for AdvanceBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Advance(self.name.clone())
    }
}

pub enum Bonus {
    MoodToken,
    CultureToken,
}

impl Bonus {
    #[must_use]
    pub fn resources(&self) -> ResourcePile {
        match self {
            MoodToken => ResourcePile::mood_tokens(1),
            CultureToken => ResourcePile::culture_tokens(1),
        }
    }
}

///
///
/// # Panics
///
/// Panics if advance does not exist
pub fn do_advance(game: &mut Game, advance: &Advance, player_index: usize) {
    advance.listeners.one_time_init(game, player_index);
    let name = advance.name.clone();
    for i in 0..game.players[player_index]
        .civilization
        .special_advances
        .len()
    {
        if game.players[player_index].civilization.special_advances[i].required_advance == name {
            let special_advance = game.players[player_index]
                .civilization
                .special_advances
                .remove(i);
            unlock_special_advance(game, &special_advance, player_index);
            game.players[player_index]
                .civilization
                .special_advances
                .insert(i, special_advance);
            break;
        }
    }
    if let Some(advance_bonus) = &advance.bonus {
        let pile = advance_bonus.resources();
        game.add_info_log_item(&format!("Player gained {pile} as advance bonus"));
        game.players[player_index].gain_resources(pile);
    }
    let player = &mut game.players[player_index];
    player.advances.push(get_advance(&advance.name));
}

pub(crate) fn gain_advance_without_payment(
    game: &mut Game,
    name: &str,
    player_index: usize,
    payment: ResourcePile,
    take_incident_token: bool,
) {
    do_advance(game, &get_advance(name), player_index);
    on_advance(game, player_index, AdvanceInfo {
        name: name.to_string(),
        payment,
        take_incident_token,
    });
}

pub(crate) fn on_advance(game: &mut Game, player_index: usize, info: AdvanceInfo) {
    let info = match game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.advance,
        info,
        PersistentEventType::Advance,
    ) {
        None => return,
        Some(i) => i,
    };

    if info.take_incident_token {
        let player = game.player_mut(player_index);
        player.incident_tokens -= 1;
        if player.incident_tokens == 0 {
            player.incident_tokens = 3;
            on_trigger_incident(game, IncidentInfo::new(player_index));
        }
    }
}

pub(crate) fn remove_advance(game: &mut Game, advance: &Advance, player_index: usize) {
    advance.listeners.undo(game, player_index);

    for i in 0..game.players[player_index]
        .civilization
        .special_advances
        .len()
    {
        if game.players[player_index].civilization.special_advances[i].required_advance
            == advance.name
        {
            let special_advance = game.players[player_index]
                .civilization
                .special_advances
                .remove(i);
            undo_unlock_special_advance(game, &special_advance, player_index);
            game.players[player_index]
                .civilization
                .special_advances
                .insert(i, special_advance);
            break;
        }
    }
    let player = &mut game.players[player_index];
    if let Some(advance_bonus) = &advance.bonus {
        player.lose_resources(advance_bonus.resources());
    }
    utils::remove_element(&mut game.players[player_index].advances, advance);
}

fn unlock_special_advance(game: &mut Game, special_advance: &SpecialAdvance, player_index: usize) {
    special_advance.listeners.one_time_init(game, player_index);
    game.players[player_index]
        .unlocked_special_advances
        .push(special_advance.name.clone());
}

fn undo_unlock_special_advance(
    game: &mut Game,
    special_advance: &SpecialAdvance,
    player_index: usize,
) {
    special_advance.listeners.undo(game, player_index);
    game.players[player_index].unlocked_special_advances.pop();
}
