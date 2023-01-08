type Listener<T> = (Box<dyn Fn(&T)>, i32, usize);

pub struct Event<T> {
    listeners: Vec<Listener<T>>,
}

impl<T> Event<T> {
    //return the index of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn(&T) + 'static,
    {
        let id = self.listeners.len();
        self.listeners.push((Box::new(new_listener), priority, id));
        self.listeners.sort_by_key(|(_, priority, _)| *priority);
        self.listeners.reverse();
        id
    }

    pub fn remove_listener(&mut self, id: usize) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|(_, _, value)| value == &id)
                .expect("Listeners should include this id"),
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
        }
    }
}

type ListenerMut<T, U, V> = (Box<dyn Fn(&mut T, &U, &V)>, i32, usize);
type ListenerImmutable<T, U, V> = (Box<dyn Fn(&T, &U, &V)>, i32, usize);

pub struct EventMut<T, U = (), V = ()> {
    listeners: Vec<ListenerImmutable<T, U, V>>,
    listeners_mut: Vec<ListenerMut<T, U, V>>,
}

impl<T, U, V> EventMut<T, U, V> {
    //return the index of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn(&T, &U, &V) + 'static,
    {
        let id = self.listeners.len();
        self.listeners.push((Box::new(new_listener), priority, id));
        self.listeners.sort_by_key(|(_, priority, _)| *priority);
        self.listeners.reverse();
        id
    }

    pub fn remove_listener(&mut self, id: usize) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|(_, _, value)| value == &id)
                .expect("Listeners should include this id"),
        );
    }

    //return the index of the listener witch can be used to remove the listener later
    pub fn add_listener_mut<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn(&mut T, &U, &V) + 'static,
    {
        let id = self.listeners_mut.len();
        self.listeners_mut
            .push((Box::new(new_listener), priority, id));
        self.listeners_mut.sort_by_key(|(_, priority, _)| *priority);
        self.listeners_mut.reverse();
        id
    }

    pub fn remove_listener_mut(&mut self, id: usize) {
        let _ = self.listeners_mut.remove(
            self.listeners_mut
                .iter()
                .position(|(_, _, value)| value == &id)
                .expect("Listeners should include this id"),
        );
    }

    pub fn trigger(&self, value: &mut T, info: &U, details: &V) {
        for (listener, _, _) in self.listeners_mut.iter() {
            listener(value, info, details);
        }
        for (listener, _, _) in self.listeners.iter() {
            listener(value, info, details);
        }
    }
}

impl<T, U, V> Default for EventMut<T, U, V> {
    fn default() -> Self {
        Self {
            listeners: Vec::new(),
            listeners_mut: Vec::new(),
        }
    }
}

type StaticListener = (Box<dyn Fn() + 'static>, i32, usize);

pub struct StaticEvent {
    listeners: Vec<StaticListener>,
}

impl StaticEvent {
    //return the index of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn() + 'static,
    {
        let id = self.listeners.len();
        self.listeners.push((Box::new(new_listener), priority, id));
        self.listeners.sort_by_key(|(_, priority, _)| *priority);
        self.listeners.reverse();
        id
    }

    pub fn remove_listener(&mut self, id: usize) {
        let _ = self.listeners.remove(
            self.listeners
                .iter()
                .position(|(_, _, value)| value == &id)
                .expect("Listeners should include this id"),
        );
    }

    pub fn trigger(&self) {
        for (listener, _, _) in self.listeners.iter() {
            listener();
        }
    }
}
