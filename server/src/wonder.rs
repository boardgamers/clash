use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::card::draw_card_from_pile;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::{CurrentEventType, PaymentRequest, PositionRequest};
use crate::content::wonders::get_wonder;
use crate::events::EventOrigin;
use crate::incident::PermanentIncidentEffect;
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;
use crate::{ability_initializer::AbilityInitializerSetup, game::Game, position::Position};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use crate::city::{City, MoodState};
use crate::construct::can_construct_anything;
use crate::player::Player;

type PlacementChecker = Box<dyn Fn(Position, &Game) -> bool>;

pub struct Wonder {
    pub name: String,
    pub description: String,
    pub cost: PaymentOptions,
    pub required_advances: Vec<String>,
    pub placement_requirement: Option<PlacementChecker>,
    pub listeners: AbilityListeners,
}

impl Wonder {
    pub fn builder(
        name: &str,
        description: &str,
        cost: PaymentOptions,
        required_advances: Vec<&str>,
    ) -> WonderBuilder {
        WonderBuilder::new(
            name,
            description,
            cost,
            required_advances
                .into_iter()
                .map(ToString::to_string)
                .collect(),
        )
    }
}

pub struct WonderBuilder {
    name: String,
    descriptions: Vec<String>,
    cost: PaymentOptions,
    required_advances: Vec<String>,
    placement_requirement: Option<PlacementChecker>,
    builder: AbilityInitializerBuilder,
}

impl WonderBuilder {
    fn new(
        name: &str,
        description: &str,
        cost: PaymentOptions,
        required_advances: Vec<String>,
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
    let _ = game.trigger_current_event(
        &[player_index],
        |e| &mut e.on_draw_wonder_card,
        (),
        |()| CurrentEventType::DrawWonderCard,
    );
}

pub(crate) fn draw_wonder_from_pile(game: &mut Game) -> Option<Wonder> {
    draw_card_from_pile(
        game,
        "Wonders",
        false,
        |game| &mut game.wonders_left,
        Vec::new,
        |_| vec![], // can't reshuffle wonders
    )
    .map(|n| get_wonder(&n))
}

fn gain_wonder(game: &mut Game, player_index: usize, wonder: Wonder) {
    game.players[player_index].wonder_cards.push(wonder.name);
}

pub(crate) fn on_draw_wonder_card() -> Builtin {
    Builtin::builder("Draw Wonder Card", "Draw a wonder card")
        .add_bool_request(
            |e| &mut e.on_draw_wonder_card,
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
                    gain_wonder(game, s.player_index, get_wonder(&name));
                    game.permanent_incident_effects
                        .retain(|e| !matches!(e, PermanentIncidentEffect::PublicWonderCard(_)));
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
    game.permanent_incident_effects
        .iter()
        .find_map(|e| match e {
            PermanentIncidentEffect::PublicWonderCard(name) => Some(name),
            _ => None,
        })
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct WonderCardInfo {
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,
}

impl WonderCardInfo {
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            name,
            selected_position: None,
        }
    }
}


pub(crate) fn can_construct_wonder(
    city: &City,
    wonder: &Wonder,
    player: &Player,
    game: &Game,
    discount: WonderDiscount,
) -> Result<PaymentOptions, String> {
    can_construct_anything(city, player)?;
    if !player.wonder_cards.contains(&wonder.name) {
        return Err("Wonder card not owned".to_string());
    }
    if city.mood_state != MoodState::Happy {
        return Err("City is not happy".to_string());
    }
    if !player.has_advance("Engineering") {
        return Err("Engineering advance missing".to_string());
    }
    if !discount.ignore_required_advances {
        for advance in &wonder.required_advances {
            if !player.has_advance(advance) {
                return Err(format!("Advance missing: {advance}"));
            }
        }
    }
    if let Some(placement_requirement) = &wonder.placement_requirement {
        if !placement_requirement(city.position, game) {
            return Err("Placement requirement not met".to_string());
        }
    }
    let mut cost = wonder.cost.clone();
    cost.default.culture_tokens = cost.default.culture_tokens.saturating_sub(discount.culture_tokens);

    if !player.can_afford(&cost) {
        return Err("Not enough resources".to_string());
    }

    Ok(cost)
}

pub(crate) fn play_wonder_card(game: &mut Game, player_index: usize, i: WonderCardInfo) {
    let _ = game.trigger_current_event(
        &[player_index],
        |e| &mut e.on_play_wonder_card,
        i,
        CurrentEventType::WonderCard,
    );
}

#[derive(Clone, Copy)]
pub(crate) struct WonderDiscount {
    ignore_required_advances: bool,
    culture_tokens: u32,
}

impl WonderDiscount {
    #[must_use]
    pub fn new(ignore_required_advances: bool, discount_culture_tokens: u32) -> Self {
        Self {
            ignore_required_advances,
            culture_tokens: discount_culture_tokens,
        }
    }
    
    #[must_use]
    pub fn no_discount() -> Self {
        Self::new(false, 0)
    }
}

pub(crate) fn build_wonder() -> Builtin {
    add_build_wonder(
        Builtin::builder("Build Wonder", "Build a wonder"),
        WonderDiscount::no_discount(),
    )
    .build()
}

pub(crate) fn add_build_wonder<S: AbilityInitializerSetup>(b: S, discount: WonderDiscount) -> S {
    b.add_position_request(
        |e| &mut e.on_play_wonder_card,
        1,
        move |game, player_index, i| {
            let p = game.get_player(player_index);
            let choices = cities_for_wonder(&i.name, game, p, discount);

            Some(PositionRequest::new(
                choices,
                1..=1,
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
        |e| &mut e.on_play_wonder_card,
        0,
        move |game, player_index, i| {
            let p = game.get_player(player_index);
            let city = p.get_city(i.selected_position.expect("city not selected"));
            let wonder = get_wonder(&i.name);
            let cost = can_construct_wonder(city, &wonder, p, game, discount)
                .expect("can't construct wonder");
            Some(vec![PaymentRequest::new(
                cost,
                "Pay to build wonder",
                false,
            )])
        },
        |game, s, i| {
            let pos = i.selected_position.expect("city not selected");
            let wonder = get_wonder(&i.name);

            game.add_info_log_item(&format!(
                "{} built {} in city {pos} for {}",
                s.player_name, i.name, s.choice[0]
            ));

            construct_wonder(game, wonder, pos, s.player_index);
        },
    )
}

pub(crate) fn cities_for_wonder(name: &str, game: &Game, p: &Player, discount: WonderDiscount) -> Vec<Position> {
    p
        .cities
        .iter()
        .filter_map(|c| {
            let result = can_construct_wonder(c, &get_wonder(name), p, game, discount);
            result.ok().map(|_| c.position)
        })
        .collect_vec()
}

pub(crate) fn construct_wonder(
    game: &mut Game,
    wonder: Wonder,
    city_position: Position,
    player_index: usize,
) {
    (wonder.listeners.initializer)(game, player_index);
    (wonder.listeners.one_time_initializer)(game, player_index);
    let player = &mut game.players[player_index];
    player.wonders_build.push(wonder.name.clone());
    let name = wonder.name.clone();
    player
        .get_city_mut(city_position)
        .pieces
        .wonders
        .push(wonder);
}
