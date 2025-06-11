use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::city_pieces::Building;
use crate::consts::ADVANCE_COST;
use crate::content::ability::advance_event_origin;
use crate::content::persistent_events::PersistentEventType;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::incident::trigger_incident;
use crate::payment::PaymentOptions;
use crate::player::{Player, gain_resources};
use crate::player_events::OnAdvanceInfo;
use crate::resource::ResourceType;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceRequirement};
use crate::{ability_initializer::AbilityInitializerSetup, resource_pile::ResourcePile};
use Bonus::*;
use enumset::{EnumSet, EnumSetType};
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
    pub fn info<'a>(&self, game: &'a Game) -> &'a AdvanceInfo {
        game.cache.get_advance(*self)
    }

    #[must_use]
    pub fn id(&self) -> String {
        format!("{self:?}")
    }

    #[must_use]
    pub fn name<'a>(&self, game: &'a Game) -> &'a str {
        self.info(game).name.as_str()
    }
}

#[derive(Clone)]
pub struct AdvanceInfo {
    pub advance: Advance,
    pub name: String,
    pub description: String,
    pub bonus: Option<Bonus>,
    pub required: Option<Advance>,
    pub contradicting: Vec<Advance>,
    pub unlocked_building: Option<Building>,
    pub government: Option<String>,
    pub leading_government: bool,
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
    leading_government: bool,
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
            leading_government: false,
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
    pub fn with_government(mut self, government: &str, leading: bool) -> Self {
        self.government = Some(government.to_string());
        self.leading_government = leading;
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
            leading_government: self.leading_government,
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

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

#[derive(Clone)]
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
    let info = advance.info(game).clone();
    let bonus = info.bonus.clone();
    info.listeners.one_time_init(game, player_index);

    if let Some(special_advance) = find_special_advance(advance, game, player_index) {
        unlock_special_advance(game, special_advance, player_index);
    }

    if let Some(advance_bonus) = &bonus {
        gain_resources(
            game,
            player_index,
            advance_bonus.resources(),
            |name, pile| format!("{name} gained {pile} as advance bonus"),
        );
    }
    let player = &mut game.players[player_index];
    player.advances.insert(advance);
}

#[must_use]
pub fn find_special_advance(
    advance: Advance,
    game: &Game,
    player_index: usize,
) -> Option<SpecialAdvance> {
    if advance.info(game).leading_government {
        find_government_special_advance(game, player_index)
    } else {
        find_non_government_special_advance(advance, game.player(player_index))
    }
}

#[must_use]
pub(crate) fn find_non_government_special_advance(
    advance: Advance,
    p: &Player,
) -> Option<SpecialAdvance> {
    p.civilization
        .special_advances
        .iter()
        .find_map(|s| match s.requirement {
            SpecialAdvanceRequirement::Advance(a) if a == advance => Some(s.advance),
            _ => None,
        })
}

#[must_use]
pub(crate) fn find_government_special_advance(
    game: &Game,
    player: usize,
) -> Option<SpecialAdvance> {
    let p = game.player(player);
    p.civilization
        .special_advances
        .iter()
        .find_map(|s| match s.requirement {
            SpecialAdvanceRequirement::AnyGovernment => Some(s.advance),
            SpecialAdvanceRequirement::Advance(_) => None,
        })
}

#[must_use]
pub fn is_special_advance_active(
    advance: SpecialAdvance,
    advances: EnumSet<Advance>,
    game: &Game,
) -> bool {
    match advance.info(game).requirement {
        SpecialAdvanceRequirement::AnyGovernment => player_government(game, advances).is_some(),
        SpecialAdvanceRequirement::Advance(a) => advances.contains(a),
    }
}

pub(crate) fn execute_advance_action(
    game: &mut Game,
    player_index: usize,
    a: &AdvanceAction,
) -> Result<(), String> {
    let advance = a.advance;
    if !game.player(player_index).can_advance(advance, game) {
        return Err("Cannot advance".to_string());
    }
    game.add_info_log_item(&format!(
        "{} paid {} to get the {} advance",
        game.player_name(player_index),
        a.payment,
        a.advance.name(game)
    ));

    game.player(player_index)
        .advance_cost(advance, game, game.execute_cost_trigger())
        .pay(game, &a.payment);
    gain_advance_without_payment(game, advance, player_index, a.payment.clone(), true);
    Ok(())
}

pub(crate) fn gain_advance_without_payment(
    game: &mut Game,
    advance: Advance,
    player_index: usize,
    payment: ResourcePile,
    take_incident_token: bool,
) {
    do_advance(game, advance, player_index);
    on_advance(
        game,
        player_index,
        OnAdvanceInfo {
            advance,
            payment,
            take_incident_token,
        },
    );
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
            trigger_incident(game, player_index);
        }
    }
}

pub(crate) fn remove_advance(game: &mut Game, advance: Advance, player_index: usize) {
    let info = advance.info(game);
    let bonus = info.bonus.clone();
    info.listeners.clone().undo(game, player_index);

    if let Some(special_advance) =
        find_non_government_special_advance(advance, game.player(player_index))
    {
        undo_unlock_special_advance(game, special_advance, player_index);
    }

    let player = &mut game.players[player_index];
    if let Some(advance_bonus) = &bonus {
        player.lose_resources(advance_bonus.resources());
    }
    game.player_mut(player_index).advances.remove(advance);
}

fn unlock_special_advance(game: &mut Game, special_advance: SpecialAdvance, player_index: usize) {
    game.add_info_log_item(&format!(
        "{} unlocked {}",
        game.player_name(player_index),
        special_advance.info(game).name
    ));
    special_advance
        .info(game)
        .listeners
        .clone()
        .one_time_init(game, player_index);
    game.players[player_index]
        .special_advances
        .insert(special_advance);
}

pub(crate) fn undo_unlock_special_advance(
    game: &mut Game,
    special_advance: SpecialAdvance,
    player_index: usize,
) {
    special_advance
        .info(game)
        .listeners
        .clone()
        .undo(game, player_index);
    game.players[player_index]
        .special_advances
        .remove(special_advance);
}

pub(crate) fn init_player(game: &mut Game, player_index: usize) {
    for advance in game.player(player_index).advances {
        advance
            .info(game)
            .listeners
            .clone()
            .init(game, player_index);
    }
    for s in game.player(player_index).special_advances {
        if is_special_advance_active(s, game.player(player_index).advances, game) {
            s.info(game).listeners.clone().init(game, player_index);
        }
    }
}

pub(crate) fn init_great_library(game: &mut Game, player_index: usize) {
    if let Some(advance) = game.player(player_index).great_library_advance {
        advance
            .info(game)
            .listeners
            .clone()
            .init(game, player_index);
    }
}

pub(crate) fn base_advance_cost(player: &Player) -> PaymentOptions {
    PaymentOptions::sum(
        player,
        advance_event_origin(),
        ADVANCE_COST,
        &[ResourceType::Ideas, ResourceType::Food, ResourceType::Gold],
    )
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct AdvanceAction {
    pub advance: Advance,
    pub payment: ResourcePile,
}

impl AdvanceAction {
    #[must_use]
    pub fn new(advance: Advance, payment: ResourcePile) -> Self {
        Self { advance, payment }
    }
}

pub(crate) fn player_government(game: &Game, advances: EnumSet<Advance>) -> Option<String> {
    advances
        .iter()
        .find_map(|advance| advance.info(game).government.clone())
}
