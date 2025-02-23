#![allow(dead_code)]

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum EventOrigin {
    Advance(String),
    SpecialAdvance(String),
    Leader(String),
    Wonder(String),
    Builtin(String),
    Incident(u8),
}

impl EventOrigin {
    #[must_use]
    pub fn id(&self) -> String {
        match self {
            EventOrigin::Advance(name)
            | EventOrigin::SpecialAdvance(name)
            | EventOrigin::Wonder(name)
            | EventOrigin::Leader(name)
            | EventOrigin::Builtin(name) => name.to_string(),
            EventOrigin::Incident(id) => id.to_string(),
        }
    }

    #[must_use]
    pub fn name(&self) -> String {
        match self {
            EventOrigin::Advance(name)
            | EventOrigin::SpecialAdvance(name)
            | EventOrigin::Wonder(name)
            | EventOrigin::Leader(name)
            | EventOrigin::Builtin(name) => name.to_string(),
            EventOrigin::Incident(id) => incidents::get_incident(*id).name,
        }
    }
}

use crate::content::incidents;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

type Listener<T, U, V> = (Box<dyn Fn(&mut T, &U, &V)>, i32, usize, EventOrigin);

pub struct EventMut<T, U = (), V = ()> {
    listeners: Vec<Listener<T, U, V>>,
    next_id: usize,
}

impl<T, U, V> EventMut<T, U, V>
where
    T: Clone + PartialEq,
{
    //return the id of the listener witch can be used to remove the listener later
    pub(crate) fn add_listener_mut<F>(&mut self, new_listener: F, priority: i32, key: EventOrigin) -> usize
    where
        F: Fn(&mut T, &U, &V) + 'static,
    {
        let id = self.next_id;
        if let Some(_) = self.listeners.iter().find(|(_, p, _, _)| &priority == p) {
            panic!(
                "Priority {priority} already used by listener with key {:?}",
                key
            )
        }
        self.listeners
            .push((Box::new(new_listener), priority, id, key));
        self.listeners.sort_by_key(|(_, priority, _, _)| *priority);
        self.listeners.reverse();
        self.next_id += 1;
        id
    }

    pub(crate) fn remove_listener_mut_by_key(&mut self, key: &EventOrigin) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|(_, _, _, value)| value == key)
                .expect("Listeners should include the key to remove"),
        );
    }

    #[must_use]
    pub(crate) fn trigger(&self, value: &mut T, info: &U, details: &V) -> Vec<EventOrigin> {
        self.trigger_with_exclude(value, info, details, &[])
    }

    #[must_use]
    fn trigger_with_exclude(
        &self,
        value: &mut T,
        info: &U,
        details: &V,
        exclude: &[EventOrigin],
    ) -> Vec<EventOrigin> {
        let mut modifiers = Vec::new();
        for (listener, _, _, key) in &self.listeners {
            if exclude.contains(key) {
                continue;
            }
            let previous_value = value.clone();
            listener(value, info, details);
            if *value != previous_value {
                modifiers.push(key.clone());
            }
        }
        modifiers
    }

    pub(crate) fn trigger_with_minimal_modifiers(
        &self,
        value: &T,
        info: &U,
        details: &V,
        is_ok: impl Fn(&T) -> bool,
        set_modifiers: impl Fn(&mut T, Vec<EventOrigin>),
    ) -> T
    where
        T: Clone + PartialEq,
    {
        let mut initial_value = value.clone();
        let initial_modifiers = self.trigger(&mut initial_value, info, details);

        initial_modifiers
            .iter()
            .powerset()
            .find_map(|try_modifiers| {
                let mut v = value.clone();
                let mut exclude = initial_modifiers.clone();
                exclude.retain(|origin| !try_modifiers.contains(&origin));
                let m = self.trigger_with_exclude(&mut v, info, details, &exclude);
                if is_ok(&v) {
                    set_modifiers(&mut v, m);
                    Some(v)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                set_modifiers(&mut initial_value, initial_modifiers);
                initial_value
            })
    }
}

impl<T, U, V> Default for EventMut<T, U, V> {
    fn default() -> Self {
        Self {
            listeners: Vec::new(),
            next_id: 0,
        }
    }
}

pub struct Event<T, U = (), V = ()> {
    pub inner: Option<EventMut<T, U, V>>,
    pub name: String,
}

impl<T, U, V> Event<T, U, V> {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            inner: Some(EventMut::default()),
            name: name.to_string(),
        }
    }

    pub(crate) fn get(&self) -> &EventMut<T, U, V> {
        self.inner.as_ref().expect("Event should be initialized")
    }

    pub(crate) fn take(&mut self) -> EventMut<T, U, V> {
        self.inner.take().expect("Event should be initialized")
    }

    pub(crate) fn set(&mut self, event: EventMut<T, U, V>) {
        self.inner = Some(event);
    }
}

#[cfg(test)]
mod tests {
    use super::{EventMut, EventOrigin};

    #[test]
    fn mutable_event() {
        let mut event = EventMut::default();
        event.add_listener_mut(
            |item, constant, _| *item += constant,
            0,
            EventOrigin::Advance("add constant".to_string()),
        );
        event.add_listener_mut(
            |item, _, multiplier| *item *= multiplier,
            -1,
            EventOrigin::Advance("multiply value".to_string()),
        );
        event.add_listener_mut(
            |item, _, _| {
                *item += 1;
                *item -= 1;
            },
            1,
            EventOrigin::Advance("no change".to_string()),
        );

        let mut item = 0;
        let addend = 2;
        let multiplier = 3;
        let modifiers = event.trigger(&mut item, &addend, &multiplier);
        assert_eq!(6, item);
        assert_eq!(
            vec![
                EventOrigin::Advance("add constant".to_string()),
                EventOrigin::Advance("multiply value".to_string())
            ],
            modifiers
        );

        event.remove_listener_mut_by_key(&EventOrigin::Advance("multiply value".to_string()));
        let mut item = 0;
        let addend = 3;
        let modifiers = event.trigger(&mut item, &addend, &0);
        assert_eq!(3, item);
        assert_eq!(
            vec![EventOrigin::Advance("add constant".to_string())],
            modifiers
        );
    }

    #[test]
    fn find_minimal_modifiers() {
        #[derive(Clone, PartialEq)]
        struct Info {
            pub value: i32,
            pub modifiers: Vec<EventOrigin>,
        }

        let mut event = EventMut::default();
        event.add_listener_mut(
            |value: &mut Info, (), ()| value.value += 1,
            0,
            EventOrigin::Advance("A".to_string()),
        );
        event.add_listener_mut(
            |value: &mut Info, (), ()| value.value += 2,
            1,
            EventOrigin::Advance("B".to_string()),
        );
        event.add_listener_mut(
            |value: &mut Info, (), ()| value.value += 4,
            2,
            EventOrigin::Advance("C".to_string()),
        );

        assert_eq!(
            vec![
                EventOrigin::Advance("C".to_string()),
                EventOrigin::Advance("A".to_string())
            ],
            event
                .trigger_with_minimal_modifiers(
                    &Info {
                        value: 0,
                        modifiers: Vec::new(),
                    },
                    &(),
                    &(),
                    |i| i.value == 5,
                    |v, m| v.modifiers = m
                )
                .modifiers
        );
    }
}
