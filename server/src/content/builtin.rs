use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::content::custom_phase_actions::CustomPhasePositionRequest;
use crate::events::EventOrigin;
use crate::unit::UnitType;
use crate::{ability_initializer::AbilityInitializerSetup, position::Position};

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
    vec![Builtin::builder(
        "Place Settler",
        "After losing a city, place a settler in another city.",
    )
    .add_position_reward_request_listener(
        |event| &mut event.on_combat_end,
        0,
        |game, player_index, i| {
            if i.is_defender(player_index)
                && i.is_loser(player_index)
                && game.get_any_city(i.defender_position).is_some()
                && !game.get_player(player_index).cities.is_empty()
                && game.get_player(player_index).available_units().settlers > 0
            {
                let p = game.get_player(player_index);
                let choices: Vec<Position> = p.cities.iter().map(|c| c.position).collect();
                Some(CustomPhasePositionRequest::new(choices, None))
            } else {
                None
            }
        },
        |c, _game, pos| {
            c.add_info_log_item(&format!(
                "{} gained 1 free Settler Unit at {pos} for losing a city",
                c.name,
            ));
            c.gain_unit(c.index, UnitType::Settler, *pos);
        },
    )
    .build()]
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
