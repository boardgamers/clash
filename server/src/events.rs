#![allow(dead_code)]

type Listener<T> = (Box<dyn Fn(&T)>, i32, usize);

pub struct Event<T> {
    listeners: Vec<Listener<T>>,
    next_id: usize,
}

impl<T> Event<T> {
    //return the id of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn(&T) + 'static,
    {
        let id = self.next_id;
        self.listeners.push((Box::new(new_listener), priority, id));
        self.listeners.sort_by_key(|(_, priority, _)| *priority);
        self.listeners.reverse();
        self.next_id += 1;
        id
    }

    pub fn remove_listener(&mut self, id: usize) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|(_, _, value)| value == &id)
                .expect("Listeners should include the id to remove"),
        );
    }

    pub fn trigger(&self, value: T) {
        for (listener, _, _) in self.listeners.iter() {
            listener(&value);
        }
    }
}

impl<T> Default for Event<T> {
    fn default() -> Self {
        Self {
            listeners: Vec::new(),
            next_id: 0,
        }
    }
}

type ListenerImmutable<T, U, V> = (Box<dyn Fn(&T, &U, &V)>, i32, usize);
type ListenerMutable<T, U, V> = (Box<dyn Fn(&mut T, &U, &V)>, i32, usize, String);

pub struct EventMut<T, U = (), V = ()> {
    listeners: Vec<ListenerImmutable<T, U, V>>,
    listeners_mut: Vec<ListenerMutable<T, U, V>>,
    next_id: usize,
}

impl<T, U, V> EventMut<T, U, V>
where
    T: Clone + PartialEq,
{
    //return the id of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn(&T, &U, &V) + 'static,
    {
        let id = self.next_id;
        self.listeners.push((Box::new(new_listener), priority, id));
        self.listeners.sort_by_key(|(_, priority, _)| *priority);
        self.listeners.reverse();
        self.next_id += 1;
        id
    }

    pub fn remove_listener(&mut self, id: usize) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|(_, _, value)| value == &id)
                .expect("Listeners should include the id to remove"),
        );
    }

    //return the id of the listener witch can be used to remove the listener later
    pub fn add_listener_mut<F>(&mut self, new_listener: F, priority: i32, key: String) -> usize
    where
        F: Fn(&mut T, &U, &V) + 'static,
    {
        let id = self.next_id;
        self.listeners_mut
            .push((Box::new(new_listener), priority, id, key));
        self.listeners_mut
            .sort_by_key(|(_, priority, _, _)| *priority);
        self.listeners_mut.reverse();
        self.next_id += 1;
        id
    }

    pub fn remove_listener_mut(&mut self, id: usize) {
        let _ = self.listeners_mut.remove(
            self.listeners_mut
                .iter()
                .position(|(_, _, value, _)| value == &id)
                .expect("Listeners should include the id to remove"),
        );
    }

    pub fn remove_listener_mut_by_key(&mut self, key: &str) {
        let _ = self.listeners_mut.remove(
            self.listeners_mut
                .iter()
                .position(|(_, _, _, value)| value == key)
                .expect("Listeners should include the key to remove"),
        );
    }

    pub fn trigger(&self, value: &mut T, info: &U, details: &V) -> Vec<String> {
        let mut modifiers = Vec::new();
        for (listener, _, _, key) in self.listeners_mut.iter() {
            let previous_value = value.clone();
            listener(value, info, details);
            if *value != previous_value {
                modifiers.push(key.clone())
            }
        }
        for (listener, _, _) in self.listeners.iter() {
            listener(value, info, details);
        }
        modifiers
    }
}

impl<T, U, V> Default for EventMut<T, U, V> {
    fn default() -> Self {
        Self {
            listeners: Vec::new(),
            listeners_mut: Vec::new(),
            next_id: 0,
        }
    }
}

type StaticListener = (Box<dyn Fn() + 'static>, i32, usize);

#[derive(Default)]
pub struct StaticEvent {
    listeners: Vec<StaticListener>,
    next_id: usize,
}

impl StaticEvent {
    //return the id of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn() + 'static,
    {
        let id = self.next_id;
        self.listeners.push((Box::new(new_listener), priority, id));
        self.listeners.sort_by_key(|(_, priority, _)| *priority);
        self.listeners.reverse();
        self.next_id += 1;
        id
    }

    pub fn remove_listener(&mut self, id: usize) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|(_, _, value)| value == &id)
                .expect("Listeners should include the id to remove"),
        );
    }

    pub fn trigger(&self) {
        for (listener, _, _) in self.listeners.iter() {
            listener();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventMut, StaticEvent};

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
                *item -= 1
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

    #[test]
    #[should_panic]
    fn static_event() {
        let mut event = StaticEvent::default();
        event.add_listener(|| panic!(), 0);
        event.trigger()
    }
}
