use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::advance::Advance;
use crate::card::draw_card_from_pile;
use crate::city::{City, MoodState};
use crate::construct::can_construct_anything;
use crate::content::builtin::Builtin;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{PaymentRequest, PersistentEventType, PositionRequest};
use crate::events::EventOrigin;
use crate::log::current_action_log_item;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::utils::remove_element;
use crate::{ability_initializer::AbilityInitializerSetup, game::Game, position::Position};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

type PlacementChecker = Arc<dyn Fn(Position, &Game) -> bool + Sync + Send>;

#[derive(Clone)]
pub struct Wonder {
    pub name: String,
    pub description: String,
    pub cost: PaymentOptions,
    pub required_advances: Vec<Advance>,
    pub placement_requirement: Option<PlacementChecker>,
    pub listeners: AbilityListeners,
}

impl Wonder {
    #[must_use]
    pub fn builder(
        name: &str,
        description: &str,
        cost: PaymentOptions,
        required_advances: Vec<Advance>,
    ) -> WonderBuilder {
        WonderBuilder::new(name, description, cost, required_advances)
    }
}

pub struct WonderBuilder {
    name: String,
    descriptions: Vec<String>,
    cost: PaymentOptions,
    required_advances: Vec<Advance>,
    placement_requirement: Option<PlacementChecker>,
    builder: AbilityInitializerBuilder,
}

impl WonderBuilder {
    fn new(
        name: &str,
        description: &str,
        cost: PaymentOptions,
        required_advances: Vec<Advance>,
    ) -> Self {
        Self {
            name: name.to_string(),
            descriptions: vec![description.to_string()],
            cost,
            required_advances,
            placement_requirement: None,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    pub fn placement_requirement(&mut self, placement_requirement: PlacementChecker) -> &mut Self {
        self.placement_requirement = Some(placement_requirement);
        self
    }

    #[must_use]
    pub fn build(self) -> Wonder {
        Wonder {
            name: self.name,
            description: String::from("✦ ") + &self.descriptions.join("\n✦ "),
            cost: self.cost,
            required_advances: self.required_advances,
            placement_requirement: self.placement_requirement,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for WonderBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Wonder(self.name.clone())
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

pub(crate) fn draw_wonder_from_pile(game: &mut Game) -> Option<String> {
    draw_card_from_pile(
        game,
        "Wonders",
        false,
        |game| &mut game.wonders_left,
        |_| Vec::new(),
        |_| vec![], // can't reshuffle wonders
    )
}

fn gain_wonder(game: &mut Game, player_index: usize, wonder: String) {
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
                        "Do you want to draw the public wonder card {public_wonder}?"
                    ))
                } else {
                    gain_wonder_from_pile(game, player_index);
                    None
                }
            },
            |game, s, ()| {
                if s.choice {
                    let name = find_public_wonder(game)
                        .expect("public wonder card not found")
                        .clone();
                    game.add_info_log_item(&format!(
                        "{} drew the public wonder card {}",
                        s.player_name, name
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

fn find_public_wonder(game: &Game) -> Option<&String> {
    game.permanent_effects.iter().find_map(|e| match e {
        PermanentEffect::PublicWonderCard(name) => Some(name),
        _ => None,
    })
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct WonderCardInfo {
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,
    pub discount: WonderDiscount,
}

impl WonderCardInfo {
    #[must_use]
    pub fn new(name: String, discount: WonderDiscount) -> Self {
        Self {
            name,
            selected_position: None,
            discount,
        }
    }
}

pub(crate) fn can_construct_wonder(
    city: &City,
    wonder: &Wonder,
    player: &Player,
    game: &Game,
    discount: &WonderDiscount,
) -> Result<PaymentOptions, String> {
    can_construct_anything(city, player)?;

    if !player.wonder_cards.contains(&wonder.name) {
        return Err("Wonder card not owned".to_string());
    }
    if city.mood_state != MoodState::Happy {
        return Err("City is not happy".to_string());
    }
    if !player.has_advance(Advance::Engineering) {
        return Err("Engineering advance missing".to_string());
    }
    if !discount.ignore_required_advances {
        for advance in &wonder.required_advances {
            if !player.has_advance(*advance) {
                return Err(format!("Advance missing: {}", advance.name(game)));
            }
        }
    }
    if let Some(placement_requirement) = &wonder.placement_requirement {
        if !placement_requirement(city.position, game) {
            return Err("Placement requirement not met".to_string());
        }
    }
    let mut cost = wonder.cost.clone();
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
                let choices = cities_for_wonder(&i.name, game, p, &i.discount);

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
                    s.player_name, i.name, position
                ));
            },
        )
        .add_payment_request_listener(
            |e| &mut e.play_wonder_card,
            10,
            move |game, player_index, i| {
                let p = game.player(player_index);
                let city = p.get_city(i.selected_position.expect("city not selected"));
                let wonder = game.cache.get_wonder(&i.name);
                let cost = can_construct_wonder(city, wonder, p, game, &i.discount)
                    .expect("can't construct wonder");
                Some(vec![PaymentRequest::new(
                    cost,
                    "Pay to build wonder",
                    false,
                )])
            },
            |game, s, i| {
                let pos = i.selected_position.expect("city not selected");
                let name = &i.name;

                game.add_info_log_item(&format!(
                    "{} built {} in city {pos} for {}",
                    s.player_name, name, s.choice[0]
                ));
                current_action_log_item(game).wonder_built = Some(name.clone());
                remove_element(&mut game.player_mut(s.player_index).wonder_cards, name);
                construct_wonder(game, name, pos, s.player_index);
            },
        )
        .build()
}

pub(crate) fn cities_for_wonder(
    name: &str,
    game: &Game,
    p: &Player,
    discount: &WonderDiscount,
) -> Vec<Position> {
    p.cities
        .iter()
        .filter_map(|c| {
            can_construct_wonder(c, game.cache.get_wonder(name), p, game, discount)
                .ok()
                .map(|_| c.position)
        })
        .collect_vec()
}

pub(crate) fn construct_wonder(
    game: &mut Game,
    name: &str,
    city_position: Position,
    player_index: usize,
) {
    let listeners = game.cache.get_wonder(name).listeners.clone();
    listeners.one_time_init(game, player_index);
    let player = &mut game.players[player_index];
    player.wonders_build.push(name.to_string());
    player
        .get_city_mut(city_position)
        .pieces
        .wonders
        .push(name.to_string());
}
