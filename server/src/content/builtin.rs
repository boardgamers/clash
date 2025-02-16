use crate::content::custom_phase_actions::CustomPhasePositionRequest;
use crate::events::EventOrigin;
use crate::unit::UnitType;
use crate::{
    ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup},
    game::Game,
    position::Position,
};

pub struct Builtin {
    pub name: String,
    pub description: String,
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
    pub player_one_time_initializer: AbilityInitializer,
    pub player_undo_deinitializer: AbilityInitializer,
}

impl Builtin {
    #[must_use]
    pub fn builder(name: &str, description: &str) -> BuiltinBuilder {
        BuiltinBuilder::new(name, description)
    }
}

pub struct BuiltinBuilder {
    name: String,
    descriptions: Vec<String>,
    player_initializers: Vec<AbilityInitializer>,
    player_deinitializers: Vec<AbilityInitializer>,
    player_one_time_initializers: Vec<AbilityInitializer>,
    player_undo_deinitializers: Vec<AbilityInitializer>,
}

impl BuiltinBuilder {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            descriptions: vec![description.to_string()],
            player_initializers: Vec::new(),
            player_deinitializers: Vec::new(),
            player_one_time_initializers: Vec::new(),
            player_undo_deinitializers: Vec::new(),
        }
    }

    #[must_use]
    pub fn build(self) -> Builtin {
        let player_initializer =
            ability_initializer::join_ability_initializers(self.player_initializers);
        let player_deinitializer =
            ability_initializer::join_ability_initializers(self.player_deinitializers);
        let player_one_time_initializer =
            ability_initializer::join_ability_initializers(self.player_one_time_initializers);
        let player_undo_deinitializer =
            ability_initializer::join_ability_initializers(self.player_undo_deinitializers);
        Builtin {
            name: self.name,
            description: String::from("✦ ") + &self.descriptions.join("\n✦ "),
            player_initializer,
            player_deinitializer,
            player_one_time_initializer,
            player_undo_deinitializer,
        }
    }
}

impl AbilityInitializerSetup for BuiltinBuilder {
    fn add_ability_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_initializers.push(Box::new(initializer));
        self
    }

    fn add_ability_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_deinitializers.push(Box::new(deinitializer));
        self
    }

    fn add_one_time_ability_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_one_time_initializers
            .push(Box::new(initializer));
        self
    }

    fn add_ability_undo_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_undo_deinitializers
            .push(Box::new(deinitializer));
        self
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
                Some(CustomPhasePositionRequest { choices })
            } else {
                None
            }
        },
        |c, _game, pos| {
            c.add_info_log_item(&format!(
                "{} gained 1 free Settler Unit at {pos} for losing a city",
                c.name,
            ));
            c.gain_unit(UnitType::Settler, *pos);
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
