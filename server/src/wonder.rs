use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::advance::Advance;
use crate::card::draw_card_from_pile;
use crate::city::{City, MoodState};
use crate::construct::can_construct_anything;
use crate::consts::WONDER_VICTORY_POINTS;
use crate::content::ability::Ability;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{PaymentRequest, PersistentEventType, PositionRequest};
use crate::events::EventOrigin;
use crate::log::{current_action_log_item, format_mood_change};
use crate::payment::PaymentOptions;
use crate::player::{CostTrigger, Player};
use crate::player_events::CostInfo;
use crate::utils::remove_element;
use crate::{ability_initializer::AbilityInitializerSetup, game::Game, position::Position};
use enumset::EnumSetType;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Arc;

const DRAW_REPLACEMENT_WONDER: &str = "draw_replacement_wonder";

type PlacementChecker = Arc<dyn Fn(Position, &Game) -> bool + Sync + Send>;

#[derive(EnumSetType, Serialize, Deserialize, Debug, Ord, PartialOrd, Hash)]
pub enum Wonder {
    Colosseum = 0,
    Pyramids,
    GreatGardens,
    GreatLibrary,
    GreatLighthouse,
    GreatMausoleum,
    GreatStatue,
    GreatWall,
    Hidden,
}

impl Wonder {
    #[must_use]
    pub fn info<'a>(&self, game: &'a Game) -> &'a WonderInfo {
        game.cache.get_wonder(*self)
    }

    #[must_use]
    pub fn id(&self) -> String {
        format!("{self:?}")
    }

    #[must_use]
    pub fn name(&self) -> String {
        self.to_string()
    }
}

impl Display for Wonder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Wonder::Colosseum => write!(f, "Colosseum"),
            Wonder::Pyramids => write!(f, "Pyramids"),
            Wonder::GreatGardens => write!(f, "Great Gardens"),
            Wonder::GreatLibrary => write!(f, "Great Library"),
            Wonder::GreatLighthouse => write!(f, "Great Lighthouse"),
            Wonder::GreatMausoleum => write!(f, "Great Mausoleum"),
            Wonder::GreatStatue => write!(f, "Great Statue"),
            Wonder::GreatWall => write!(f, "Great Wall"),
            Wonder::Hidden => write!(f, "Hidden"),
        }
    }
}

#[derive(Clone)]
pub struct WonderInfo {
    pub wonder: Wonder,
    pub description: String,
    pub cost: PaymentOptions,
    pub required_advance: Advance,
    pub placement_requirement: Option<PlacementChecker>,
    pub listeners: AbilityListeners,
    pub owned_victory_points: u8,
    pub built_victory_points: f32,
}

impl WonderInfo {
    #[must_use]
    pub fn builder(
        wonder: Wonder,
        description: &str,
        cost: PaymentOptions,
        required_advance: Advance,
    ) -> WonderBuilder {
        WonderBuilder::new(wonder, description, cost, required_advance)
    }

    #[must_use]
    pub fn name(&self) -> String {
        self.wonder.name()
    }
}

pub struct WonderBuilder {
    wonder: Wonder,
    description: String,
    cost: PaymentOptions,
    required_advance: Advance,
    placement_requirement: Option<PlacementChecker>,
    builder: AbilityInitializerBuilder,
    pub owned_victory_points: u8,
    pub built_victory_points: f32,
}

impl WonderBuilder {
    fn new(
        wonder: Wonder,
        description: &str,
        cost: PaymentOptions,
        required_advance: Advance,
    ) -> Self {
        Self {
            wonder,
            description: description.to_string(),
            cost,
            required_advance,
            placement_requirement: None,
            builder: AbilityInitializerBuilder::new(),
            built_victory_points: WONDER_VICTORY_POINTS as f32 / 2.0,
            owned_victory_points: WONDER_VICTORY_POINTS / 2,
        }
    }

    #[must_use]
    pub fn placement_requirement(mut self, placement_requirement: PlacementChecker) -> Self {
        self.placement_requirement = Some(placement_requirement);
        self
    }

    #[must_use]
    pub fn built_victory_points(mut self, points: f32) -> Self {
        self.built_victory_points = points;
        self
    }

