use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{
    AbilityInitializerBuilder, AbilityListeners, SelectedMultiChoice, SelectedSingleChoice,
    SelectedWithoutChoices,
};
use crate::action_card::ActionCard;
use crate::advance::Advance;
use crate::barbarians::{barbarians_move, barbarians_spawn};
use crate::card::{HandCard, discard_card, draw_card_from_pile};
use crate::city::{MoodState, decrease_city_mood, is_valid_city_terrain, set_city_mood};
use crate::content::incidents::great_persons::GREAT_PERSON_OFFSET;
use crate::content::persistent_events::{
    HandCardsRequest, PaymentRequest, PersistentEventType, PlayerRequest, PositionRequest,
    ResourceRewardRequest, SelectedStructure, StructuresRequest, TriggerPersistentEventParams,
    UnitsRequest, trigger_persistent_event_with_listener,
};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::map::Terrain;
use crate::payment::{PaymentConversion, PaymentConversionType};
use crate::pirates::pirates_spawn_and_raid;
use crate::player::Player;
use crate::player_events::{IncidentInfo, IncidentPlayerInfo, IncidentTarget};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use crate::wonder::Wonder;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub(crate) const BASE_EFFECT_PRIORITY: i32 = 100;

///
/// An incident represents a Game Event that is triggered for every third advance.
/// We use the term incident to differentiate it from the events system to avoid confusion.
#[derive(Clone)]
pub struct Incident {
    pub id: u8,
    pub name: String,
    description: String,
    protection_advance: Option<Advance>,
    protection_special_advance: Option<SpecialAdvance>,
    pub base_effect: IncidentBaseEffect,
    pub listeners: AbilityListeners,
    pub(crate) action_card: Option<ActionCard>,
}

impl Incident {
    #[must_use]
    pub(crate) fn builder(
        id: u8,
        name: &str,
        description: &str,
        base_effect: IncidentBaseEffect,
    ) -> IncidentBuilder {
        IncidentBuilder::new(id, name, description, base_effect)
    }

    #[must_use]
    pub fn description(&self, game: &Game) -> Vec<String> {
        let mut h = vec![];

        if matches!(self.base_effect, IncidentBaseEffect::None) {
            h.push(self.base_effect.to_string());
        }
        if let Some(p) = &self.protection_advance {
            h.push(format!("Protection advance: {}", p.name(game)));
        }
        if let Some(p) = &self.protection_special_advance {
            h.push(format!("Protection advance: {}", p.name(game)));
        }
        h.push(self.description.clone());
        h
    }
}

#[derive(Clone)]
pub enum IncidentBaseEffect {
    None,
    BarbariansSpawn,
    BarbariansMove,
    PiratesSpawnAndRaid,
    ExhaustedLand,
    GoldDeposits,
}

impl std::fmt::Display for IncidentBaseEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncidentBaseEffect::None => write!(f, "No base effect."),
            IncidentBaseEffect::BarbariansSpawn => write!(f, "Barbarians spawn."),
            IncidentBaseEffect::BarbariansMove => write!(f, "Barbarians move."),
            IncidentBaseEffect::PiratesSpawnAndRaid => write!(f, "Pirates spawn."),
            IncidentBaseEffect::ExhaustedLand => write!(f, "Exhausted land."),
            IncidentBaseEffect::GoldDeposits => write!(f, "Gold deposits."),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum PassedIncident {
    NewPlayer(usize),
    AlreadyPassed,
}

#[derive(Clone)]
pub(crate) struct IncidentFilter {
    role: IncidentTarget,
    priority: i32,
    protection_advance: Option<Advance>,
    protection_special_advance: Option<SpecialAdvance>,
}

impl IncidentFilter {
    pub fn new(
        role: IncidentTarget,
        priority: i32,
        protection_advance: Option<Advance>,
        protection_special_advance: Option<SpecialAdvance>,
    ) -> Self {
        Self {
            role,
            priority,
            protection_advance,
            protection_special_advance,
        }
    }

