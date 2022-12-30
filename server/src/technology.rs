use crate::player::{self, PlayerSetup, PlayerInitializer};

pub struct Technology {
    pub name: String,
    pub required_technology: Option<usize>,
    pub contradicting_technology: Option<usize>,
    pub initializer: PlayerInitializer,
    pub deinitializer: PlayerInitializer,
}

impl Technology {
    pub fn builder(name: &str) -> TechnologyBuilder {
        TechnologyBuilder::new(name.to_string())
    }

    fn new(
        name: String,
        required_technology: Option<usize>,
        contradicting_technology: Option<usize>,
        initializer: PlayerInitializer,
        deinitializer: PlayerInitializer,
    ) -> Self {
        Self {
            name,
            required_technology,
            contradicting_technology,
            initializer,
            deinitializer,
        }
    }
}

pub struct TechnologyBuilder {
    name: String,
    required_technology: Option<usize>,
    contradicting_technology: Option<usize>,
    initializers: Vec<PlayerInitializer>,
    deinitializers: Vec<PlayerInitializer>,
}

impl TechnologyBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            required_technology: None,
            contradicting_technology: None,
            initializers: Vec::new(),
            deinitializers: Vec::new(),
        }
    }

    pub fn with_required_technology(mut self, required_technology: usize) -> Self {
        self.required_technology = Some(required_technology);
        self
    }

    pub fn with_contradicting_technology(mut self, contradicting_technology: usize) -> Self {
        self.contradicting_technology = Some(contradicting_technology);
        self
    }

    pub fn build(self) -> Technology {
        let initializer = player::join_player_initializers(self.initializers);
        let deinitializer = player::join_player_initializers(self.deinitializers);
        Technology::new(
            self.name,
            self.required_technology,
            self.contradicting_technology,
            initializer,
            deinitializer,
        )
    }
}

impl PlayerSetup for TechnologyBuilder {
    fn add_player_initializer(mut self, initializer: PlayerInitializer) -> Self {
        self.initializers.push(initializer);
        self
    }

    fn add_player_deinitializer(mut self, deinitializer: PlayerInitializer) -> Self {
        self.deinitializers.push(deinitializer);
        self
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
