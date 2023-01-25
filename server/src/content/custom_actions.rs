use serde::{Deserialize, Serialize};

use crate::{
    city::{City, CityData},
    game::Game,
    playing_actions::{ActionType, CustomAction},
    resource_pile::ResourcePile,
};

use super::wonders;

pub fn get_custom_action(name: &str, contents: &str) -> Box<dyn CustomAction> {
    Box::new(
        match name {
            "Construct wonder" => serde_json::from_str::<ConstructWonder>(contents),
            _ => panic!("Invalid action name"),
        }
        .unwrap_or_else(|_| panic!("Invalid {} action name", name)),
    )
}

#[derive(Serialize, Deserialize)]
struct ConstructWonder {
    city: CityData,
    wonder: String,
    payment: ResourcePile,
}

impl CustomAction for ConstructWonder {
    fn execute(&self, game: &mut Game, player_index: usize) {
        let city = City::from_data(self.city.clone());
        let wonder = wonders::get_wonder_by_name(&self.wonder)
            .expect("construct wonder data should include a valid wonder name");
        if !city.can_build_wonder(&wonder, &game.players[player_index])
            || !self.payment.can_afford(&wonder.cost)
        {
            panic!("Illegal action");
        }
        game.players[player_index].loose_resources(self.payment.clone());
        game.build_wonder(wonder, &city.position, player_index);
    }

    fn action_type(&self) -> ActionType {
        ActionType::default()
    }

    fn name(&self) -> String {
        String::from("Construct wonder")
    }
}