    #[must_use]
    pub fn is_active(&self, game: &Game, i: &IncidentInfo, player: usize) -> bool {
        is_active(
            &self.protection_advance,
            &self.protection_special_advance,
            self.priority,
            game,
            i,
            self.role,
            player,
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) enum MoodModifier {
    Decrease,
    MakeAngry,
}

pub(crate) struct DecreaseMood {
    pub choices: Vec<Position>,
    pub needed: u8,
}

impl DecreaseMood {
    #[must_use]
    pub fn new(possible: Vec<Position>, needed: u8) -> Self {
        Self {
            needed: needed.min(possible.len() as u8),
            choices: possible,
        }
    }

    #[must_use]
    pub fn none() -> Self {
        Self {
            choices: vec![],
            needed: 0,
        }
    }
}

pub(crate) struct IncidentBuilder {
    id: u8,
    pub name: String,
    description: String,
    base_effect: IncidentBaseEffect,
    protection_advance: Option<Advance>,
    protection_special_advance: Option<SpecialAdvance>,
    action_card: Option<ActionCard>,
    builder: AbilityInitializerBuilder,
}

impl IncidentBuilder {
    #[must_use]
    fn new(id: u8, name: &str, description: &str, base_effect: IncidentBaseEffect) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            base_effect,
            builder: AbilityInitializerBuilder::new(),
            protection_advance: None,
            protection_special_advance: None,
            action_card: None,
        }
    }

    #[must_use]
    pub fn build(self) -> Incident {
        Self::new_incident(match self.base_effect {
            IncidentBaseEffect::None => self,
            IncidentBaseEffect::BarbariansSpawn => barbarians_spawn(self),
            IncidentBaseEffect::BarbariansMove => barbarians_move(self),
            IncidentBaseEffect::PiratesSpawnAndRaid => pirates_spawn_and_raid(self),
            IncidentBaseEffect::ExhaustedLand => exhausted_land(self),
            IncidentBaseEffect::GoldDeposits => gold_deposits(self),
        })
    }

    fn new_incident(builder: IncidentBuilder) -> Incident {
        Incident {
            id: builder.id,
            name: builder.name,
            description: builder.description,
            base_effect: builder.base_effect,
            listeners: builder.builder.build(),
            protection_advance: builder.protection_advance,
            protection_special_advance: builder.protection_special_advance,
            action_card: builder.action_card,
        }
    }

    #[must_use]
    pub fn with_protection_advance(mut self, advance: Advance) -> Self {
        self.protection_advance = Some(advance);
        self
    }

    #[must_use]
    pub fn with_protection_special_advance(mut self, advance: SpecialAdvance) -> Self {
        self.protection_special_advance = Some(advance);
        self
    }

    #[must_use]
    pub fn with_action_card(mut self, action_card: ActionCard) -> Self {
        self.action_card = Some(action_card);
        self
    }

    #[must_use]
    pub fn add_simple_incident_listener<F>(
        self,
        role: IncidentTarget,
        priority: i32,
        listener: F,
    ) -> Self
    where
        F: Fn(&mut Game, &EventPlayer, &mut IncidentInfo) + 'static + Clone + Sync + Send,
    {
        let f = self.new_filter(role, priority);
        self.add_simple_persistent_event_listener(
            |event| &mut event.incident,
            priority,
            move |game, p, i| {
                if f.is_active(game, i, p.index) {
                    listener(game, p, i);
                }
            },
        )
    }

