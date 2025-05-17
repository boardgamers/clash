#![allow(dead_code)]

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum EventOrigin {
    Advance(Advance),
    SpecialAdvance(SpecialAdvance),
    Leader(String),
    Wonder(Wonder),
    Builtin(String),
    Incident(u8),
    CivilCard(u8),
    TacticsCard(u8),
    Objective(String),
}

impl EventOrigin {
    #[must_use]
    pub fn id(&self) -> String {
        match self {
            EventOrigin::Wonder(name) => format!("{name:?}"),
            | EventOrigin::Advance(name)
            // can't call to_string, because cache is not constructed
            | EventOrigin::SpecialAdvance(name) => format!("{name:?}"),
            | EventOrigin::Leader(name)
            | EventOrigin::Objective(name)
            | EventOrigin::Builtin(name) => name.to_string(),
            EventOrigin::CivilCard(id)
            | EventOrigin::TacticsCard(id)
            | EventOrigin::Incident(id) => id.to_string(),
        }
    }

    #[must_use]
    pub fn name(&self, game: &Game) -> String {
        let cache = &game.cache;
        match self {
            EventOrigin::Advance(name) | EventOrigin::SpecialAdvance(name) => {
                name.name(game).to_string()
            }
            EventOrigin::Wonder(name) => name.name(game).to_string(),
            EventOrigin::Leader(name)
            | EventOrigin::Objective(name)
            | EventOrigin::Builtin(name) => name.to_string(),
            EventOrigin::CivilCard(id) => cache.get_civil_card(*id).name.clone(),
            EventOrigin::TacticsCard(id) => cache.get_tactics_card(*id).name.clone(),
            EventOrigin::Incident(id) => cache.get_incident(*id).name.clone(),
        }
    }
}

use crate::advance::Advance;
use crate::game::Game;
use crate::player::CostTrigger;
use crate::wonder::Wonder;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::special_advance::SpecialAdvance;

struct Listener<T, U, V, W> {
    #[allow(clippy::type_complexity)]
    callback: Arc<dyn Fn(&mut T, &U, &V, &mut W) + Sync + Send>,
    priority: i32,
    id: usize,
    origin: EventOrigin,
    owning_player: usize,
}

impl<T, U, V, W> Listener<T, U, V, W> {
    fn new<F>(
        callback: F,
        priority: i32,
        id: usize,
        origin: EventOrigin,
        owning_player: usize,
    ) -> Self
    where
        F: Fn(&mut T, &U, &V, &mut W) + 'static + Sync + Send,
    {
        Self {
            callback: Arc::new(callback),
            priority,
            id,
            origin,
            owning_player,
        }
    }
}

pub struct EventMut<T, U = (), V = (), W = ()> {
    name: String, // for debugging
    listeners: Vec<Listener<T, U, V, W>>,
    next_id: usize,
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
            next_id: 0,
        }
    }

    //return the id of the listener witch can be used to remove the listener later
    pub(crate) fn add_listener_mut<F>(
        &mut self,
        new_listener: F,
        priority: i32,
        key: EventOrigin,
        owning_player: usize,
    ) -> usize
    where
        F: Fn(&mut T, &U, &V, &mut W) + 'static + Sync + Send,
    {
        let id = self.next_id;
        // objectives can have the same key, but you still can only fulfill one of them at a time
        if let Some(old) = self
            .listeners
            .iter()
            .find(|l| priority == l.priority && l.origin != key)
        {
            panic!(
                "Event {}: Priority {priority} already used by listener with key {:?} when adding {key:?}",
                self.name, old.origin
            )
        }
        self.listeners.push(Listener::new(
            new_listener,
            priority,
            id,
            key,
            owning_player,
        ));

        self.listeners.sort_by_key(|l| l.priority);
        self.listeners.reverse();
        self.next_id += 1;
        id
    }

    pub(crate) fn remove_listener_mut_by_key(&mut self, key: &EventOrigin) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|l| &l.origin == key)
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
            self.trigger_with_exclude(value, info, details, extra_value, &[])
        } else {
            self.trigger(value, info, details, extra_value);
            vec![]
        }
    }

    pub(crate) fn trigger(&self, value: &mut T, info: &U, details: &V, extra_value: &mut W) {
        for l in &self.listeners {
            (l.callback)(value, info, details, extra_value);
        }
    }

    #[must_use]
    fn trigger_with_exclude(
        &self,
        value: &mut T,
        info: &U,
        details: &V,
        extra_value: &mut W,
        exclude: &[EventOrigin],
    ) -> Vec<EventOrigin> {
        let mut modifiers = Vec::new();
        for l in &self.listeners {
            if exclude.contains(&l.origin) {
                continue;
            }
            let previous_value = value.clone();
            let previous_extra_value = extra_value.clone();
            (l.callback)(value, info, details, extra_value);
            if *value != previous_value || *extra_value != previous_extra_value {
                modifiers.push(l.origin.clone());
            }
        }
        modifiers
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
    use super::{EventMut, EventOrigin};
    use crate::advance::Advance;
    use crate::player::CostTrigger;

    #[test]
    fn mutable_event() {
        let mut event = EventMut::new("test");
        let add_constant = Advance::Arts;
        event.add_listener_mut(
            |item, constant, _, ()| *item += constant,
            0,
            EventOrigin::Advance(add_constant),
            0,
        );
        let multiply_value = Advance::Sanitation;
        event.add_listener_mut(
            |item, _, multiplier, ()| *item *= multiplier,
            -1,
            EventOrigin::Advance(multiply_value),
            0,
        );
        let no_change = Advance::Bartering;
        event.add_listener_mut(
            |item, _, _, ()| {
                *item += 1;
                *item -= 1;
            },
            1,
            EventOrigin::Advance(no_change),
            0,
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
