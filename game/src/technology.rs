use crate::{player_setup::PlayerSetup, Player};

struct Technology {
    name: String,
    required_technology: Option<usize>,
    contradicting_technology: Option<usize>,
    initializer: PlayerSetup,
}

impl Technology {
    fn new<F>(
        name: &str,
        required_technology: Option<usize>,
        contradicting_technology: Option<usize>,
        initializer: F,
    ) -> Self
    where
        F: Fn(&mut Player) + 'static,
    {
        Self {
            name: name.to_string(),
            required_technology,
            contradicting_technology,
            initializer: Box::new(initializer),
        }
    }
}
