use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::barbarians::barbarians_bonus;
use crate::combat::{
    choose_carried_units_casualties, choose_fighter_casualties, offer_retreat, place_settler,
};
use crate::events::EventOrigin;

pub struct Builtin {
    pub name: String,
    pub description: String,
    pub listeners: AbilityListeners,
}

impl Builtin {
    #[must_use]
    pub fn builder(name: &str, description: &str) -> BuiltinBuilder {
        BuiltinBuilder::new(name, description)
    }
}

pub struct BuiltinBuilder {
    name: String,
    descriptions: String,
    builder: AbilityInitializerBuilder,
}

impl BuiltinBuilder {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            descriptions: description.to_string(),
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn build(self) -> Builtin {
        Builtin {
            name: self.name,
            description: self.descriptions,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for BuiltinBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Builtin(self.name.clone())
    }
}

#[must_use]
pub fn get_all() -> Vec<Builtin> {
    vec![
        place_settler(),
        choose_fighter_casualties(),
        choose_carried_units_casualties(),
        offer_retreat(),
        barbarians_bonus(),
    ]
}

///
/// # Panics
/// Panics if builtin does not exist
#[must_use]
pub fn get_builtin(name: &str) -> Builtin {
    get_all()
        .into_iter()
        .find(|builtin| builtin.name == name)
        .expect("builtin not found")
}
