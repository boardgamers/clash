use crate::advance::Advance;
use crate::game::Game;
use crate::payment::{PaymentOptionsBuilder, RewardBuilder};
use crate::player::{CostTrigger, Player};
use crate::resource::{gain_resources, lose_resources};
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use crate::wonder::Wonder;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum EventOrigin {
    Advance(Advance),
    SpecialAdvance(SpecialAdvance),
    LeaderAbility(String),
    Wonder(Wonder),
    Ability(String),
    Incident(u8),
    CivilCard(u8),
    TacticsCard(u8),
    Objective(String),
}

impl EventOrigin {
    #[must_use]
    pub fn id(&self) -> String {
        match self {
            // can't call to_string, because cache is not constructed
            EventOrigin::Wonder(name) => format!("{name:?}"),
            EventOrigin::Advance(name) => format!("{name:?}"),
            EventOrigin::SpecialAdvance(name) => format!("{name:?}"),
            EventOrigin::LeaderAbility(name)
            | EventOrigin::Objective(name)
            | EventOrigin::Ability(name) => name.to_string(),
            EventOrigin::CivilCard(id)
            | EventOrigin::TacticsCard(id)
            | EventOrigin::Incident(id) => id.to_string(),
        }
    }

    #[must_use]
    pub fn name(&self, game: &Game) -> String {
        let cache = &game.cache;
        match self {
            EventOrigin::Advance(name) => name.name(game).to_string(),
            EventOrigin::SpecialAdvance(name) => name.name(game).to_string(),
            EventOrigin::Wonder(name) => name.name().to_string(),
            EventOrigin::LeaderAbility(name)
            | EventOrigin::Objective(name)
            | EventOrigin::Ability(name) => name.to_string(),
            EventOrigin::CivilCard(id) => cache.get_civil_card(*id).name.clone(),
            EventOrigin::TacticsCard(id) => cache.get_tactics_card(*id).name.clone(),
            EventOrigin::Incident(id) => cache.get_incident(*id).name.clone(),
        }
    }
}

// to check if a payment option is affordable
#[must_use]
pub fn check_event_origin() -> EventOrigin {
    EventOrigin::Ability("only for checking".to_string())
}

pub(crate) struct EventPlayer {
    pub index: usize,
    pub name: String,
    pub origin: EventOrigin,
}

impl EventPlayer {
    #[must_use]
    pub fn new(player_index: usize, player_name: String, origin: EventOrigin) -> Self {
        Self {
            index: player_index,
            name: player_name,
            origin,
        }
    }

    #[must_use]
    pub fn get<'a>(&self, game: &'a Game) -> &'a Player {
        game.player(self.index)
    }

    #[must_use]
    pub fn get_mut<'a>(&self, game: &'a mut Game) -> &'a mut Player {
        game.player_mut(self.index)
    }

    pub fn gain_resources(&self, game: &mut Game, resources: ResourcePile) {
        gain_resources(game, self.index, resources, self.origin.clone());
    }

    pub fn lose_resources(&self, game: &mut Game, resources: ResourcePile) {
        lose_resources(game, self.index, resources, self.origin.clone());
    }

    #[must_use]
    pub fn with_origin(&self, origin: EventOrigin) -> Self {
        Self {
            index: self.index,
            name: self.name.clone(),
            origin,
        }
    }

    #[must_use]
    pub fn payment_options(&self) -> PaymentOptionsBuilder {
        PaymentOptionsBuilder::new(self.origin.clone())
    }

    #[must_use]
    pub fn reward_options(&self) -> RewardBuilder {
        RewardBuilder::new(self.origin.clone())
    }

    pub fn log(&self, game: &mut Game, message: &str) {
        game.log_with_origin(self.index, &self.origin, message);
    }
}

impl Display for EventPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

struct Listener<T, U, V, W> {
    #[allow(clippy::type_complexity)]
    callback: Arc<dyn Fn(&mut T, &U, &V, &mut W, &EventPlayer) + Sync + Send>,
    priority: i32,
    player: EventPlayer,
}

impl<T, U, V, W> Listener<T, U, V, W> {
    fn new<F>(callback: F, priority: i32, player: EventPlayer) -> Self
    where
        F: Fn(&mut T, &U, &V, &mut W, &EventPlayer) + 'static + Sync + Send,
    {
        Self {
            callback: Arc::new(callback),
            priority,
            player,
        }
    }
}

pub struct EventMut<T, U = (), V = (), W = ()> {
    name: String, // for debugging
    listeners: Vec<Listener<T, U, V, W>>,
}

