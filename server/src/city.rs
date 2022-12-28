use crate::building::Building;

pub struct City {
    pub buildings: Vec<Building>,
    pub mood_state: MoodState,
    pub is_activated: bool,
}

pub enum MoodState {
    Angry,
    Neutral,
    Happy,
}
