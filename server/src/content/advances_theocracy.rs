use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, AdvanceBuilder};
use crate::city_pieces::Building::Temple;
use crate::content::advances::{advance_group_builder, get_group, AdvanceGroup};
use crate::content::custom_phase_actions::CustomPhaseAdvanceRewardRequest;
use crate::resource_pile::ResourcePile;

pub(crate) fn theocracy() -> AdvanceGroup {
    advance_group_builder(
        "Theocracy",
        vec![
            dogma(),
            Advance::builder("Dedication", "todo"),
            Advance::builder("Conversion", "todo"),
            Advance::builder("Fanaticism", "todo"),
        ],
    )
}

fn dogma() -> AdvanceBuilder {
    Advance::builder("Dogma", "Whenever you Construct a new Temple, either through the Construct Action or through playing of cards, you may immediately get a Theocracy Advance for free, marking it with a cube from your Event tracker as normal.
    You are now limited to a maximum of 2 ideas. If you have more than 2 ideas when
    getting this Advance, you must immediately reduce down to 2.
    Note: Dogma Advance does not apply when you conquer a city with a Temple.")
        .add_one_time_ability_initializer(|game, player_index| {
            let p = & mut game.players[player_index];
            p.resource_limit.ideas = 2;
            p.gain_resources(ResourcePile::ideas(0)); // to trigger the limit
          })
          .add_ability_undo_deinitializer(|game, player_index| {
              game.players[player_index].resource_limit.ideas = 7;
          })
        .add_advance_reward_request_listener(
            |event| &mut event.on_construct,
            0,
            |game, player_index, building| {
                if matches!(building, Temple) {
                    let player = game.get_player(player_index);
                    let choices: Vec<String> = get_group("Theocracy").advances
                        .iter()
                        .filter(|a| player.can_advance_free(a))
                        .map(|a| a.name.clone())
                        .collect();
                    if choices.is_empty() {
                        return None;
                    }
                    return Some(CustomPhaseAdvanceRewardRequest {
                        choices,
                    });
                }
                None
            },
            |game, player_index, player_name, name, selected| {
                let verb = if selected {
                    "selected"
                } else {
                    "got"
                };
                game.add_info_log_item(format!(
                    "{player_name} {verb} {name} as a reward for constructing a Temple",
                ));
                game.advance(name, player_index, ResourcePile::empty());
            },
        )
}
