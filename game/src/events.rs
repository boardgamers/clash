type Listener<T> = (Box<dyn Fn(&T)>, i32);

#[derive(Default)]
pub struct Event<T> {
    listeners: Vec<Listener<T>>,
}

impl<T> Event<T> {
    //return the index of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn(&T) + 'static,
    {
        self.listeners.push((Box::new(new_listener), priority));
        self.listeners.sort_by_key(|(_, priority)| *priority);
        self.listeners.reverse();
        self.listeners.len() - 1
    }

    pub fn remove_listener(&mut self, index: usize) {
        self.listeners.remove(index);
    }

    pub fn trigger(&self, value: T) {
        for (listener, _) in self.listeners.iter() {
            listener(&value);
        }
    }
}

type ListenerMut<T> = (Box<dyn Fn(&mut T)>, i32);

#[derive(Default)]
pub struct EventMut<T> {
    listeners: Vec<Listener<T>>,
    listeners_mut: Vec<ListenerMut<T>>,
}

impl<T> EventMut<T> {
    //return the index of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn(&T) + 'static,
    {
        self.listeners.push((Box::new(new_listener), priority));
        self.listeners.sort_by_key(|(_, priority)| *priority);
        self.listeners.reverse();
        self.listeners.len() - 1
    }

    pub fn remove_listener(&mut self, index: usize) {
        self.listeners.remove(index);
    }

    //return the index of the listener witch can be used to remove the listener later
    pub fn add_listener_mut<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn(&mut T) + 'static,
    {
        self.listeners_mut.push((Box::new(new_listener), priority));
        self.listeners_mut.sort_by_key(|(_, priority)| *priority);
        self.listeners_mut.reverse();
        self.listeners_mut.len() - 1
    }

    pub fn remove_listener_mut(&mut self, index: usize) {
        self.listeners_mut.remove(index);
    }

    pub fn trigger(&mut self, value: &mut T) {
        for (listener, _) in self.listeners_mut.iter_mut() {
            listener(value);
        }
        for (listener, _) in self.listeners.iter() {
            listener(value);
        }
    }
}

type StaticListener = (Box<dyn Fn() + 'static>, i32);

#[derive(Default)]
pub struct StaticEvent {
    listeners: Vec<StaticListener>,
}

impl StaticEvent {
    //return the index of the listener witch can be used to remove the listener later
    pub fn add_listener<F>(&mut self, new_listener: F, priority: i32) -> usize
    where
        F: Fn() + 'static,
    {
        self.listeners.push((Box::new(new_listener), priority));
        self.listeners.sort_by_key(|(_, priority)| *priority);
        self.listeners.reverse();
        self.listeners.len() - 1
    }

    pub fn remove_listener(&mut self, index: usize) {
        self.listeners.remove(index);
    }

    pub fn trigger(&self) {
        for (listener, _) in self.listeners.iter() {
            listener();
        }
    }
}
