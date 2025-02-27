use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::ability_initializer::{AbilityInitializerSetup, SelectedChoice};
use crate::barbarians::{barbarians_move, barbarians_spawn};
use crate::content::custom_phase_actions::{
    PaymentRequest, PlayerRequest, PositionRequest, ResourceRewardRequest, UnitTypeRequest,
};
use crate::content::incidents;
use crate::events::EventOrigin;
use crate::game::{Game, GameState};
use crate::pirates::pirates_spawn_and_raid;
use crate::player_events::{CustomPhaseInfo, IncidentInfo, IncidentTarget};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::status_phase::StatusPhaseAction;
use crate::unit::UnitType;
use crate::utils::Shuffle;
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
}

impl std::fmt::Display for IncidentBaseEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncidentBaseEffect::None => write!(f, "No base effect."),
            IncidentBaseEffect::BarbariansSpawn => write!(f, "Barbarians spawn."),
            IncidentBaseEffect::BarbariansMove => write!(f, "Barbarians move."),
            IncidentBaseEffect::PiratesSpawnAndRaid => write!(f, "Pirates spawn."),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum PermanentIncidentEffect {
    Pestilence,
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

pub struct IncidentBuilder {
    id: u8,
    name: String,
    description: String,
    base_effect: IncidentBaseEffect,
    protection_advance: Option<String>,
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
        }
    }

    #[must_use]
    pub fn build(self) -> Incident {
        Self::new_incident(match self.base_effect {
            IncidentBaseEffect::None => self,
            IncidentBaseEffect::BarbariansSpawn => barbarians_spawn(self),
            IncidentBaseEffect::BarbariansMove => barbarians_move(self),
            IncidentBaseEffect::PiratesSpawnAndRaid => pirates_spawn_and_raid(self),
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
        }
    }

    #[must_use]
    pub fn set_protection_advance(mut self, advance: &str) -> Self {
        self.protection_advance = Some(advance.to_string());
        self
    }

    #[must_use]
    pub fn add_incident_listener<F>(self, role: IncidentTarget, priority: i32, listener: F) -> Self
    where
        F: Fn(&mut Game, &CustomPhaseInfo, &IncidentInfo) + 'static + Clone,
    {
        let f = self.new_filter(role, priority);
        self.add_player_event_listener(
            |event| &mut event.on_incident,
            move |game, p, i| {
                if f.is_active(game, i, p.player) {
                    listener(game, p, i);
                }
            },
            priority,
        )
    }

    #[must_use]
    pub(crate) fn add_incident_position_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<PositionRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Position>) + 'static + Clone,
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
            gain_reward,
        )
    }

    #[must_use]
    pub(crate) fn add_incident_unit_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<UnitTypeRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<UnitType>) + 'static + Clone,
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
            gain_reward,
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
            gain_reward_log,
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
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<Vec<PaymentRequest>>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<ResourcePile>>) + 'static + Clone,
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
            gain_reward,
        )
    }

    #[must_use]
    pub(crate) fn add_incident_player_request(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<PlayerRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<usize>) + 'static + Clone,
    ) -> Self {
        let f = self.new_filter(role, priority);
        self.add_player_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if f.is_active(game, i, player_index) {
                    request(game, player_index, i)
                } else {
                    None
                }
            },
            gain_reward,
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

pub(crate) fn trigger_incident(game: &mut Game, player_index: usize) {
    game.lock_undo();

    if game.incidents_left.is_empty() {
        game.incidents_left = incidents::get_all().iter().map(|i| i.id).collect_vec();
        game.incidents_left.shuffle(&mut game.rng);
    }

    let id = *game.incidents_left.first().expect("incident should exist");
    let incident = incidents::get_incident(id);
    for p in &game.human_players() {
        (incident.listeners.initializer)(game, *p);
    }

    let i = game
        .human_players()
        .iter()
        .position(|&p| p == player_index)
        .expect("player should exist");
    let mut players: Vec<_> = game.human_players();
    players.rotate_left(i);

    game.trigger_custom_phase_event(
        &players,
        |events| &mut events.on_incident,
        &IncidentInfo::new(player_index),
        Some(&format!(
            "A new game event has been triggered: {}",
            incident.name
        )),
    );

    for p in &players {
        (incidents::get_incident(id).listeners.deinitializer)(game, *p);
    }

    if game.custom_phase_state.is_empty() {
        game.incidents_left.remove(0);

        if matches!(game.state, GameState::StatusPhase(_)) {
            StatusPhaseAction::action_done(game);
        }
    }
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
        // protection advance does not protect against base effects
        return true;
    }
    if let Some(advance) = &protection_advance {
        if game.players[player].has_advance(advance) {
            return false;
        }
    }
    true
}
