use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::ability_initializer::{AbilityInitializerSetup, SelectedChoice};
use crate::action_card::ActionCard;
use crate::barbarians::{barbarians_move, barbarians_spawn};
use crate::card::{draw_card_from_pile, HandCard};
use crate::city::MoodState;
use crate::content::custom_phase_actions::{
    new_position_request, CurrentEventType, HandCardsRequest, PaymentRequest, PlayerRequest,
    PositionRequest, ResourceRewardRequest, SelectedStructure, StructuresRequest, UnitTypeRequest,
    UnitsRequest,
};
use crate::content::incidents;
use crate::content::incidents::great_diplomat::DiplomaticRelations;
use crate::content::incidents::great_persons::GREAT_PERSON_OFFSET;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::map::Terrain;
use crate::payment::{PaymentConversion, PaymentConversionType, PaymentOptions};
use crate::pirates::pirates_spawn_and_raid;
use crate::player::Player;
use crate::player_events::{IncidentInfo, IncidentPlayerInfo, IncidentTarget};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub(crate) const BASE_EFFECT_PRIORITY: i32 = 100;

///
/// An incident represents a Game Event that is triggerd for every third advance.
/// We use the term incident to differentiate it from the events system to avoid confusion.
pub struct Incident {
    pub id: u8,
    pub name: String,
    description: String,
    protection_advance: Option<String>,
    pub base_effect: IncidentBaseEffect,
    pub listeners: AbilityListeners,
    pub(crate) action_card: Option<ActionCard>,
}

impl Incident {
    #[must_use]
    pub fn builder(
        id: u8,
        name: &str,
        description: &str,
        base_effect: IncidentBaseEffect,
    ) -> IncidentBuilder {
        IncidentBuilder::new(id, name, description, base_effect)
    }

