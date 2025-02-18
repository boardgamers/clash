use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::barbarians::barbarians_spawn;
use crate::content::custom_phase_actions::{
    CustomPhasePositionRequest, CustomPhaseResourceRewardRequest, CustomPhaseUnitRequest,
};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::player_events::{CustomPhaseInfo, IncidentInfo, IncidentTarget, PlayerCommands};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;

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
}

impl std::fmt::Display for IncidentBaseEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncidentBaseEffect::BarbariansSpawn => write!(f, "Barbarians spawn."),
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
    pub fn add_incident_listener<F>(self, role: IncidentTarget, listener: F) -> Self
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
            1,
        )
    }

    #[must_use]
    pub(crate) fn add_incident_position_listener(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<CustomPhasePositionRequest>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut PlayerCommands, &Game, &Position) + 'static + Clone,
    ) -> Self {
        self.add_position_reward_request_listener(
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
    pub(crate) fn add_incident_unit_listener(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<CustomPhaseUnitRequest>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut PlayerCommands, &Game, &UnitType) + 'static + Clone,
    ) -> Self {
        self.add_unit_reward_request_listener(
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
    pub(crate) fn add_incident_resource_listener(
        self,
        role: IncidentTarget,
        priority: i32,
        request: impl Fn(&mut Game, usize, &IncidentInfo) -> Option<CustomPhaseResourceRewardRequest>
            + 'static
            + Clone,
        gain_reward_log: impl Fn(&Game, usize, &str, &ResourcePile, bool) -> String + 'static + Clone,
    ) -> Self {
        self.add_resource_reward_request_listener(
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
}

impl AbilityInitializerSetup for IncidentBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Incident(self.id)
    }
}
