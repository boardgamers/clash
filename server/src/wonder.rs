use serde::{Deserialize, Serialize};
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::card::draw_card_from_pile;
use crate::content::builtin::Builtin;
use crate::content::custom_phase_actions::CurrentEventType;
use crate::content::wonders::get_wonder;
use crate::events::EventOrigin;
use crate::incident::PermanentIncidentEffect;
use crate::payment::PaymentOptions;
use crate::{ability_initializer::AbilityInitializerSetup, game::Game, position::Position};
use crate::resource_pile::ResourcePile;

type PlacementChecker = Box<dyn Fn(Position, &Game) -> bool>;

pub struct Wonder {
    pub name: String,
    pub description: String,
    pub cost: PaymentOptions,
    pub required_advances: Vec<String>,
    pub placement_requirement: Option<PlacementChecker>,
    pub listeners: AbilityListeners,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ConstructWonder {
    pub city_position: Position,
    pub wonder: String,
    pub payment: ResourcePile,
}

impl ConstructWonder {
    #[must_use]
    pub fn new(city_position: Position, wonder: String, payment: ResourcePile) -> Self {
        Self {
            city_position,
            wonder,
            payment,
        }
    }
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