    #[must_use]
    pub fn owned_victory_points(mut self, points: u8) -> Self {
        self.owned_victory_points = points;
        self
    }

    #[must_use]
    pub fn build(self) -> WonderInfo {
        WonderInfo {
            wonder: self.wonder,
            description: self.description,
            cost: self.cost,
            required_advance: self.required_advance,
            placement_requirement: self.placement_requirement,
            listeners: self.builder.build(),
            owned_victory_points: self.owned_victory_points,
            built_victory_points: self.built_victory_points,
        }
    }
}

impl AbilityInitializerSetup for WonderBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Wonder(self.wonder)
    }

    fn name(&self) -> String {
        self.wonder.name()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

pub(crate) fn draw_wonder_card(game: &mut Game, player_index: usize) {
    on_draw_wonder_card(game, player_index, false);
}

pub(crate) fn on_draw_wonder_card(game: &mut Game, player_index: usize, drawn: bool) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.draw_wonder_card,
        drawn,
        PersistentEventType::DrawWonderCard,
    );
}

pub(crate) fn draw_wonder_from_pile(game: &mut Game) -> Option<Wonder> {
    draw_card_from_pile(
        game,
        "Wonders",
        |game| &mut game.wonders_left,
        |_| Vec::new(),
        |_| vec![],
    )
}

fn gain_wonder(game: &mut Game, player_index: usize, wonder: Wonder) {
    game.players[player_index].wonder_cards.push(wonder);
}

pub(crate) fn draw_wonder_card_handler() -> Ability {
    Ability::builder("Draw Wonder Card", "Draw a wonder card")
        .add_bool_request(
            |e| &mut e.draw_wonder_card,
            0,
            |game, player_index, drawn| {
                if *drawn {
                    return None;
                }

                let public_wonder = find_public_wonder(game);
                if let Some(public_wonder) = public_wonder {
                    Some(format!(
                        "Do you want to draw the public wonder card {}?",
                        public_wonder.name()
                    ))
                } else {
                    gain_wonder_from_pile(game, player_index);
                    None
                }
            },
            |game, s, _| {
                if s.choice {
                    let name = *find_public_wonder(game).expect("public wonder card not found");
                    game.add_info_log_item(&format!(
                        "{} drew the public wonder card {}",
                        s.player_name,
                        name.name()
                    ));
                    gain_wonder(game, s.player_index, name);
                    remove_public_wonder(game);
                } else {
                    gain_wonder_from_pile(game, s.player_index);
                }
            },
        )
        .build()
}

pub(crate) fn force_draw_wonder_from_anywhere(
    game: &mut Game,
    player: usize,
    wonder: Wonder,
) -> bool {
    if remove_element(&mut game.wonders_left, &wonder).is_some() {
        gain_specific_wonder(game, player, wonder, "the wonder pile");
        true
    } else if find_public_wonder(game).is_some_and(|w| w == &wonder) {
        gain_specific_wonder(game, player, wonder, "the public wonder card");
        remove_public_wonder(game);
        draw_public_wonder(game);
        true
    } else if let Some(last_player) = player_with_wonder_card(game, wonder) {
        let l = game.player_name(last_player);
        gain_specific_wonder(game, player, wonder, &l);

        let p = game.player_mut(last_player);
        remove_element(&mut p.wonder_cards, &wonder);
        p.event_info
            .insert(DRAW_REPLACEMENT_WONDER.to_string(), "true".to_string());
        true
    } else if game
        .players
        .iter()
        .any(|p| p.wonders_built.contains(&wonder))
    {
        false
    } else {
        panic!("Wonder card {wonder} not found in the game state, but was requested to be drawn.")
    }
}

fn gain_specific_wonder(game: &mut Game, player: usize, wonder: Wonder, source: &str) {
    game.add_info_log_item(&format!(
        "{} gained wonder card {wonder} from {source} using {wonder}",
        game.player_name(player),
    ));
    gain_wonder(game, player, wonder);
}

