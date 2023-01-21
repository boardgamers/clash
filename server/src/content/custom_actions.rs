use serde::{Serialize, Deserialize};

use crate::{playing_actions::{CustomAction, ActionType}, city::{CityData, City}, resource_pile::ResourcePile, game::Game};

use super::wonders;

pub fn get_custom_action(name: &str, contents: &str) -> Box<dyn CustomAction> {
    Box::new(match name {
        "Construct wonder" => serde_json::from_str::<ConstructWonder>(contents),
        _ => panic!("Invalid action name"),
    }.unwrap_or_else(|_| panic!("Invalid {} action name", name)))
}

#[derive(Serialize, Deserialize)]
struct ConstructWonder {
    city: CityData,
    wonder: String,
    payment: ResourcePile,
}

impl CustomAction for ConstructWonder {
    fn execute(&self, game: &mut Game, player: usize) {
        let city = City::from_data(self.city.clone());
        let wonder = wonders::get_wonder_by_name(&self.wonder).expect("construct wonder data should include a valid wonder name");
        if !city.can_build_wonder(&wonder, &game.players[player]) || !self.payment.can_afford(&wonder.cost) {
            panic!("Illegal action");
        }
        game.with_player(player, |player, game| {
            player.loose_resources(self.payment.clone());
            player.with_city(&city.position, |player, city| {
                city.build_wonder(wonder, game, player.id);
            });
        });
    }

    fn action_type(&self) -> ActionType {
        ActionType::default()
    }

    fn name(&self) -> String {
        String::from("Construct wonder")
    }
}