    #[must_use]
    pub(crate) fn add_incident_position_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut IncidentInfo) -> Option<PositionRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedMultiChoice<Vec<Position>>, &mut IncidentInfo)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_position_request(
            |event| &mut event.incident,
            priority,
            move |game, p, i| {
                if f.is_active(game, i, p.index) {
                    request(game, p, i)
                } else {
                    None
                }
            },
            move |game, s, i| {
                gain_reward(game, s, i);
            },
        )
    }

    #[must_use]
    pub(crate) fn add_incident_units_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut IncidentInfo) -> Option<UnitsRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedMultiChoice<Vec<u32>>, &mut IncidentInfo)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_units_request(
            |event| &mut event.incident,
            priority,
            move |game, p, i| {
                if f.is_active(game, i, p.index) {
                    request(game, p, i)
                } else {
                    None
                }
            },
            move |game, s, i| {
                gain_reward(game, s, i);
            },
        )
    }

    #[must_use]
    pub(crate) fn add_incident_structures_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &IncidentInfo) -> Option<StructuresRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        structures_selected: impl Fn(
            &mut Game,
            &SelectedMultiChoice<Vec<SelectedStructure>>,
            &mut IncidentInfo,
        )
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_structures_request(
            |event| &mut event.incident,
            priority,
            move |game, p, i| {
                if f.is_active(game, i, p.index) {
                    request(game, p, i)
                } else {
                    None
                }
            },
            move |game, s, i| {
                structures_selected(game, s, i);
            },
        )
    }

    #[must_use]
    pub(crate) fn add_incident_resource_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &IncidentInfo) -> Option<ResourceRewardRequest>
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_resource_request(
            |event| &mut event.incident,
            priority,
            move |game, p, i| {
                if f.is_active(game, i, p.index) {
                    request(game, p, i)
                } else {
                    None
                }
            },
        )
    }

    fn new_filter(&self, role: IncidentTarget, priority: i32) -> IncidentFilter {
        IncidentFilter::new(
            role,
            priority,
            self.protection_advance,
            self.protection_special_advance,
        )
    }

    #[must_use]
    pub(crate) fn add_incident_payment_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut IncidentInfo) -> Option<Vec<PaymentRequest>>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedWithoutChoices<Vec<ResourcePile>>, &mut IncidentInfo)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_payment_request_listener(
            |event| &mut event.incident,
            priority,
            move |game, p, i| {
                if f.is_active(game, i, p.index) {
                    request(game, p, i)
                } else {
                    None
                }
            },
            move |game, s, i| {
                gain_reward(game, s, i);
            },
        )
    }

    #[must_use]
    pub(crate) fn add_incident_hand_card_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &IncidentInfo) -> Option<HandCardsRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        cards_selected: impl Fn(&mut Game, &SelectedMultiChoice<Vec<HandCard>>, &mut IncidentInfo)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_hand_card_request(
            |event| &mut event.incident,
            priority,
            move |game, p, i| {
                if f.is_active(game, i, p.index) {
                    request(game, p, i)
                } else {
                    None
                }
            },
            move |game, s, i| {
                cards_selected(game, s, i);
            },
        )
    }

    #[must_use]
    pub(crate) fn add_incident_player_request(
        self,
        target: IncidentTarget,
        description: &str,
        player_pred: impl Fn(&Player, &Game, &IncidentInfo) -> bool + 'static + Clone + Sync + Send,
        priority: i32,
        gain_reward: impl Fn(&mut Game, &SelectedSingleChoice<usize>, &mut IncidentInfo)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self {
        let f = self.new_filter(target, priority);
        let d = description.to_string();
        self.add_player_request(
            |event| &mut event.incident,
            priority,
            move |game, player, i| {
                if f.is_active(game, i, player.index) {
                    let choices = game
                        .players
                        .iter()
                        .filter(|p| {
                            p.is_human() && p.index != player.index && player_pred(p, game, i)
                        })
                        .map(|p| p.index)
                        .collect_vec();

                    if choices.is_empty() {
                        None
                    } else {
                        Some(PlayerRequest::new(choices, &d))
                    }
                } else {
                    None
                }
            },
            move |game, s, i| {
                gain_reward(game, s, i);
            },
        )
    }

    pub(crate) fn add_decrease_mood(
        self,
        target: IncidentTarget,
        mood_modifier: MoodModifier,
        cities: impl Fn(&Player, &Game, &IncidentInfo) -> DecreaseMood + 'static + Clone + Sync + Send,
    ) -> Self {
        let cities2 = cities.clone();
        self.add_myths_payment(target, mood_modifier, move |g, p, i| cities(p, g, i).needed)
            .decrease_mood(target, mood_modifier, cities2)
    }

    fn add_myths_payment(
        self,
        target: IncidentTarget,
        mood_modifier: MoodModifier,
        amount: impl Fn(&Game, &Player, &IncidentInfo) -> u8 + 'static + Clone + Sync + Send,
    ) -> Self {
        self.add_incident_payment_request(
            target,
            10,
            move |game, player, i| {
                let p = player.get(game);
                if p.can_use_advance(Advance::Myths) {
                    let needed = amount(game, p, i);
                    if needed == 0 {
                        return None;
                    }
                    let mut options = player
                        .with_origin(EventOrigin::Advance(Advance::Myths))
                        .payment_options()
                        .sum(p, needed, &[ResourceType::MoodTokens]);
                    options
                        .conversions
                        .push(PaymentConversion::resource_options(
                            vec![ResourcePile::mood_tokens(1)],
                            ResourcePile::empty(),
                            PaymentConversionType::MayOverpay(needed),
                        ));

                    let action = match mood_modifier {
                        MoodModifier::Decrease => "reducing the mood",
                        MoodModifier::MakeAngry => "making it Angry",
                    };

                    // mandatory - but may be 0
                    Some(vec![PaymentRequest::mandatory(
                        options,
                        &format!("You may pay 1 mood token for each city to avoid {action}"),
                    )])
                } else {
                    None
                }
            },
            move |game, s, i| {
                let pile = &s.choice[0];
                i.player.myths_payment = pile.amount();
                s.player()
                    .with_origin(EventOrigin::Advance(Advance::Myths))
                    .log(game, "Avoid mood change");
            },
        )
    }

    fn decrease_mood(
        self,
        target: IncidentTarget,
        mood_modifier: MoodModifier,
        cities: impl Fn(&Player, &Game, &IncidentInfo) -> DecreaseMood + 'static + Clone + Sync + Send,
    ) -> Self {
        self.add_incident_position_request(
            target,
            9,
            move |game, p, i| {
                let d = cities(p.get(game), game, i);
                let mut needed = d.needed;
                needed -= i.player.myths_payment;

                let action = match mood_modifier {
                    MoodModifier::Decrease => "decrease the mood",
                    MoodModifier::MakeAngry => "make Angry",
                };

                let needed1 = needed..=needed;
                let description = &format!("Select a city to {action}");
                Some(PositionRequest::new(d.choices, needed1, description))
            },
            move |game, s, _| {
                decrease_mod_and_log(game, s, mood_modifier);
            },
        )
    }
}

