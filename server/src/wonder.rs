use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::advance::Advance;
use crate::card::draw_card_from_pile;
use crate::city::{City, MoodState};
use crate::construct::can_construct_anything;
use crate::consts::WONDER_VICTORY_POINTS;
use crate::content::builtin::Builtin;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{PaymentRequest, PersistentEventType, PositionRequest};
use crate::events::EventOrigin;
use crate::log::{current_action_log_item, format_mood_change};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::utils::remove_element;
use crate::{ability_initializer::AbilityInitializerSetup, game::Game, position::Position};
use enumset::EnumSetType;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    pub fn name<'a>(&self, game: &'a Game) -> &'a str {
        self.info(game).name.as_str()
    }
}

#[derive(Clone)]
pub struct WonderInfo {
    pub wonder: Wonder,
    pub name: String,
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
        name: &str,
        description: &str,
        cost: PaymentOptions,
        required_advance: Advance,
    ) -> WonderBuilder {
        WonderBuilder::new(wonder, name, description, cost, required_advance)
    }
}

pub struct WonderBuilder {
    wonder: Wonder,
    name: String,
    descriptions: Vec<String>,
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
        name: &str,
        description: &str,
        cost: PaymentOptions,
        required_advance: Advance,
    ) -> Self {
        Self {
            wonder,
            name: name.to_string(),
            descriptions: vec![description.to_string()],
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
            name: self.name,
            description: String::from("✦ ") + &self.descriptions.join("\n✦ "),
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
}

pub(crate) fn draw_wonder_card(game: &mut Game, player_index: usize) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.draw_wonder_card,
        (),
        |()| PersistentEventType::DrawWonderCard,
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

pub(crate) fn on_draw_wonder_card() -> Builtin {
    Builtin::builder("Draw Wonder Card", "Draw a wonder card")
        .add_bool_request(
            |e| &mut e.draw_wonder_card,
            0,
            |game, player_index, ()| {
                let public_wonder = find_public_wonder(game);
                if let Some(public_wonder) = public_wonder {
                    Some(format!(
                        "Do you want to draw the public wonder card {}?",
                        public_wonder.name(game)
                    ))
                } else {
                    gain_wonder_from_pile(game, player_index);
                    None
                }
            },
            |game, s, ()| {
                if s.choice {
                    let name = *find_public_wonder(game).expect("public wonder card not found");
                    game.add_info_log_item(&format!(
                        "{} drew the public wonder card {}",
                        s.player_name,
                        name.name(game)
                    ));
                    gain_wonder(game, s.player_index, name);
                    game.permanent_effects
                        .retain(|e| !matches!(e, PermanentEffect::PublicWonderCard(_)));
                } else {
                    gain_wonder_from_pile(game, s.player_index);
                }
            },
        )
        .build()
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

fn find_public_wonder(game: &Game) -> Option<&Wonder> {
    game.permanent_effects.iter().find_map(|e| match e {
        PermanentEffect::PublicWonderCard(name) => Some(name),
        _ => None,
    })
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct WonderCardInfo {
    pub name: Wonder,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,
    pub discount: WonderDiscount,
}

impl WonderCardInfo {
    #[must_use]
    pub fn new(name: Wonder, discount: WonderDiscount) -> Self {
        Self {
            name,
            selected_position: None,
            discount,
        }
    }
}

pub(crate) fn can_construct_wonder(
    city: &City,
    wonder: Wonder,
    player: &Player,
    game: &Game,
    discount: &WonderDiscount,
) -> Result<PaymentOptions, String> {
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
    if !discount.ignore_required_advances {
        let a = info.required_advance;
        if !player.can_use_advance(a) {
            return Err(format!("Advance missing: {a:?}"));
        }
    }
    if let Some(placement_requirement) = &info.placement_requirement {
        if !placement_requirement(city.position, game) {
            return Err("Placement requirement not met".to_string());
        }
    }
    let mut cost = info.cost.clone();
    cost.default.culture_tokens = cost
        .default
        .culture_tokens
        .saturating_sub(discount.culture_tokens);

    if !player.can_afford(&cost) {
        return Err("Not enough resources".to_string());
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct WonderDiscount {
    ignore_required_advances: bool,
    culture_tokens: u8,
}

impl WonderDiscount {
    #[must_use]
    pub const fn new(ignore_required_advances: bool, discount_culture_tokens: u8) -> Self {
        Self {
            ignore_required_advances,
            culture_tokens: discount_culture_tokens,
        }
    }
}

impl Default for WonderDiscount {
    fn default() -> Self {
        Self::new(false, 0)
    }
}

pub(crate) fn build_wonder() -> Builtin {
    Builtin::builder("Build Wonder", "Build a wonder")
        .add_position_request(
            |e| &mut e.play_wonder_card,
            11,
            move |game, player_index, i| {
                let p = game.player(player_index);
                let choices = cities_for_wonder(i.name, game, p, &i.discount);

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
                    i.name.name(game),
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
                let cost = can_construct_wonder(city, i.name, p, game, &i.discount)
                    .expect("can't construct wonder");
                Some(vec![PaymentRequest::mandatory(cost, "Pay to build wonder")])
            },
            |game, s, i| {
                let pos = i.selected_position.expect("city not selected");
                let name = i.name;

                game.add_info_log_item(&format!(
                    "{} built {} in city {pos} for {}{}",
                    s.player_name,
                    name.name(game),
                    s.choice[0],
                    format_mood_change(game.player(s.player_index), pos)
                ));
                current_action_log_item(game).wonder_built = Some(name);
                remove_element(&mut game.player_mut(s.player_index).wonder_cards, &name);
                construct_wonder(game, name, pos, s.player_index);
            },
        )
        .build()
}

pub(crate) fn cities_for_wonder(
    name: Wonder,
    game: &Game,
    p: &Player,
    discount: &WonderDiscount,
) -> Vec<Position> {
    p.cities
        .iter()
        .filter_map(|c| {
            can_construct_wonder(c, name, p, game, discount)
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

pub(crate) fn init_wonder(game: &mut Game, owner: usize, name: &Wonder) {
    let wonder = game.cache.get_wonder(*name);
    wonder.listeners.clone().init(game, owner);
}

pub(crate) fn deinit_wonder(game: &mut Game, owner: usize, name: &Wonder) {
    let wonder = game.cache.get_wonder(*name);
    wonder.listeners.clone().deinit(game, owner);
}