fn player_with_wonder_card(game: &Game, wonder: Wonder) -> Option<usize> {
    game.players
        .iter()
        .position(|p| p.wonder_cards.iter().any(|w| w == &wonder))
}

fn gain_wonder_from_pile(game: &mut Game, player: usize) {
    if let Some(w) = draw_wonder_from_pile(game) {
        game.add_info_log_item(&format!(
            "{} drew a wonder card from the pile",
            game.player_name(player)
        ));
        gain_wonder(game, player, w);
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct WonderCardInfo {
    pub wonder: Wonder,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,
    pub cost: CostInfo,
}

impl WonderCardInfo {
    #[must_use]
    pub fn new(wonder: Wonder, cost: CostInfo) -> Self {
        Self {
            wonder,
            cost,
            selected_position: None,
        }
    }
}

pub(crate) struct WonderBuildInfo {
    pub wonder: Wonder,
    pub city_position: Position,
    pub player: usize,
}

impl WonderBuildInfo {
    #[must_use]
    pub fn new(wonder: Wonder, city_position: Position, player: usize) -> Self {
        Self {
            wonder,
            city_position,
            player,
        }
    }
}

pub(crate) fn can_construct_wonder(
    city: &City,
    wonder: Wonder,
    player: &Player,
    game: &Game,
    cost: CostInfo,
    trigger: CostTrigger,
) -> Result<CostInfo, String> {
    can_construct_anything(city, player)?;

    if !player.wonder_cards.contains(&wonder) {
        return Err("Wonder card not owned".to_string());
    }
    let info = wonder.info(game);

    if city.mood_state != MoodState::Happy {
        return Err("City is not happy".to_string());
    }
    if !player.can_use_advance(Advance::Engineering) {
        return Err("Engineering advance missing".to_string());
    }
    if let Some(placement_requirement) = &info.placement_requirement {
        if !placement_requirement(city.position, game) {
            return Err("Placement requirement not met".to_string());
        }
    }
    let cost = player.trigger_cost_event(
        |e| &e.wonder_cost,
        cost,
        &WonderBuildInfo::new(wonder, city.position, player.index),
        game,
        trigger,
    );

    if !cost.ignore_required_advances {
        let a = info.required_advance;
        if !player.can_use_advance(a) {
            return Err(format!("Advance missing: {a:?}"));
        }
    }

    if !player.can_afford(&cost.cost) {
        return Err("Not enough resources".to_string());
    }

    if game.actions_left == 0 && !cost.ignore_action_cost {
        return Err("Not enough actions left".to_string());
    }

    Ok(cost)
}

pub(crate) fn on_play_wonder_card(game: &mut Game, player_index: usize, i: WonderCardInfo) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.play_wonder_card,
        i,
        PersistentEventType::WonderCard,
    );
}

pub(crate) fn build_wonder_handler() -> Ability {
    Ability::builder("Build Wonder", "Build a wonder")
        .add_position_request(
            |e| &mut e.play_wonder_card,
            11,
            move |game, player_index, i| {
                let p = game.player(player_index);
                let choices = cities_for_wonder(i.wonder, game, p, i.cost.clone());

                let needed = 1..=1;
                Some(PositionRequest::new(
                    choices,
                    needed,
                    "Select city to build wonder",
                ))
            },
            |game, s, i| {
                let position = s.choice[0];
                i.selected_position = Some(position);
                game.add_info_log_item(&format!(
                    "{} decided to build {} in city {}",
                    s.player_name,
                    i.wonder.name(),
                    position
                ));
            },
        )
        .add_payment_request_listener(
            |e| &mut e.play_wonder_card,
            10,
            move |game, player_index, i| {
                let p = game.player(player_index);
                let city = p.get_city(i.selected_position.expect("city not selected"));
                let cost = can_construct_wonder(
                    city,
                    i.wonder,
                    p,
                    game,
                    i.cost.clone(),
                    game.execute_cost_trigger(),
                )
                .expect("can't construct wonder");

                i.cost = cost.clone();

                if !cost.ignore_action_cost {
                    game.actions_left -= 1;
                }

                Some(vec![PaymentRequest::mandatory(
                    cost.cost,
                    "Pay to build wonder",
                )])
            },
            |game, s, i| {
                let pos = i.selected_position.expect("city not selected");
                let name = i.wonder;

                game.add_info_log_item(&format!(
                    "{} built {} in city {pos} for {}{}",
                    s.player_name,
                    name.name(),
                    s.choice[0],
                    format_mood_change(game.player(s.player_index), pos)
                ));
                i.cost.info.execute(game);
                current_action_log_item(game).wonder_built = Some(name);
                remove_element(&mut game.player_mut(s.player_index).wonder_cards, &name);
                construct_wonder(game, name, pos, s.player_index);
            },
        )
        .build()
}

