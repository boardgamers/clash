#![allow(dead_code)]

type Listener<T, U, V> = (Box<dyn Fn(&mut T, &U, &V)>, i32, usize, String);

pub struct EventMut<T, U = (), V = ()> {
    listeners: Vec<Listener<T, U, V>>,
    next_id: usize,
}

impl<T, U, V> EventMut<T, U, V>
where
    T: Clone + PartialEq,
{
    //return the id of the listener witch can be used to remove the listener later
    pub fn add_listener_mut<F>(&mut self, new_listener: F, priority: i32, key: String) -> usize
    where
        F: Fn(&mut T, &U, &V) + 'static,
    {
        let id = self.next_id;
        self.listeners
            .push((Box::new(new_listener), priority, id, key));
        self.listeners.sort_by_key(|(_, priority, _, _)| *priority);
        self.listeners.reverse();
        self.next_id += 1;
        id
    }

    pub fn remove_listener_mut_by_key(&mut self, key: &str) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|(_, _, _, value)| value == key)
                .expect("Listeners should include the key to remove"),
        );
    }

    pub fn trigger(&self, value: &mut T, info: &U, details: &V) -> Vec<String> {
        let mut modifiers = Vec::new();
        for (listener, _, _, key) in &self.listeners {
            let previous_value = value.clone();
            listener(value, info, details);
            if *value != previous_value {
                modifiers.push(key.clone());
            }
        }
        modifiers
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
}

impl<T, U, V> Event<T, U, V> {
    pub fn get(&self) -> &EventMut<T, U, V> {
        self.inner.as_ref().expect("Event should be initialized")
    }

    pub fn take(&mut self) -> EventMut<T, U, V> {
        self.inner.take().expect("Event should be initialized")
    }

    pub fn set(&mut self, event: EventMut<T, U, V>) {
        self.inner = Some(event);
    }
}

impl<T, U, V> Default for Event<T, U, V> {
    fn default() -> Self {
        Self {
            inner: Some(EventMut::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EventMut;

    #[test]
    fn mutable_event() {
        let mut event = EventMut::default();
        event.add_listener_mut(
            |item, constant, _| *item += constant,
            0,
            String::from("add constant"),
        );
        event.add_listener_mut(
            |item, _, multiplier| *item *= multiplier,
            -1,
            String::from("multiply value"),
        );
        event.add_listener_mut(
            |item, _, _| {
                *item += 1;
                *item -= 1;
            },
            0,
            String::from("no change"),
        );

        let mut item = 0;
        let addend = 2;
        let multiplier = 3;
        let modifiers = event.trigger(&mut item, &addend, &multiplier);
        assert_eq!(6, item);
        assert_eq!(
            vec![String::from("add constant"), String::from("multiply value")],
            modifiers
        );

        event.remove_listener_mut_by_key("multiply value");
        let mut item = 0;
        let addend = 3;
        let modifiers = event.trigger(&mut item, &addend, &0);
        assert_eq!(3, item);
        assert_eq!(vec![String::from("add constant")], modifiers);
    }
}