impl<T, U, V, W> EventMut<T, U, V, W>
where
    T: Clone + PartialEq,
    W: Clone + PartialEq,
{
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            listeners: Vec::new(),
        }
    }

    //return the id of the listener witch can be used to remove the listener later
    pub(crate) fn add_listener_mut<F>(
        &mut self,
        new_listener: F,
        priority: i32,
        player: EventPlayer,
    ) where
        F: Fn(&mut T, &U, &V, &mut W, &EventPlayer) + 'static + Sync + Send,
    {
        // objectives can have the same key, but you still can only fulfill one of them at a time
        let key = &player.origin;
        if let Some(old) = self
            .listeners
            .iter()
            .find(|l| priority == l.priority && &l.player.origin != key)
        {
            panic!(
                "Event {}: Priority {priority} already used by listener with key {:?} when adding {key:?}",
                self.name, old.player.origin
            )
        }
        self.listeners
            .push(Listener::new(new_listener, priority, player));

        self.listeners.sort_by_key(|l| l.priority);
        self.listeners.reverse();
    }

    pub(crate) fn remove_listener_mut_by_key(&mut self, key: &EventOrigin) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|l| &l.player.origin == key)
                .unwrap_or_else(|| panic!("Listeners should include the key {key:?} to remove")),
        );
    }

    #[must_use]
    pub(crate) fn trigger_with_modifiers(
        &self,
        value: &mut T,
        info: &U,
        details: &V,
        extra_value: &mut W,
        trigger: CostTrigger,
    ) -> Vec<EventOrigin> {
        if trigger == CostTrigger::WithModifiers {
            let mut modifiers = Vec::new();
            for l in &self.listeners {
                let previous_value = value.clone();
                let previous_extra_value = extra_value.clone();
                (l.callback)(value, info, details, extra_value, &l.player);
                if *value != previous_value || *extra_value != previous_extra_value {
                    modifiers.push(l.player.origin.clone());
                }
            }
            modifiers
        } else {
            self.trigger(value, info, details, extra_value);
            vec![]
        }
    }

    pub(crate) fn trigger(&self, value: &mut T, info: &U, details: &V, extra_value: &mut W) {
        for l in &self.listeners {
            (l.callback)(value, info, details, extra_value, &l.player);
        }
    }
}

pub struct Event<T, U = (), V = (), W = ()> {
    pub name: String,
    pub inner: Option<EventMut<T, U, V, W>>,
}

impl<T, U, V, W> Event<T, U, V, W> {
    #[must_use]
    pub fn new(name: &str) -> Self
    where
        T: Clone + PartialEq,
        W: Clone + PartialEq,
    {
        Self {
            name: name.to_string(),
            inner: Some(EventMut::new(name)),
        }
    }

    pub(crate) fn get(&self) -> &EventMut<T, U, V, W> {
        self.inner.as_ref().expect("Event should be initialized")
    }

    pub(crate) fn take(&mut self) -> EventMut<T, U, V, W> {
        self.inner.take().expect("Event should be initialized")
    }

    pub(crate) fn set(&mut self, event: EventMut<T, U, V, W>) {
        self.inner = Some(event);
    }
}

#[cfg(test)]
mod tests {
    use super::{EventMut, EventOrigin, EventPlayer};
    use crate::advance::Advance;
    use crate::player::CostTrigger;

    #[test]
    fn mutable_event() {
        let mut event = EventMut::new("test");
        let add_constant = Advance::Arts;
        event.add_listener_mut(
            |item, constant, _, (), _| *item += constant,
            0,
            EventPlayer::new(0, String::new(), EventOrigin::Advance(add_constant)),
        );
        let multiply_value = Advance::Sanitation;
        event.add_listener_mut(
            |item, _, multiplier, (), _| *item *= multiplier,
            -1,
            EventPlayer::new(0, String::new(), EventOrigin::Advance(multiply_value)),
        );
        let no_change = Advance::Bartering;
        event.add_listener_mut(
            |item, _, _, (), _| {
                *item += 1;
                *item -= 1;
            },
            1,
            EventPlayer::new(0, String::new(), EventOrigin::Advance(no_change)),
        );

        let mut item = 0;
        let addend = 2;
        let multiplier = 3;
        let modifiers = event.trigger_with_modifiers(
            &mut item,
            &addend,
            &multiplier,
            &mut (),
            CostTrigger::WithModifiers,
        );
        assert_eq!(6, item);
        assert_eq!(
            vec![
                EventOrigin::Advance(add_constant),
                EventOrigin::Advance(multiply_value)
            ],
            modifiers
        );

        event.remove_listener_mut_by_key(&EventOrigin::Advance(multiply_value));
        let mut item = 0;
        let addend = 3;
        let modifiers = event.trigger_with_modifiers(
            &mut item,
            &addend,
            &0,
            &mut (),
            CostTrigger::WithModifiers,
        );
        assert_eq!(3, item);
        assert_eq!(vec![EventOrigin::Advance(add_constant)], modifiers);
    }
}