    #[must_use]
    pub fn description(&self) -> Vec<String> {
        let mut h = vec![];

        if matches!(self.base_effect, IncidentBaseEffect::None) {
            h.push(self.base_effect.to_string());
        }
        if let Some(p) = &self.protection_advance {
            h.push(format!("Protection advance: {p}"));
        }
        h.push(self.description.clone());
        h
    }
}

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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Anarchy {
    pub player: usize,
    pub advances_lost: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum PassedIncident {
    NewPlayer(usize),
    AlreadyPassed,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum PermanentIncidentEffect {
    Pestilence,
    LoseAction(usize),
    PublicWonderCard(String),
    TrojanHorse,
    SolarEclipse,
    Anarchy(Anarchy),
    GreatEngineer,
    DiplomaticRelations(DiplomaticRelations),
}

#[derive(Clone)]
pub(crate) struct IncidentFilter {
    role: IncidentTarget,
    priority: i32,
    protection_advance: Option<String>,
}

impl IncidentFilter {
    pub fn new(role: IncidentTarget, priority: i32, protection_advance: Option<String>) -> Self {
        Self {
            role,
            priority,
            protection_advance,
        }
    }

    #[must_use]
    pub fn is_active(&self, game: &Game, i: &IncidentInfo, player: usize) -> bool {
        is_active(
            &self.protection_advance,
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

pub struct IncidentBuilder {
    id: u8,
    pub name: String,
    description: String,
    base_effect: IncidentBaseEffect,
    protection_advance: Option<String>,
    action_card: Option<ActionCard>,
    associated_permanent_effect: Option<PermanentIncidentEffect>,
    builder: AbilityInitializerBuilder,
}

impl IncidentBuilder {
    fn new(id: u8, name: &str, description: &str, base_effect: IncidentBaseEffect) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            base_effect,
            builder: AbilityInitializerBuilder::new(),
            protection_advance: None,
            action_card: None,
            associated_permanent_effect: None,
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
            action_card: builder.action_card,
        }
    }

    #[must_use]
    pub fn with_protection_advance(mut self, advance: &str) -> Self {
        self.protection_advance = Some(advance.to_string());
        self
    }

    #[must_use]
    pub fn with_action_card(mut self, action_card: ActionCard) -> Self {
        self.action_card = Some(action_card);
        self
    }
    
    #[must_use]
    pub fn with_associated_permanent_effect(mut self, effect: PermanentIncidentEffect) -> Self {
        self.associated_permanent_effect = Some(effect);
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
        F: Fn(&mut Game, usize, &str, &mut IncidentInfo) + 'static + Clone,
    {
        self.add_simple_persistent_event_listener(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, player_name, i| {
                if i.is_active(role, player_index) {
                    listener(game, player_index, player_name, i);
                }
            },
        )
    }

    #[must_use]
    pub(crate) fn add_incident_position_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &mut IncidentInfo) -> Option<PositionRequest>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<Position>>, &mut IncidentInfo)
            + 'static
            + Clone,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_position_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    request(game, player_index, i)
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
    pub(crate) fn add_incident_unit_type_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &mut IncidentInfo) -> Option<UnitTypeRequest>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<UnitType>, &mut IncidentInfo) + 'static + Clone,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_unit_type_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    request(game, player_index, i)
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
        request: impl Fn(&mut Game, usize, &mut IncidentInfo) -> Option<UnitsRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<u32>>, &mut IncidentInfo) + 'static + Clone,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_units_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    request(game, player_index, i)
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
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<StructuresRequest> + 'static + Clone,
        structures_selected: impl Fn(&mut Game, &SelectedChoice<Vec<SelectedStructure>>, &mut IncidentInfo)
            + 'static
            + Clone,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_structures_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    request(game, player_index, i)
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
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<ResourceRewardRequest>
            + 'static
            + Clone,
        gain_reward_log: impl Fn(&Game, &SelectedChoice<ResourcePile>) -> Vec<String> + 'static + Clone,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_resource_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    request(game, player_index, i)
                } else {
                    None
                }
            },
            move |game, s, _| gain_reward_log(game, s),
        )
    }

    fn new_filter(&self, role: IncidentTarget, priority: i32) -> IncidentFilter {
        IncidentFilter::new(role, priority, self.protection_advance.clone())
    }

    #[must_use]
    pub(crate) fn add_incident_payment_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &mut IncidentInfo) -> Option<Vec<PaymentRequest>>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<ResourcePile>>, &mut IncidentInfo)
            + 'static
            + Clone,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_payment_request_listener(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    request(game, player_index, i)
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
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<HandCardsRequest> + 'static + Clone,
        cards_selected: impl Fn(&mut Game, &SelectedChoice<Vec<HandCard>>, &mut IncidentInfo)
            + 'static
            + Clone,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_hand_card_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    request(game, player_index, i)
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
        player_pred: impl Fn(&Player, &Game, &IncidentInfo) -> bool + 'static + Clone,
        priority: i32,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<usize>, &mut IncidentInfo) + 'static + Clone,
    ) -> Self {
        let f = self.new_filter(target, priority);
        let d = description.to_string();
        self.add_player_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    let choices = game
                        .players
                        .iter()
                        .filter(|p| {
                            p.is_human() && p.index != player_index && player_pred(p, game, i)
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
        cities: impl Fn(&Player, &Game, &IncidentInfo) -> (Vec<Position>, u8) + 'static + Clone,
    ) -> Self {
        let cities2 = cities.clone();
        self.add_myths_payment(target, mood_modifier, move |g, p, i| {
            cities(p, g, i).1 as u32
        })
        .decrease_mood(target, mood_modifier, cities2)
    }

    fn add_myths_payment(
        self,
        target: IncidentTarget,
        mood_modifier: MoodModifier,
        amount: impl Fn(&Game, &Player, &IncidentInfo) -> u32 + 'static + Clone,
    ) -> Self {
        self.add_incident_payment_request(
            target,
            10,
            move |game, player_index, i| {
                let p = game.get_player(player_index);
                if p.has_advance("Myths") {
                    let needed = amount(game, p, i);
                    if needed == 0 {
                        return None;
                    }
                    let mut options = PaymentOptions::sum(needed, &[ResourceType::MoodTokens]);
                    options.conversions.push(PaymentConversion::new(
                        vec![ResourcePile::mood_tokens(1)],
                        ResourcePile::empty(),
                        PaymentConversionType::MayOverpay(needed),
                    ));

                    let action = match mood_modifier {
                        MoodModifier::Decrease => "reducing the mood",
                        MoodModifier::MakeAngry => "making it Angry",
                    };

                    Some(vec![PaymentRequest::new(
                        options,
                        &format!("You may pay 1 mood token for each city to avoid {action}"),
                        false,
                    )])
                } else {
                    None
                }
            },
            move |game, s, i| {
                let pile = &s.choice[0];
                i.player.myths_payment = pile.amount() as u8;
                game.add_info_log_item(&format!(
                    "{} paid {pile} to avoid the mood change using Myths",
                    s.player_name
                ));
            },
        )
    }

    fn decrease_mood(
        self,
        target: IncidentTarget,
        mood_modifier: MoodModifier,
        cities: impl Fn(&Player, &Game, &IncidentInfo) -> (Vec<Position>, u8) + 'static + Clone,
    ) -> Self {
        self.add_incident_position_request(
            target,
            9,
            move |game, p, i| {
                let (cities, mut needed) = cities(game.get_player(p), game, i);
                needed -= i.player.myths_payment;

                let action = match mood_modifier {
                    MoodModifier::Decrease => "decrease the mood",
                    MoodModifier::MakeAngry => "make Angry",
                };

                Some(new_position_request(
                    cities,
                    needed..=needed,
                    &format!("Select a city to {action}"),
                ))
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
}

pub(crate) fn trigger_incident(game: &mut Game, mut info: IncidentInfo) {
    let incident = incidents::get_incident(
        draw_card_from_pile(
            game,
            "Events",
            true,
            |g| &mut g.incidents_left,
            || incidents::get_all().iter().map(|i| i.id).collect_vec(),
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
        .expect("incident should exist"),
    );

    loop {
        let log: Option<String> = play_base_effect(&info).then_some(format!(
            "A new game event has been triggered: {}",
            incident.name
        ));
        info = match game.trigger_current_event_with_listener(
            &game.human_players(info.active_player),
            |events| &mut events.on_incident,
            &incident.listeners,
            info,
            CurrentEventType::Incident,
            log.as_deref(),
            |i| i.player = IncidentPlayerInfo::new(),
        ) {
            Some(p) => p,
            None => return,
        };

        if !game
            .events
            .iter()
            .any(|e| matches!(e.event_type, CurrentEventType::Incident(_)))
        {
            if passed_to_player(game, &mut info) {
                continue;
            }
            break;
        }
    }

    game.incidents_left.remove(0);
}

pub(crate) fn play_base_effect(i: &IncidentInfo) -> bool {
    i.passed.is_none()
}

fn passed_to_player(game: &mut Game, i: &mut IncidentInfo) -> bool {
    if let Some(PassedIncident::NewPlayer(p)) = i.passed {
        game.add_info_log_item(&format!(
            "Player {} has passed the incident to {}",
            game.player_name(i.active_player),
            game.player_name(p)
        ));
        i.passed = Some(PassedIncident::AlreadyPassed);
        i.active_player = p;
        return true;
    }
    false
}

#[must_use]
pub fn is_active(
    protection_advance: &Option<String>,
    priority: i32,
    game: &Game,
    i: &IncidentInfo,
    role: IncidentTarget,
    player: usize,
) -> bool {
    if !i.is_active(role, player) {
        return false;
    }
    if priority >= BASE_EFFECT_PRIORITY {
        return play_base_effect(i);
    }
    // protection advance does not protect against base effects
    if let Some(advance) = &protection_advance {
        if game.players[player].has_advance(advance) {
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
        |game, player_index, _incident| {
            let p = game.get_player(player_index);
            let positions = p
                .cities
                .iter()
                .flat_map(|c| c.position.neighbors())
                .filter(|p| {
                    game.try_get_any_city(*p).is_none()
                        && !enemy_units_present(game, *p, player_index)
                        && game.map.get(*p).is_some_and(|t| {
                            t.is_land() && !matches!(t, Terrain::Exhausted(_) | Terrain::Barren)
                        })
                })
                .collect_vec();
            Some(new_position_request(
                positions,
                1..=1,
                "Select a land position to exhaust",
            ))
        },
        |game, s, _| {
            let pos = s.choice[0];
            game.add_info_log_item(&format!(
                "{} exhausted the land in position {}",
                s.player_name, pos
            ));
            let t = game.map.tiles.get_mut(&pos).expect("tile should exist");
            *t = Terrain::Exhausted(Box::new(t.clone()));
        },
    )
}

fn gold_deposits(b: IncidentBuilder) -> IncidentBuilder {
    b.add_incident_resource_request(
        IncidentTarget::ActivePlayer,
        BASE_EFFECT_PRIORITY,
        |_game, _player_index, _incident| {
            Some(ResourceRewardRequest::new(
                PaymentOptions::sum(2, &[ResourceType::Gold]),
                "-".to_string(),
            ))
        },
        |_game, s| {
            vec![format!(
                "{} gained {} from a Gold Mine",
                s.player_name, s.choice
            )]
        },
    )
}

pub(crate) fn decrease_mod_and_log(
    game: &mut Game,
    s: &SelectedChoice<Vec<Position>>,
    mood_modifier: MoodModifier,
) {
    for &pos in &s.choice {
        let name = &s.player_name;
        match mood_modifier {
            MoodModifier::Decrease => {
                game.get_player_mut(s.player_index)
                    .get_city_mut(pos)
                    .decrease_mood_state();
                let mood_state = &game.get_player(s.player_index).get_city(pos).mood_state;
                if s.actively_selected {
                    game.add_info_log_item(&format!(
                        "{name} selected to decrease the mood in city {pos} to {mood_state:?}",
                    ));
                } else {
                    game.add_info_log_item(&format!(
                        "{name} decreased the mood in city {pos} to {mood_state:?}",
                    ));
                }
            }
            MoodModifier::MakeAngry => {
                game.add_info_log_item(&format!("{name} made city {pos} Angry"));
                game.get_player_mut(s.player_index)
                    .get_city_mut(pos)
                    .mood_state = MoodState::Angry;
            }
        }
    }
}

fn enemy_units_present(game: &Game, pos: Position, player: usize) -> bool {
    game.players
        .iter()
        .any(|p| player != p.index && p.units.iter().any(|u| u.position == pos))
}