pub(crate) fn cities_for_wonder(
    wonder: Wonder,
    game: &Game,
    p: &Player,
    cost: CostInfo,
) -> Vec<Position> {
    p.cities
        .iter()
        .filter_map(move |c| {
            can_construct_wonder(c, wonder, p, game, cost.clone(), CostTrigger::NoModifiers)
                .ok()
                .map(|_| c.position)
        })
        .collect_vec()
}

pub(crate) fn construct_wonder(
    game: &mut Game,
    name: Wonder,
    city_position: Position,
    player_index: usize,
) {
    let listeners = game.cache.get_wonder(name).listeners.clone();
    listeners.one_time_init(game, player_index);
    let player = &mut game.players[player_index];
    player.wonders_built.push(name);
    player.wonders_owned.insert(name);
    let city = player.get_city_mut(city_position);
    city.pieces.wonders.push(name);
    city.activate();
}

#[must_use]
pub(crate) fn wonders_owned_points(player: &Player, game: &Game) -> u8 {
    player
        .wonders_owned
        .iter()
        .map(|wonder| wonder.info(game).owned_victory_points)
        .sum::<u8>()
}

#[must_use]
pub(crate) fn wonders_built_points(player: &Player, game: &Game) -> f32 {
    player
        .wonders_built
        .iter()
        .map(|wonder| wonder.info(game).built_victory_points)
        .sum::<f32>()
}

pub(crate) fn init_wonder(game: &mut Game, owner: usize, name: Wonder) {
    let wonder = game.cache.get_wonder(name);
    wonder.listeners.clone().init(game, owner);
}

pub(crate) fn deinit_wonder(game: &mut Game, owner: usize, name: Wonder) {
    let wonder = game.cache.get_wonder(name);
    wonder.listeners.clone().deinit(game, owner);
}

fn remove_public_wonder(game: &mut Game) {
    game.permanent_effects
        .retain(|e| !matches!(e, PermanentEffect::PublicWonderCard(_)));
}

pub(crate) fn draw_public_wonder(game: &mut Game) {
    if let Some(wonder) = draw_wonder_from_pile(game) {
        game.add_info_log_item(&format!(
            "{} is now available to be taken by anyone",
            wonder.name()
        ));
        game.permanent_effects
            .push(PermanentEffect::PublicWonderCard(wonder));
    } else {
        game.add_info_log_item("No wonders left to draw as public wonder card");
    }
}

fn find_public_wonder(game: &Game) -> Option<&Wonder> {
    game.permanent_effects.iter().find_map(|e| match e {
        PermanentEffect::PublicWonderCard(name) => Some(name),
        _ => None,
    })
}

pub(crate) fn use_draw_replacement_wonder() -> Ability {
    Ability::builder(
        "Draw wonder card",
        "A leader ability took a wonder card away from you. \
        You can draw a replacement wonder card.",
    )
    .add_simple_persistent_event_listener(
        |e| &mut e.turn_start,
        3,
        |game, player_index, player_name, ()| {
            let p = game.player_mut(player_index);
            if p.event_info.remove(DRAW_REPLACEMENT_WONDER).is_some() {
                game.add_info_log_item(&format!(
                    "{player_name} gets to draw a replacement wonder card."
                ));
                draw_wonder_card(game, player_index);
            }
        },
    )
    .build()
}

pub(crate) fn wonder_cost(game: &Game, player: &Player, w: Wonder) -> CostInfo {
    CostInfo::new(player, w.info(game).cost.clone())
}
