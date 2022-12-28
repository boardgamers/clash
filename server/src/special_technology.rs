use crate::{player_setup::PlayerSetup, Player};

pub struct SpecialTechnology {
    pub name: String,
    pub required_technology: usize,
    pub initializer: PlayerSetup,
}

impl SpecialTechnology {
    pub fn new<F>(name: &str, required_technology: usize, initializer: F) -> Self
    where
        F: Fn(&mut Player) + 'static,
    {
        Self {
            name: name.to_string(),
            required_technology,
            initializer: Box::new(initializer),
        }
    }
}
