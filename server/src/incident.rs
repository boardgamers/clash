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

pub(crate) const BASE_EFFECT_PRIORITY: i32 = 100;

///
/// An incident represents a Game Event that is triggerd for every third advance.
/// We use the term incident to differentiate it from the events system to avoid confusion.
pub struct Incident {
    pub id: u8,
    pub name: String,
    description: String,
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
    pub fn description(&self) -> String {
        format!("{}. {}", self.base_effect, self.description)
    }
}

pub enum IncidentBaseEffect {
    BarbariansSpawn,
    BarbariansMove,
    PiratesSpawnAndRaid,
}

impl std::fmt::Display for IncidentBaseEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncidentBaseEffect::BarbariansSpawn => write!(f, "Barbarians spawn."),
            IncidentBaseEffect::BarbariansMove => write!(f, "Barbarians move."),
            IncidentBaseEffect::PiratesSpawnAndRaid => write!(f, "Pirates spawn."),
        }
    }
}

pub struct IncidentBuilder {
    id: u8,
    name: String,
    description: String,
    base_effect: IncidentBaseEffect,
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
        }
    }

    #[must_use]
    pub fn build(self) -> Incident {
        Self::new_incident(match self.base_effect {
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
        }
    }

    #[must_use]
    pub fn add_incident_listener<F>(self, role: IncidentTarget, priority: i32, listener: F) -> Self
    where
        F: Fn(&mut Game, &CustomPhaseInfo, &IncidentInfo) + 'static + Clone,
    {
        self.add_player_event_listener(
            |event| &mut event.on_incident,
            move |game, p, i| {
                if i.is_active(role, p.player) {
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
        self.add_position_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if i.is_active(role, player_index) {
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
        self.add_unit_type_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if i.is_active(role, player_index) {
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
        self.add_resource_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if i.is_active(role, player_index) {
                    request(game, player_index, i)
                } else {
                    None
                }
            },
            gain_reward_log,
        )
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
        self.add_payment_request_listener(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if i.is_active(role, player_index) {
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
        self.add_player_request(
            |event| &mut event.on_incident,
            priority,
            move |game, player_index, i| {
                if i.is_active(role, player_index) {
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
    for p in &game.human_players() {
        (incidents::get_incident(id).listeners.initializer)(game, *p);
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
        Some("A new game event has been triggered: "),
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