impl AbilityInitializerSetup for IncidentBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Incident(self.id)
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

pub(crate) fn trigger_incident(game: &mut Game, player_index: usize) {
    if game
        .player(player_index)
        .wonders_owned
        .contains(Wonder::GreatMausoleum)
    {
        on_choose_incident(game, player_index, IncidentInfo::new(0, player_index));
    } else {
        let info = IncidentInfo::new(
            draw_and_discard_incident_card_from_pile(game, player_index),
            player_index,
        );
        on_trigger_incident(game, info);
    }
}

pub(crate) fn on_choose_incident(game: &mut Game, player_index: usize, info: IncidentInfo) {
    if let Some(info) = game.trigger_persistent_event(
        &[player_index],
        |events| &mut events.choose_incident,
        info,
        PersistentEventType::ChooseIncident,
    ) {
        on_trigger_incident(game, info);
    }
}

pub(crate) fn on_trigger_incident(game: &mut Game, mut info: IncidentInfo) {
    loop {
        let log: Option<String> = play_base_effect(&info).then_some(format!(
            "A new game event has been triggered: {}",
            game.cache.get_incident(info.incident_id).name
        ));
        info = match trigger_persistent_event_with_listener(
            game,
            &game.human_players_sorted(info.active_player),
            |events| &mut events.incident,
            &game.cache.get_incident(info.incident_id).listeners.clone(),
            info,
            PersistentEventType::Incident,
            TriggerPersistentEventParams {
                log,
                next_player: |i| i.player = IncidentPlayerInfo::new(),
                ..Default::default()
            },
        ) {
            Some(p) => p,
            None => return,
        };

        if !game
            .events
            .iter()
            .any(|e| matches!(e.event_type, PersistentEventType::Incident(_)))
        {
            if passed_to_player(game, &mut info) {
                continue;
            }
            break;
        }
    }
}

