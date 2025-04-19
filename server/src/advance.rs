use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::city_pieces::Building;
use crate::content::advances::get_advance;
use crate::content::persistent_events::PersistentEventType;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::incident::on_trigger_incident;
use crate::player_events::{IncidentInfo, OnAdvanceInfo};
use crate::special_advance::SpecialAdvance;
use crate::{ability_initializer::AbilityInitializerSetup, resource_pile::ResourcePile, utils};
use Bonus::*;
use enumset::EnumSetType;
use std::mem;
use serde::{Deserialize, Serialize};

// id / 4 = advance group
#[derive(EnumSetType, Serialize, Deserialize, Debug, Ord, PartialOrd, Hash)]
pub enum Advance {
    // Farming Group
    Farming = 0,
    Storage = 1,
    Irrigation = 2,
    Husbandry = 3,

    // Construction Group
    Mining = 4,
    Engineering = 5,
    Sanitation = 6,
    Roads = 7,

    // Seafaring Group
    Fishing = 8,
    Navigation = 9,
    WarShips = 10,
    Cartography = 11,

    // Education Group
    Writing = 12,
    PublicEducation = 13,
    FreeEducation = 14,
    Philosophy = 15,

    // Warfare Group
    Tactics = 16,
    Siegecraft = 17,
    SteelWeapons = 18,
    Draft = 19,

    // Spirituality Group
    Myths = 20,
    Rituals = 21,
    Priesthood = 22,
    StateReligion = 23,

    // Economy Group
    Bartering = 24,
    TradeRoutes = 25,
    Taxes = 26,
    Currency = 27,

    // Culture Group
    Arts = 28,
    Sports = 29,
    Monuments = 30,
    Theaters = 31,

    // Science Group
    Math = 32,
    Astronomy = 33,
    Medicine = 34,
    Metallurgy = 35,

    // Democracy Group
    Voting = 36,
    SeparationOfPower = 37,
    CivilLiberties = 38,
    FreeEconomy = 39,

    // Autocracy Group
    Nationalism = 40,
    Totalitarianism = 41,
    AbsolutePower = 42,
    ForcedLabor = 43,

    // Theocracy Group
    Dogma = 44,
    Devotion = 45,
    Conversion = 46,
    Fanaticism = 47,
}

impl Advance {
    #[must_use]
    pub fn info(&self) -> &'static AdvanceInfo {
        get_advance(self)
    }
}

pub struct AdvanceInfo {
    pub advance: Advance,
    pub name: String,
    pub description: String,
    pub bonus: Option<Bonus>,
    pub required: Option<Advance>,
    pub contradicting: Vec<Advance>,
    pub unlocked_building: Option<Building>,
    pub government: Option<String>,
    pub listeners: AbilityListeners,
}

impl AdvanceInfo {
    #[must_use]
    pub(crate) fn builder(advance: Advance, name: &str, description: &str) -> AdvanceBuilder {
        AdvanceBuilder::new(advance, name.to_string(), description.to_string())
    }
}

impl PartialEq for AdvanceInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub(crate) struct AdvanceBuilder {
    pub advance: Advance,
    pub name: String,
    description: String,
    advance_bonus: Option<Bonus>,
    pub required_advance: Option<Advance>,
    contradicting_advance: Vec<Advance>,
    unlocked_building: Option<Building>,
    government: Option<String>,
    builder: AbilityInitializerBuilder,
}

impl AdvanceBuilder {
    fn new(advance: Advance, name: String, description: String) -> Self {
        Self {
            advance,
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
    pub fn with_required_advance(mut self, required_advance: Advance) -> Self {
        self.required_advance = Some(required_advance);
        self
    }

    #[must_use]
    pub fn with_contradicting_advance(mut self, contradicting_advance: &[Advance]) -> Self {
        self.contradicting_advance = contradicting_advance.to_vec();
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
    pub fn build(self) -> AdvanceInfo {
        AdvanceInfo {
            advance: self.advance,
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
        EventOrigin::Advance(self.advance)
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
pub fn do_advance(game: &mut Game, advance: Advance, player_index: usize) {
    let info = advance.info();
    info.listeners.one_time_init(game, player_index);
    for i in 0..game.players[player_index]
        .civilization
        .special_advances
        .len()
    {
        if game.players[player_index].civilization.special_advances[i].required_advance == advance {
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
    if let Some(advance_bonus) = &info.bonus {
        let pile = advance_bonus.resources();
        game.add_info_log_item(&format!("Player gained {pile} as advance bonus"));
        game.players[player_index].gain_resources(pile);
    }
    let player = &mut game.players[player_index];
    player.advances.insert(advance);
}

pub(crate) fn gain_advance_without_payment(
    game: &mut Game,
    advance: Advance,
    player_index: usize,
    payment: ResourcePile,
    take_incident_token: bool,
) {
    do_advance(game, advance, player_index);
    on_advance(game, player_index, OnAdvanceInfo {
        advance,
        payment,
        take_incident_token,
    });
}

pub(crate) fn on_advance(game: &mut Game, player_index: usize, info: OnAdvanceInfo) {
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

pub(crate) fn remove_advance(game: &mut Game, advance: Advance, player_index: usize) {
    let info = advance.info();
    info.listeners.undo(game, player_index);

    for i in 0..game.players[player_index]
        .civilization
        .special_advances
        .len()
    {
        if game.players[player_index].civilization.special_advances[i].required_advance == advance {
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
    if let Some(advance_bonus) = &info.bonus {
        player.lose_resources(advance_bonus.resources());
    }
    game.player_mut(player_index).advances.remove(advance);
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

pub(crate) fn init_player(game: &mut Game, player_index: usize) {
    let advances = mem::take(&mut game.player_mut(player_index).advances);
    for advance in advances.iter() {
        let info = advance.info();
        info.listeners.init(game, player_index);
        for i in 0..game
            .player(player_index)
            .civilization
            .special_advances
            .len()
        {
            if game.players[player_index].civilization.special_advances[i].required_advance
                == advance
            {
                let special_advance = game
                    .player_mut(player_index)
                    .civilization
                    .special_advances
                    .remove(i);
                special_advance.listeners.init(game, player_index);
                game.players[player_index]
                    .civilization
                    .special_advances
                    .insert(i, special_advance);
                break;
            }
        }
    }
    game.player_mut(player_index).advances = advances;
}