pub(crate) fn draw_and_discard_incident_card_from_pile(game: &mut Game, player: usize) -> u8 {
    let id = draw_card_from_pile(
        game,
        "Events",
        |g| &mut g.incidents_left,
        |g| g.cache.get_incidents().iter().map(|i| i.id).collect_vec(),
        |p| {
            p.action_cards
                .iter()
                .filter_map(|a| {
                    if *a >= GREAT_PERSON_OFFSET {
                        Some(a - GREAT_PERSON_OFFSET)
                    } else {
                        None
                    }
                })
                .collect()
        },
    )
    .expect("incident should exist");

    discard_card(|g| &mut g.incidents_discarded, id, player, game);
    id
}

pub(crate) fn play_base_effect(i: &IncidentInfo) -> bool {
    i.passed.is_none()
}

fn passed_to_player(game: &mut Game, i: &mut IncidentInfo) -> bool {
    if let Some(PassedIncident::NewPlayer(p)) = i.passed {
        game.log(
            i.active_player,
            &i.origin(),
            &format!("Pass the event to {}", game.player_name(p)),
        );
        i.passed = Some(PassedIncident::AlreadyPassed);
        i.active_player = p;
        return true;
    }
    false
}

#[must_use]
pub fn is_active(
    protection_advance: &Option<Advance>,
    protection_special_advance: &Option<SpecialAdvance>,
    priority: i32,
    game: &Game,
    i: &IncidentInfo,
    role: IncidentTarget,
    player: usize,
) -> bool {
    if !i.is_active_ignoring_protection(role, player) {
        return false;
    }
    if priority >= BASE_EFFECT_PRIORITY {
        return play_base_effect(i);
    }
    // protection advance does not protect against base effects
    if let Some(advance) = protection_advance {
        if game.player(player).can_use_advance(*advance) {
            return false;
        }
    }
    if let Some(advance) = protection_special_advance {
        if game.player(player).has_special_advance(*advance) {
            return false;
        }
    }
    true
}

#[must_use]
fn exhausted_land(builder: IncidentBuilder) -> IncidentBuilder {
    builder.add_incident_position_request(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY,
        |game, p, _incident| {
            let p = game.player(p.index);
            let positions = p
                .cities
                .iter()
                .flat_map(|c| c.position.neighbors())
                .filter(|pos| {
                    game.try_get_any_city(*pos).is_none()
                        && !enemy_units_present(game, *pos, p.index)
                        && game.map.get(*pos).is_some_and(is_valid_city_terrain)
                })
                .collect_vec();
            let needed = 1..=1;
            Some(PositionRequest::new(
                positions,
                needed,
                "Select a land position to exhaust",
            ))
        },
        |game, s, _| {
            let pos = s.choice[0];
            s.log(game, &format!("Exhausted the land in position {pos}",));
            let t = game.map.tiles.get_mut(&pos).expect("tile should exist");
            *t = Terrain::Exhausted(Box::new(t.clone()));
        },
    )
}

fn gold_deposits(b: IncidentBuilder) -> IncidentBuilder {
    b.add_simple_incident_listener(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY,
        |game, p, _incident| {
            p.with_origin(EventOrigin::Ability("Gold deposits".to_string()))
                .gain_resources(game, ResourcePile::gold(2));
        },
    )
}

pub(crate) fn decrease_mod_and_log(
    game: &mut Game,
    s: &SelectedMultiChoice<Vec<Position>>,
    mood_modifier: MoodModifier,
) {
    for &pos in &s.choice {
        match mood_modifier {
            MoodModifier::Decrease => decrease_city_mood(game, pos, &s.origin),
            MoodModifier::MakeAngry => set_city_mood(game, pos, &s.origin, MoodState::Angry),
        }
    }
}

fn enemy_units_present(game: &Game, pos: Position, player: usize) -> bool {
    game.players
        .iter()
        .any(|p| player != p.index && p.units.iter().any(|u| u.position == pos))
}
